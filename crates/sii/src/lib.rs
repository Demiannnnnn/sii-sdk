//! Unofficial Rust SDK for Chilean SII DTE (Electronic Tax Documents).
//!
//! Provides sans-IO DTE building, CAF handling, digital signing (`XMLDSig`),
//! stamp generation (TED), envelopes, and communication with the SII.

pub mod auth;
pub mod caf;
pub mod client;
pub mod codes;
pub mod dte;
pub mod envio;
pub mod error;
pub mod libro;
pub mod signature;
pub mod ted;
pub mod transport;
pub mod types;

pub use caf::Caf;
pub use codes::{Ambiente, TipoDte};
pub use dte::{
    CdgItem, Detalle, Dte, DteTimbrado, Emisor, Encabezado, FacturaAfectaBuilder, FacturaBuilder,
    FacturaExentaBuilder, IdDoc, Receptor, Totales,
};
pub use error::{Error, Result};
pub use signature::{
    Certificate, DigestAlgorithm, SignatureAlgorithm, SignatureOptions, Signer, X509Certificate,
    canonicalize, sign, sign_dte, verify,
};
pub use ted::{Dd, Ted};
pub use types::{Clock, ManualClock, Monto, Rut};

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
