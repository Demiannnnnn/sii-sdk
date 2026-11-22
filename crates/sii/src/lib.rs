//! Unofficial Rust SDK for Chilean SII DTE (Electronic Tax Documents).
//!
//! Provides sans-IO DTE document modeling, CAF authorization parsing, digital signing (`XMLDSig`),
//! electronic stamp generation (`TED`), batch envelopes (`EnvioDTE` and `EnvioBOLETA`), monthly
//! ledgers (`LibroCompraVenta` / IECV), and communication with the Chilean SII Web Services.
//!
//! # Architecture Highlights
//!
//! - **Sans-IO Core (ADR-01):** The core library executes zero network I/O, disk access, or system clock queries.
//! - **Exact SII XML (ADR-03):** Conforms strictly to official Chilean SII XSD schemas (`DTE_v10.xsd`, `LibroCV_v10.xsd`).
//! - **Injected Cryptography (ADR-02):** Cryptographic keys and identities implement [`Signer`] and [`Certificate`].
//! - **Deterministic Testing:** In-memory transports and injectable clocks allow 100% reproducible testing.
//! - **Domain-Level Rejections (ADR-04):** Rejections from the SII are handled as domain values ([`EstadoEnvio::Rechazado`]), not runtime errors.
//!
//! # Quick Examples
//!
//! ## 1. Building a Factura Electrónica (Tipo 33)
//!
//! ```
//! use sii::codes::TipoDte;
//! use sii::dte::{Detalle, Emisor, FacturaAfectaBuilder, Receptor};
//! use sii::types::{Monto, Rut};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let emisor_rut = Rut::new(76_086_428, '5')?;
//! let receptor_rut = Rut::new(12_345_678, '5')?;
//!
//! let dte = FacturaAfectaBuilder::new()
//!     .folio(1)
//!     .fch_emis_str("2026-11-22")?
//!     .emisor(Emisor {
//!         rut_emisor: emisor_rut,
//!         rzn_soc: "Empresa Emisora SpA".into(),
//!         giro_emis: "Servicios Informáticos".into(),
//!         acteco: vec![620100],
//!         cdg_sii_sucur: None,
//!         dir_origen: "Av. Providencia 1234".into(),
//!         cmna_origen: "Providencia".into(),
//!         ciudad_origen: Some("Santiago".into()),
//!     })
//!     .receptor(Receptor::minimo(receptor_rut, "Cliente S.A."))
//!     .item(
//!         Detalle::new(1, "Desarrollo de Software", Monto::from_pesos(100_000))
//!             .qty(1)
//!             .prc(Monto::from_pesos(100_000)),
//!     )
//!     .build()?;
//!
//! // Net $100,000 + 19% IVA ($19,000) = Total $119,000
//! assert_eq!(dte.total(), Monto::from_pesos(119_000));
//! assert_eq!(dte.encabezado().totales.iva, Some(Monto::from_pesos(19_000)));
//!
//! // Generate canonical SII XML
//! let xml = dte.to_xml()?;
//! assert!(xml.starts_with(b"<DTE"));
//! # Ok(())
//! # }
//! ```
//!
//! ## 2. Monetary Math and RUT Validation
//!
//! ```
//! use sii::types::{Monto, Rut};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Validated Chilean RUT with modulo 11 check digit
//! let rut = Rut::parse("76.086.428-5")?;
//! assert_eq!(rut.to_string(), "76086428-5");
//! assert_eq!(rut.format_with_dots(), "76.086.428-5");
//!
//! // Integer CLP currency math without floating-point errors
//! let neto = Monto::from_pesos(100_000);
//! let iva = neto.iva_19()?;
//! let total = neto.add_monto(iva)?;
//! assert_eq!(total, Monto::from_pesos(119_000));
//! # Ok(())
//! # }
//! ```
//!
//! ## 3. Querying Envelope Status with Client
//!
//! ```
//! use sii::client::{EstadoEnvio, SiiClient, TrackId};
//! use sii::codes::Ambiente;
//! use sii::transport::{MemoryTransport, Response};
//! use sii::auth::Token;
//!
//! use sii::types::Rut;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let transport = MemoryTransport::new();
//! transport.set_fallback(Response::new(
//!     200,
//!     br#"<?xml version="1.0" encoding="ISO-8859-1"?>
//! <SOAP-ENV:Envelope xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/">
//!   <SOAP-ENV:Body>
//!     <getEstUpReturn>&lt;?xml version="1.0"?&gt;&lt;RESPUESTA&gt;&lt;ESTADO&gt;EPR&lt;/ESTADO&gt;&lt;ACEPTADOS&gt;1&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;0&lt;/RECHAZADOS&gt;&lt;/RESPUESTA&gt;</getEstUpReturn>
//!   </SOAP-ENV:Body>
//! </SOAP-ENV:Envelope>"#.to_vec(),
//! ));
//!
//! let client = SiiClient::new(Ambiente::Certificacion, transport)
//!     .with_rut(Rut::new(76_086_428, '5')?);
//! let token = Token::new("SESSION_TOKEN_123");
//! let estado = client.estado(TrackId::from_u64(123456), &token)?;
//!
//! assert!(matches!(estado, EstadoEnvio::Aceptado { .. }));
//! # Ok(())
//! # }
//! ```

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
pub use client::{DetalleEnvio, EstadoEnvio, SiiClient, TrackId};
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
#[cfg(feature = "http")]
pub use transport::HttpTransport;
pub use transport::{MemoryTransport, Method, Request, Response, Transport};
pub use types::{Clock, ManualClock, Monto, Rut};

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
