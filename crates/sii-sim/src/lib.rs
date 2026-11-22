//! # sii-sim
//!
//! **Unofficial — not affiliated with the Chilean Internal Revenue Service (SII).**
//!
//! Local simulation of Chilean SII DTE endpoints and Web Services (`docs/simulator-spec.md`):
//! - [`SiiSimulator`]: sans-IO simulation core implementing [`sii::transport::Transport`].
//! - [`Scenario`]: injectable failure and success scenarios (`Aceptado`, `Rechazado`, `EnProceso`, etc.).
//! - [`HttpServer`]: local TCP HTTP server binding exclusively to loopback `127.0.0.1` (QA-01).
//! - [`SimTransport`]: transport bridge connecting [`sii::SiiClient`] to the simulator.
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//! use sii::SiiClient;
//! use sii::codes::Ambiente;
//! use sii_sim::{Scenario, SiiSimulator, SimTransport};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let simulator = Arc::new(SiiSimulator::new().with_scenario(Scenario::Aceptado));
//! let transport = SimTransport::direct(simulator);
//! let client = SiiClient::new(Ambiente::Certificacion, transport);
//!
//! let seed = client.obtener_semilla()?;
//! assert!(!seed.as_str().is_empty());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod http;
pub mod scenario;
pub mod simulator;
pub mod transport;

pub use error::{Result, SimError};
pub use http::{HttpServer, HttpServerHandle};
pub use scenario::{Scenario, ScenarioConfig, TrackStatusConfig};
pub use simulator::{SiiSimulator, SimulatedUpload};
pub use transport::SimTransport;

/// Current simulator library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
