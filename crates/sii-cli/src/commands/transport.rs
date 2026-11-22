//! Network and simulation transport abstraction for CLI operations.

use sii::error::Result as SiiResult;
use sii::transport::{HttpTransport, Request, Response, Transport};
use sii_sim::SimTransport;

/// Composite transport implementation supporting in-memory simulation,
/// local HTTP simulation, and live SII HTTP requests.
pub(crate) enum CliTransport {
    /// In-memory or local HTTP simulator transport.
    Simulator(SimTransport),
    /// Real HTTP network transport against Chilean SII servers.
    Http(HttpTransport),
}

impl CliTransport {
    /// Builds a transport for the simulator or live environment.
    #[must_use]
    pub(crate) fn build(is_sim: bool, sim_url: Option<&str>) -> Self {
        if let Some(url) = sim_url {
            Self::Simulator(SimTransport::http(url))
        } else if is_sim {
            Self::Simulator(SimTransport::memory())
        } else {
            Self::Http(HttpTransport::new())
        }
    }
}

impl Transport for CliTransport {
    fn send(&self, request: &Request) -> SiiResult<Response> {
        match self {
            Self::Simulator(sim) => sim.send(request),
            Self::Http(http) => http.send(request),
        }
    }
}
