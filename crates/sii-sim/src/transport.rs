//! Custom simulator transport implementations (`docs/md/simulator-spec.md` §4).

use crate::simulator::SiiSimulator;
use sii::error::Result as SiiResult;
use sii::transport::{Request, Response, Transport};
use std::sync::Arc;

/// Transport connecting an [`sii::SiiClient`] to the SII simulator (ADR-01, RF-02).
///
/// Can operate either purely in-memory (sans-IO) using [`SiiSimulator`],
/// or over local HTTP loopback network (feature `http`).
pub enum SimTransport {
    /// In-memory direct transport delegating immediately to [`SiiSimulator`].
    Direct(Arc<SiiSimulator>),
    /// Local HTTP client transport redirecting SII endpoints to a local server.
    #[cfg(feature = "http")]
    Http {
        /// Base URL of the local simulator server (e.g. `http://127.0.0.1:8080`).
        base_url: String,
        /// Underlying HTTP transport client.
        transport: sii::HttpTransport,
    },
}

impl SimTransport {
    /// Creates an in-memory `SimTransport` wrapping an [`SiiSimulator`].
    #[must_use]
    pub fn direct(simulator: Arc<SiiSimulator>) -> Self {
        Self::Direct(simulator)
    }

    /// Creates an in-memory `SimTransport` from a new default [`SiiSimulator`].
    #[must_use]
    pub fn memory() -> Self {
        Self::Direct(Arc::new(SiiSimulator::new()))
    }

    /// Creates an HTTP `SimTransport` targeting a local `sii-sim` server address.
    #[cfg(feature = "http")]
    #[must_use]
    pub fn http(base_url: impl Into<String>) -> Self {
        Self::Http {
            base_url: base_url.into(),
            transport: sii::HttpTransport::new(),
        }
    }
}

impl Transport for SimTransport {
    fn send(&self, request: &Request) -> SiiResult<Response> {
        match self {
            Self::Direct(sim) => sim.send(request),
            #[cfg(feature = "http")]
            Self::Http {
                base_url,
                transport,
            } => {
                let path = extract_url_path(&request.url);
                let target_url = format!("{}{path}", base_url.trim_end_matches('/'));

                let mut rewritten = request.clone();
                rewritten.url = target_url;

                transport.send(&rewritten)
            }
        }
    }
}

#[cfg(any(feature = "http", test))]
fn extract_url_path(url: &str) -> &str {
    if let Some(pos) = url.find("://") {
        let after_scheme = url.get(pos.saturating_add(3)..).unwrap_or_default();
        if let Some(slash_idx) = after_scheme.find('/') {
            return after_scheme.get(slash_idx..).unwrap_or(url);
        }
        return "/";
    }
    if url.starts_with('/') {
        return url;
    }
    "/"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_url_path() {
        assert_eq!(
            extract_url_path("https://maullin.sii.cl/DTEWS/CrSeed.jws"),
            "/DTEWS/CrSeed.jws"
        );
        assert_eq!(
            extract_url_path("https://palena.sii.cl/cgi_dte/UPL/DTEUpload"),
            "/cgi_dte/UPL/DTEUpload"
        );
        assert_eq!(
            extract_url_path("/DTEWS/QueryEstUp.jws"),
            "/DTEWS/QueryEstUp.jws"
        );
    }
}
