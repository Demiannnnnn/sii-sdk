//! Library error types and variants.

use thiserror::Error;

/// Result alias for operations in the `sii` crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error types returned by the `sii` SDK.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum Error {
    /// Invalid Chilean RUT (format or check digit mismatch).
    #[error("invalid RUT: format or verification digit is incorrect")]
    InvalidRut,

    /// Invalid amount in CLP or arithmetic overflow (DTE-03).
    #[error("invalid amount: {0}")]
    InvalidMonto(String),

    /// Invalid or unsupported DTE type code.
    #[error("invalid DTE type code: {0}")]
    InvalidTipoDte(u16),

    /// Invalid environment specification.
    #[error("invalid environment: {0}")]
    InvalidAmbiente(String),

    /// DTE validation error (business or schema rules violated).
    #[error("DTE validation error: {0}")]
    DteValidation(String),

    /// XML generation or parsing error.
    #[error("XML error: {0}")]
    Xml(String),

    /// Invalid date format or value.
    #[error("invalid date: {0}")]
    InvalidDate(String),
}
