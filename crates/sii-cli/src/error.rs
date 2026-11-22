//! Error definitions and formatting for the `sii` CLI.
//!
//! Provides typed errors with human-readable descriptions in Chilean Spanish,
//! guaranteeing that secrets (private keys, passwords) are never leaked in error
//! messages (SEC-02, SEC-03, RF-03).

use std::fmt;

/// CLI-level error encountered during execution.
#[derive(Debug)]
pub(crate) enum CliError {
    /// Missing required environment variable for certificate password.
    MissingCertPassword,
    /// An I/O error occurred accessing files.
    Io(std::io::Error),
    /// Error parsing a Chilean RUT (format or check digit mismatch).
    InvalidRut,
    /// Error parsing a date in YYYY-MM-DD or YYYY-MM format.
    InvalidDate(String),
    /// Error parsing a monetary amount.
    InvalidMonto(String),
    /// Error parsing or validating an electronic tax document (DTE).
    Dte(sii::Error),
    /// Error reading or validating a CAF authorization file.
    Caf(sii::Error),
    /// Error loading a digital certificate or private key.
    Certificate(String),
    /// Error generating or verifying digital signatures (`XMLDSig`).
    Signature(sii::Error),
    /// Error during network transport or SII Web Service communication.
    Transport(String),
    /// Error serializing or deserializing JSON data.
    Json(serde_json::Error),
    /// Missing required command line argument or option.
    MissingArgument(String),
    /// General invalid input or configuration.
    InvalidInput(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCertPassword => write!(
                f,
                "Error: la variable de entorno SII_CERT_PASSWORD no está configurada (requerida para abrir el certificado digital)"
            ),
            Self::Io(e) => write!(f, "Error de entrada/salida de archivos: {e}"),
            Self::InvalidRut => write!(f, "RUT inválido: formato o dígito verificador incorrecto"),
            Self::InvalidDate(s) => write!(f, "Fecha inválida (formato esperado YYYY-MM-DD): {s}"),
            Self::InvalidMonto(s) => write!(f, "Monto inválido: {s}"),
            Self::Dte(e) => write!(f, "Error en validación o armado de DTE: {e}"),
            Self::Caf(e) => write!(f, "Error en archivo CAF de autorización: {e}"),
            Self::Certificate(s) => write!(f, "Error al cargar certificado digital: {s}"),
            Self::Signature(e) => write!(f, "Error en firma digital XMLDSig: {e}"),
            Self::Transport(s) => write!(f, "Error de comunicación con el SII: {s}"),
            Self::Json(e) => write!(f, "Error en formato JSON: {e}"),
            Self::MissingArgument(s) => write!(f, "Falta argumento obligatorio: {s}"),
            Self::InvalidInput(s) => write!(f, "Entrada inválida: {s}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<sii::Error> for CliError {
    fn from(err: sii::Error) -> Self {
        match err {
            sii::Error::InvalidRut => Self::InvalidRut,
            sii::Error::InvalidDate(d) => Self::InvalidDate(d),
            sii::Error::InvalidMonto(m) => Self::InvalidMonto(m),
            sii::Error::InvalidCaf(_) | sii::Error::FolioOutOfRange { .. } => Self::Caf(err),
            sii::Error::Signature(_) | sii::Error::Crypto(_) => Self::Signature(err),
            sii::Error::Transport(t) => Self::Transport(t),
            _ => Self::Dte(err),
        }
    }
}
