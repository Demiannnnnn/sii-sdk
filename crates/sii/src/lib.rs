//! Unofficial Rust SDK for Chilean SII DTE (Electronic Tax Documents).
//!
//! Provides sans-IO DTE building, CAF handling, digital signing (`XMLDSig`),
//! stamp generation (TED), envelopes, and communication with the SII.

pub mod auth;
pub mod caf;
#[cfg(feature = "pkcs12")]
pub mod cert_loader;
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

pub use auth::{RespuestaSemilla, RespuestaToken, Semilla, SolicitudToken, Token, firmar_semilla};
pub use caf::Caf;
pub use codes::{Ambiente, TipoDte};
pub use dte::{
    BoletaBuilder, BoletaExentaBuilder, CdgItem, Chofer, Detalle, Dte, DteTimbrado, Emisor,
    Encabezado, FacturaAfectaBuilder, FacturaBuilder, FacturaExentaBuilder, GuiaDespachoBuilder,
    IdDoc, NotaCreditoBuilder, NotaDebitoBuilder, Receptor, Referencia, Totales, Transporte,
};
pub use envio::{EnvioBoleta, EnvioDte, RUT_RECEPTOR_SII};
pub use error::{Error, Result};
pub use libro::{
    CaratulaLibro, DetalleLibro, LibroCompraVenta, PeriodoTributario, TipoEnvio, TipoLibro,
    TipoOperacion, TotalesPeriodo,
};
pub use signature::{
    Certificate, DigestAlgorithm, SignatureAlgorithm, SignatureOptions, Signer, X509Certificate,
    canonicalize, sign, sign_dte, sign_enveloped, sign_targeting_before, verify,
    verify_by_reference,
};
pub use ted::{Dd, Ted};
pub use types::{Clock, ManualClock, Monto, Rut};

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
