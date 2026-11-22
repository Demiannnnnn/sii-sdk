//! Sans-IO transport abstraction, in-memory testing transport, and HTTP client.
//!
//! Implements network communication layer behind the [`Transport`] trait (ADR-01, RF-01..03).
//! The core library remains completely sans-IO. Deterministic testing is facilitated
//! by [`MemoryTransport`]. Real network communication uses [`HttpTransport`] (feature `http`).

use crate::error::{Error, Result};
use std::fmt;
use std::sync::{Arc, Mutex};

/// HTTP request method supported by the Chilean SII communication interfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// HTTP GET method (e.g. seed retrieval).
    Get,
    /// HTTP POST method (e.g. token request, envelope upload, status query).
    Post,
}

impl Method {
    /// Returns the static string representation of the HTTP method.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Generic HTTP request representation for sans-IO transport execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// HTTP method (GET or POST).
    pub method: Method,
    /// Target destination URL.
    pub url: String,
    /// Collection of HTTP headers (name, value).
    pub headers: Vec<(String, String)>,
    /// Request payload body bytes.
    pub body: Vec<u8>,
}

impl Request {
    /// Creates a new `Request` with the given method and URL.
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Creates a new HTTP GET request targeting `url`.
    pub fn get(url: impl Into<String>) -> Self {
        Self::new(Method::Get, url)
    }

    /// Creates a new HTTP POST request targeting `url` with payload `body`.
    pub fn post(url: impl Into<String>, body: Vec<u8>) -> Self {
        Self {
            method: Method::Post,
            url: url.into(),
            headers: Vec::new(),
            body,
        }
    }

    /// Appends an HTTP header to the request.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets or replaces the request body payload.
    #[must_use]
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Finds the value of a header by case-insensitive name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        for (k, v) in &self.headers {
            if k.eq_ignore_ascii_case(name) {
                return Some(v.as_str());
            }
        }
        None
    }
}

/// Generic HTTP response representation returned by [`Transport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status code (e.g. 200, 400, 500).
    pub status: u16,
    /// Collection of response headers (name, value).
    pub headers: Vec<(String, String)>,
    /// Response payload body bytes.
    pub body: Vec<u8>,
}

impl Response {
    /// Creates a new `Response` with the specified status code and body.
    #[must_use]
    pub fn new(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body,
        }
    }

    /// Appends a response header.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Returns `true` if the status code indicates HTTP success (200..=299).
    #[inline]
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Finds the value of a response header by case-insensitive name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        for (k, v) in &self.headers {
            if k.eq_ignore_ascii_case(name) {
                return Some(v.as_str());
            }
        }
        None
    }

    /// Returns response body as a UTF-8 string slice.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] if the response body is not valid UTF-8.
    pub fn text(&self) -> Result<&str> {
        std::str::from_utf8(&self.body)
            .map_err(|e| Error::Xml(format!("response body is not valid UTF-8: {e}")))
    }

    /// Consumes the response and returns its raw body bytes.
    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}

/// Abstraction for transmitting HTTP requests to external endpoints (ADR-01, RF-01).
///
/// Implemented by test mock transports ([`MemoryTransport`]) and real network
/// clients ([`HttpTransport`]).
pub trait Transport: Send + Sync {
    /// Transmits an HTTP request and returns the received response.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if communication fails.
    fn send(&self, request: &Request) -> Result<Response>;
}

impl<T: Transport + ?Sized> Transport for &T {
    fn send(&self, request: &Request) -> Result<Response> {
        (**self).send(request)
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn send(&self, request: &Request) -> Result<Response> {
        (**self).send(request)
    }
}

impl<T: Transport + ?Sized> Transport for Arc<T> {
    fn send(&self, request: &Request) -> Result<Response> {
        (**self).send(request)
    }
}

type HandlerFn = Box<dyn Fn(&Request) -> Option<Result<Response>> + Send + Sync>;

struct MemoryInner {
    requests: Vec<Request>,
    handlers: Vec<HandlerFn>,
    fallback: Option<Response>,
}

/// In-memory mock transport for deterministic testing without network I/O (ADR-01, QA-01).
///
/// Records all transmitted requests and returns pre-configured responses based
/// on matching URL prefixes, exact URLs, or custom closure handlers.
pub struct MemoryTransport {
    inner: Mutex<MemoryInner>,
}

impl MemoryTransport {
    /// Creates a new empty `MemoryTransport`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MemoryInner {
                requests: Vec::new(),
                handlers: Vec::new(),
                fallback: None,
            }),
        }
    }

    /// Registers a response for requests matching an exact target URL.
    pub fn register_exact(&self, url: impl Into<String>, response: Response) {
        let target = url.into();
        self.register_handler(move |req| {
            if req.url == target {
                Some(Ok(response.clone()))
            } else {
                None
            }
        });
    }

    /// Registers a response for requests whose URL starts with the given prefix.
    pub fn register_prefix(&self, prefix: impl Into<String>, response: Response) {
        let prefix_str = prefix.into();
        self.register_handler(move |req| {
            if req.url.starts_with(&prefix_str) {
                Some(Ok(response.clone()))
            } else {
                None
            }
        });
    }

    /// Registers a dynamic handler closure evaluating incoming requests.
    pub fn register_handler<F>(&self, handler: F)
    where
        F: Fn(&Request) -> Option<Result<Response>> + Send + Sync + 'static,
    {
        if let Ok(mut lock) = self.inner.lock() {
            lock.handlers.push(Box::new(handler));
        }
    }

    /// Configures a default fallback response when no handler matches.
    pub fn set_fallback(&self, response: Response) {
        if let Ok(mut lock) = self.inner.lock() {
            lock.fallback = Some(response);
        }
    }

    /// Returns a list of all requests captured by this transport.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        if let Ok(lock) = self.inner.lock() {
            lock.requests.clone()
        } else {
            Vec::new()
        }
    }

    /// Returns the most recently transmitted request, if any.
    #[must_use]
    pub fn last_request(&self) -> Option<Request> {
        if let Ok(lock) = self.inner.lock() {
            lock.requests.last().cloned()
        } else {
            None
        }
    }

    /// Clears all recorded requests.
    pub fn clear_requests(&self) {
        if let Ok(mut lock) = self.inner.lock() {
            lock.requests.clear();
        }
    }
}

