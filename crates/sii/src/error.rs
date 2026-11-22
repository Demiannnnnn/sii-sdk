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

    /// Folio number is outside the authorized CAF range [min, max] (CAF-01).
    #[error("folio {folio} is outside authorized CAF range [{min}, {max}]")]
    FolioOutOfRange {
        /// The requested folio number.
        folio: u64,
        /// The minimum authorized folio.
        min: u64,
        /// The maximum authorized folio.
        max: u64,
    },

    /// CAF digital signature (FRMA) verification failed (CAF-01).
    #[error("CAF digital signature verification failed: {0}")]
    CafSignature(String),

    /// Invalid CAF structure or contents.
    #[error("invalid CAF: {0}")]
    InvalidCaf(String),

    /// DTE validation error (business or schema rules violated).
    #[error("DTE validation error: {0}")]
    DteValidation(String),

    /// XML parsing or serialization error.
    #[error("XML error: {0}")]
    Xml(String),

    /// Invalid date format or value.
    #[error("invalid date: {0}")]
    InvalidDate(String),

    /// Cryptographic operation error (RSA, hashing, key parsing).
    #[error("cryptographic error: {0}")]
    Crypto(String),

    /// Digital signature error (`XMLDSig` signing or verification).
    #[error("signature error: {0}")]
    Signature(String),

    /// Authentication error with SII (seed acquisition or token exchange).
    #[error("authentication error: {0}")]
    Auth(String),
}
