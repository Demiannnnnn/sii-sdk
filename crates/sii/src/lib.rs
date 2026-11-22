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

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
