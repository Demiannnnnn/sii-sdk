//! Error types for the SII simulator.

/// Result type specialized for simulator operations.
pub type Result<T, E = SimError> = std::result::Result<T, E>;

/// Errors that can occur within the SII simulator.
#[derive(Debug, thiserror::Error)]
pub enum SimError {
    /// I/O error during network communication or file reading.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Error originating from the core SII SDK.
    #[error("SII SDK error: {0}")]
    Sii(#[from] sii::Error),

    /// Attempted to bind to a non-loopback address without permission (QA-01).
    #[error(
        "refusing to bind to non-loopback address '{0}'; only 127.0.0.1 loopback is permitted (QA-01)"
    )]
    NonLoopbackAddress(String),

    /// Received an invalid or unsupported HTTP request.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Internal synchronization lock was poisoned.
    #[error("internal lock poisoned: {0}")]
    LockPoisoned(String),

    /// HTTP client transport error.
    #[error("HTTP client error: {0}")]
    Http(String),
}

impl<T> From<std::sync::PoisonError<T>> for SimError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Self::LockPoisoned(format!("{err}"))
    }
}