impl Default for MemoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for MemoryTransport {
    fn send(&self, request: &Request) -> Result<Response> {
        let mut lock = self
            .inner
            .lock()
            .map_err(|e| Error::Transport(format!("memory transport lock poisoned: {e}")))?;
        lock.requests.push(request.clone());

        for handler in &lock.handlers {
            if let Some(res) = handler(request) {
                return res;
            }
        }

        if let Some(ref fallback) = lock.fallback {
            return Ok(fallback.clone());
        }

        Err(Error::Transport(format!(
            "MemoryTransport: no matching response handler registered for URL: {}",
            request.url
        )))
    }
}

/// Real HTTP transport client using TLS via `rustls` (RF-03, feature `http`).
#[cfg(feature = "http")]
pub struct HttpTransport {
    agent: ureq::Agent,
}

#[cfg(feature = "http")]
impl HttpTransport {
    /// Creates a new `HttpTransport` with standard default TLS configuration.
    #[must_use]
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build();
        Self { agent }
    }

    /// Creates an `HttpTransport` wrapping a custom pre-configured `ureq::Agent`.
    #[must_use]
    pub fn with_agent(agent: ureq::Agent) -> Self {
        Self { agent }
    }
}

#[cfg(feature = "http")]
impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "http")]
impl Transport for HttpTransport {
    fn send(&self, request: &Request) -> Result<Response> {
        use std::io::Read as _;

        let mut call_builder = match request.method {
            Method::Get => self.agent.get(&request.url),
            Method::Post => self.agent.post(&request.url),
        };

        for (name, val) in &request.headers {
            call_builder = call_builder.set(name, val);
        }

        let ureq_res = if request.body.is_empty() {
            call_builder.call()
        } else {
            call_builder.send_bytes(&request.body)
        };

        let (status, resp) = match ureq_res {
            Ok(r) => (r.status(), r),
            Err(ureq::Error::Status(code, r)) => (code, r),
            Err(ureq::Error::Transport(transport_err)) => {
                return Err(Error::Transport(format!(
                    "HTTP connection error: {transport_err}"
                )));
            }
        };

        let mut headers = Vec::new();
        for name in resp.headers_names() {
            if let Some(val) = resp.header(&name) {
                headers.push((name, val.to_string()));
            }
        }

        let mut body = Vec::new();
        resp.into_reader()
            .read_to_end(&mut body)
            .map_err(|e| Error::Transport(format!("failed to read HTTP response body: {e}")))?;

        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_and_response_builders() {
        let req = Request::post("https://example.com/api", b"payload".to_vec())
            .with_header("Content-Type", "application/xml")
            .with_header("Cookie", "TOKEN=xyz");

        assert_eq!(req.method, Method::Post);
        assert_eq!(req.url, "https://example.com/api");
        assert_eq!(req.header("content-type"), Some("application/xml"));
        assert_eq!(req.header("cookie"), Some("TOKEN=xyz"));
        assert_eq!(req.header("missing"), None);
        assert_eq!(req.body, b"payload");

        let resp = Response::new(200, b"<xml>OK</xml>".to_vec())
            .with_header("Server", "Apache")
            .with_header("Content-Type", "text/xml");

        assert!(resp.is_success());
        assert_eq!(resp.status, 200);
        assert_eq!(resp.header("server"), Some("Apache"));
        assert_eq!(resp.text().unwrap(), "<xml>OK</xml>");
    }

    #[test]
    fn test_memory_transport_exact_matching() {
        let transport = MemoryTransport::new();
        let expected_resp = Response::new(200, b"response_body".to_vec());
        transport.register_exact("https://sii.cl/seed", expected_resp.clone());

        let target_request = Request::get("https://sii.cl/seed");
        let received_response = transport.send(&target_request).expect("send seed request");
        assert_eq!(received_response, expected_resp);

        let missing_request = Request::get("https://sii.cl/unknown");
        let unk_err = transport
            .send(&missing_request)
            .expect_err("should fail unknown");
        match unk_err {
            Error::Transport(msg) => assert!(msg.contains("no matching response handler")),
            other => panic!("expected Error::Transport, got {other:?}"),
        }

        let recorded = transport.requests();
        assert_eq!(recorded.len(), 2);
        assert_eq!(
            transport.last_request().unwrap().url,
            "https://sii.cl/unknown"
        );
    }

    #[test]
    fn test_memory_transport_prefix_matching() {
        let transport = MemoryTransport::new();
        transport.register_prefix(
            "https://sii.cl/api/",
            Response::new(200, b"api_ok".to_vec()),
        );

        let req = Request::get("https://sii.cl/api/resource/123");
        let res = transport.send(&req).expect("send prefix");
        assert_eq!(res.status, 200);
        assert_eq!(res.body, b"api_ok");
    }

    #[test]
    fn test_memory_transport_fallback() {
        let transport = MemoryTransport::new();
        transport.set_fallback(Response::new(404, b"not found".to_vec()));

        let req = Request::get("https://anywhere.com");
        let res = transport.send(&req).expect("fallback response");
        assert_eq!(res.status, 404);
        assert_eq!(res.body, b"not found");
    }
}
