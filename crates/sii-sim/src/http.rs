//! Local TCP HTTP server simulating Chilean SII services over loopback (RF-02, RF-04, QA-01).

use crate::error::{Result, SimError};
use crate::simulator::SiiSimulator;
use sii::transport::{Method, Request, Response};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Callback observer invoked for each HTTP request processed by the server.
pub type Observer = dyn Fn(&Request, &Response) + Send + Sync;

/// Local HTTP server hosting the SII simulator on loopback (`127.0.0.1`).
///
/// In accordance with RF-04 and QA-01, binding to any external or non-loopback
/// address is strictly forbidden.
pub struct HttpServer {
    listener: TcpListener,
}

impl HttpServer {
    /// Binds the HTTP server to a local loopback address (e.g. `127.0.0.1:8080` or `127.0.0.1:0`).
    ///
    /// # Errors
    ///
    /// Returns [`SimError::NonLoopbackAddress`] if any resolved address is not loopback (QA-01).
    /// Returns [`SimError::Io`] if socket binding fails.
    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self> {
        let addrs: Vec<SocketAddr> = addr.to_socket_addrs()?.collect();
        if addrs.is_empty() {
            return Err(SimError::InvalidRequest(
                "no addresses resolved".to_string(),
            ));
        }

        for a in &addrs {
            if !a.ip().is_loopback() {
                return Err(SimError::NonLoopbackAddress(a.to_string()));
            }
        }

        let listener = TcpListener::bind(addrs.as_slice())?;
        listener.set_nonblocking(false)?;

        Ok(Self { listener })
    }

    /// Returns the local socket address bound by the server.
    ///
    /// # Errors
    ///
    /// Returns [`SimError::Io`] if socket address cannot be retrieved.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.listener.local_addr().map_err(SimError::from)
    }

    /// Spawns the HTTP server in a background worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`SimError::Io`] if thread creation or socket operations fail.
    pub fn spawn(self, simulator: Arc<SiiSimulator>) -> Result<HttpServerHandle> {
        let local_addr = self.local_addr()?;
        let running = Arc::new(AtomicBool::new(true));

        let running_clone = Arc::clone(&running);
        let listener = self.listener;

        let thread_handle = thread::spawn(move || {
            // Set short timeout so the loop checks running state periodically
            let _ = listener.set_nonblocking(false);
            while running_clone.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                        handle_connection(stream, &simulator, None);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        if !running_clone.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                }
            }
        });

        Ok(HttpServerHandle {
            local_addr,
            running,
            thread: Some(thread_handle),
        })
    }

    /// Runs the HTTP server synchronously in a blocking loop (used by the `sii-sim` CLI binary).
    ///
    /// # Errors
    ///
    /// Returns [`SimError`] if accept fails critically.
    pub fn run_blocking(self, simulator: &SiiSimulator, observer: Option<&Observer>) -> Result<()> {
        for incoming in self.listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                    handle_connection(stream, simulator, observer);
                }
                Err(e) => {
                    return Err(SimError::from(e));
                }
            }
        }
        Ok(())
    }
}

/// Handle to a running background HTTP server. Stops server when dropped.
pub struct HttpServerHandle {
    local_addr: SocketAddr,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HttpServerHandle {
    /// Returns the local socket address of the running server.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Formats a complete HTTP URL pointing to this server for the given path.
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        let clean_path = if path.starts_with('/') {
            path
        } else {
            &format!("/{path}")
        };
        format!("http://{}{clean_path}", self.local_addr)
    }

    /// Signals the server thread to stop.
    pub fn stop(&mut self) {
        if self.running.swap(false, Ordering::SeqCst) {
            // Send dummy connection to unblock accept() if needed
            let _ = TcpStream::connect(self.local_addr);
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for HttpServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_connection(mut stream: TcpStream, simulator: &SiiSimulator, observer: Option<&Observer>) {
    if let Ok(req) = parse_http_request(&mut stream) {
        let resp = match simulator.handle_request(&req) {
            Ok(r) => r,
            Err(e) => Response::new(500, format!("sii-sim error: {e}").into_bytes())
                .with_header("Content-Type", "text/plain"),
        };

        if let Some(obs) = observer {
            obs(&req, &resp);
        }

        let _ = write_http_response(&mut stream, &resp);
    }
}

fn parse_http_request(stream: &mut TcpStream) -> Result<Request> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line)? == 0 {
        return Err(SimError::InvalidRequest("empty request".to_string()));
    }

    let trimmed_line = request_line.trim_end();
    let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(SimError::InvalidRequest(format!(
            "malformed request line: {trimmed_line}"
        )));
    }

    let method_str = parts.first().copied().unwrap_or_default();
    let path = parts.get(1).copied().unwrap_or_default().to_string();

    let method = match method_str {
        "GET" => Method::Get,
        "POST" => Method::Post,
        other => {
            return Err(SimError::InvalidRequest(format!(
                "unsupported method: {other}"
            )));
        }
    };

    let mut headers = Vec::new();
    let mut content_length: usize = 0;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            // End of headers
            break;
        }

        if let Some((name, val)) = trimmed.split_once(':') {
            let n = name.trim().to_string();
            let v = val.trim().to_string();
            if n.eq_ignore_ascii_case("content-length") {
                content_length = v.parse().unwrap_or(0);
            }
            headers.push((n, v));
        }
    }

    let mut body = Vec::new();
    if content_length > 0 {
        // Prevent huge payloads in simulator (max 20 MB)
        let safe_len = content_length.min(20 * 1024 * 1024);
        body.resize(safe_len, 0);
        reader.read_exact(&mut body)?;
    }

    Ok(Request {
        method,
        url: path,
        headers,
        body,
    })
}

fn write_http_response(stream: &mut TcpStream, resp: &Response) -> Result<()> {
    let status_text = match resp.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    };

    write!(
        stream,
        "HTTP/1.1 {status} {status_text}\r\nContent-Length: {len}\r\nConnection: close\r\n",
        status = resp.status,
        len = resp.body.len()
    )?;

    for (name, val) in &resp.headers {
        write!(stream, "{name}: {val}\r\n")?;
    }

    write!(stream, "\r\n")?;
    stream.write_all(&resp.body)?;
    stream.flush()?;
    Ok(())
}
