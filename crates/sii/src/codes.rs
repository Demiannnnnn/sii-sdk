//! Domain codes, enumerations, and SII constants.

use crate::error::Error;
use std::fmt;
use std::str::FromStr;

/// Factura electrónica (afecta a IVA 19 %). Código SII 33.
pub const FACTURA_AFECTA: u16 = 33;

/// Factura no afecta o exenta electrónica. Código SII 34.
pub const FACTURA_EXENTA: u16 = 34;

/// Boleta electrónica. Código SII 39.
pub const BOLETA: u16 = 39;

/// Boleta exenta electrónica. Código SII 41.
pub const BOLETA_EXENTA: u16 = 41;

/// Guía de despacho electrónica. Código SII 52.
pub const GUIA_DESPACHO: u16 = 52;

/// Nota de débito electrónica. Código SII 56.
pub const NOTA_DEBITO: u16 = 56;

/// Nota de crédito electrónica. Código SII 61.
pub const NOTA_CREDITO: u16 = 61;

/// Electronic Tax Document (DTE) type according to Chilean SII specification.
///
/// See `domain.md` §2 and `public-api.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u16)]
pub enum TipoDte {
    /// Factura electrónica (afecta a IVA 19 %) - Código 33.
    FacturaAfecta = FACTURA_AFECTA,
    /// Factura no afecta o exenta electrónica - Código 34.
    FacturaExenta = FACTURA_EXENTA,
    /// Boleta electrónica - Código 39.
    Boleta = BOLETA,
    /// Boleta exenta electrónica - Código 41.
    BoletaExenta = BOLETA_EXENTA,
    /// Guía de despacho electrónica - Código 52.
    GuiaDespacho = GUIA_DESPACHO,
    /// Nota de débito electrónica - Código 56.
    NotaDebito = NOTA_DEBITO,
    /// Nota de crédito electrónica - Código 61.
    NotaCredito = NOTA_CREDITO,
}

impl TipoDte {
    /// Returns the numerical code for this DTE type.
    #[inline]
    #[must_use]
    pub const fn code(&self) -> u16 {
        *self as u16
    }

    /// Alias for [`TipoDte::code`].
    #[inline]
    #[must_use]
    pub const fn as_u16(&self) -> u16 {
        self.code()
    }

    /// Returns `true` if this document is a boleta (types 39 and 41).
    #[inline]
    #[must_use]
    pub const fn is_boleta(&self) -> bool {
        matches!(self, Self::Boleta | Self::BoletaExenta)
    }

    /// Returns `true` if this document must be sent via `EnvioBOLETA`.
    #[inline]
    #[must_use]
    pub const fn is_envio_boleta(&self) -> bool {
        self.is_boleta()
    }

    /// Returns `true` if this document must be sent via `EnvioDTE`.
    #[inline]
    #[must_use]
    pub const fn is_envio_dte(&self) -> bool {
        !self.is_boleta()
    }

    /// Returns `true` if this document requires 19 % IVA calculation.
    #[inline]
    #[must_use]
    pub const fn requires_iva(&self) -> bool {
        matches!(
            self,
            Self::FacturaAfecta
                | Self::Boleta
                | Self::GuiaDespacho
                | Self::NotaDebito
                | Self::NotaCredito
        )
    }

    /// Returns `true` if this document is exempt from IVA.
    #[inline]
    #[must_use]
    pub const fn is_exento(&self) -> bool {
        matches!(self, Self::FacturaExenta | Self::BoletaExenta)
    }

    /// Returns `true` if this document requires at least one document reference (DTE-02).
    #[inline]
    #[must_use]
    pub const fn requires_reference(&self) -> bool {
        matches!(self, Self::NotaCredito | Self::NotaDebito)
    }

    /// Human-readable Spanish name for this document type.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::FacturaAfecta => "Factura electrónica",
            Self::FacturaExenta => "Factura electrónica exenta",
            Self::Boleta => "Boleta electrónica",
            Self::BoletaExenta => "Boleta exenta electrónica",
            Self::GuiaDespacho => "Guía de despacho electrónica",
            Self::NotaDebito => "Nota de débito electrónica",
            Self::NotaCredito => "Nota de crédito electrónica",
        }
    }
}

impl fmt::Display for TipoDte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl From<TipoDte> for u16 {
    #[inline]
    fn from(tipo: TipoDte) -> Self {
        tipo.code()
    }
}

impl TryFrom<u16> for TipoDte {
    type Error = Error;

    fn try_from(code: u16) -> Result<Self, Self::Error> {
        match code {
            FACTURA_AFECTA => Ok(Self::FacturaAfecta),
            FACTURA_EXENTA => Ok(Self::FacturaExenta),
            BOLETA => Ok(Self::Boleta),
            BOLETA_EXENTA => Ok(Self::BoletaExenta),
            GUIA_DESPACHO => Ok(Self::GuiaDespacho),
            NOTA_DEBITO => Ok(Self::NotaDebito),
            NOTA_CREDITO => Ok(Self::NotaCredito),
            other => Err(Error::InvalidTipoDte(other)),
        }
    }
}

impl FromStr for TipoDte {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if let Ok(code) = trimmed.parse::<u16>() {
            return Self::try_from(code);
        }

        match trimmed.to_ascii_lowercase().as_str() {
            "factura_afecta" | "facturaafecta" | "factura electrónica" | "factura electronica" => {
                Ok(Self::FacturaAfecta)
            }
            "factura_exenta"
            | "facturaexenta"
            | "factura exenta electrónica"
            | "factura exenta electronica" => Ok(Self::FacturaExenta),
            "boleta" | "boleta electrónica" | "boleta electronica" => Ok(Self::Boleta),
            "boleta_exenta"
            | "boletaexenta"
            | "boleta exenta electrónica"
            | "boleta exenta electronica" => Ok(Self::BoletaExenta),
            "guia_despacho"
            | "guiadespacho"
            | "guía de despacho electrónica"
            | "guia de despacho electronica" => Ok(Self::GuiaDespacho),
            "nota_debito"
            | "notadebito"
            | "nota de débito electrónica"
            | "nota de debito electronica" => Ok(Self::NotaDebito),
            "nota_credito"
            | "notacredito"
            | "nota de crédito electrónica"
            | "nota de credito electronica" => Ok(Self::NotaCredito),
            _ => Err(Error::InvalidTipoDte(0)),
        }
    }
}

/// SII execution environment.
///
/// In accordance with ADR-07 and SEC-04, there is no default environment pointing
/// to production. The caller must explicitly select the target environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Ambiente {
    /// Certification and testing environment (maullin).
    Certificacion,
    /// Real production environment (palena).
    Produccion,
}

impl Ambiente {
    /// Returns `true` if this is the certification environment.
    #[inline]
    #[must_use]
    pub const fn is_certificacion(&self) -> bool {
        matches!(self, Self::Certificacion)
    }

    /// Returns `true` if this is the production environment.
    #[inline]
    #[must_use]
    pub const fn is_produccion(&self) -> bool {
        matches!(self, Self::Produccion)
    }

    /// Returns the internal SII environment name ("maullin" or "palena").
    #[must_use]
    pub const fn sii_name(&self) -> &'static str {
        match self {
            Self::Certificacion => "maullin",
            Self::Produccion => "palena",
        }
    }

    /// Returns the base URL for the SII environment.
    #[must_use]
    pub const fn base_url(&self) -> &'static str {
        match self {
            Self::Certificacion => "https://maullin.sii.cl",
            Self::Produccion => "https://palena.sii.cl",
        }
    }

    /// Returns the Web Service URL for acquiring an authentication seed (`CrSeed.jws`).
    #[must_use]
    pub const fn seed_url(&self) -> &'static str {
        match self {
            Self::Certificacion => "https://maullin.sii.cl/DTEWS/CrSeed.jws",
            Self::Produccion => "https://palena.sii.cl/DTEWS/CrSeed.jws",
        }
    }

    /// Returns the Web Service URL for exchanging a signed seed for a token (`GetTokenFromSeed.jws`).
    #[must_use]
    pub const fn token_url(&self) -> &'static str {
        match self {
            Self::Certificacion => "https://maullin.sii.cl/DTEWS/GetTokenFromSeed.jws",
            Self::Produccion => "https://palena.sii.cl/DTEWS/GetTokenFromSeed.jws",
        }
    }

    /// Returns the HTTP endpoint URL for uploading DTE envelopes (`DTEUpload`).
    #[must_use]
    pub const fn upload_url(&self) -> &'static str {
        match self {
            Self::Certificacion => "https://maullin.sii.cl/cgi_dte/UPL/DTEUpload",
            Self::Produccion => "https://palena.sii.cl/cgi_dte/UPL/DTEUpload",
        }
    }

    /// Returns the Web Service URL for querying upload status by Track ID (`QueryEstUp.jws`).
    #[must_use]
    pub const fn query_status_url(&self) -> &'static str {
        match self {
            Self::Certificacion => "https://maullin.sii.cl/DTEWS/QueryEstUp.jws",
            Self::Produccion => "https://palena.sii.cl/DTEWS/QueryEstUp.jws",
        }
    }
}

impl fmt::Display for Ambiente {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Certificacion => write!(f, "Certificacion"),
            Self::Produccion => write!(f, "Produccion"),
        }
    }
}

impl FromStr for Ambiente {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "certificacion" | "cert" | "maullin" => Ok(Self::Certificacion),
            "produccion" | "prod" | "palena" => Ok(Self::Produccion),
            other => Err(Error::InvalidAmbiente(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tipo_dte_u16_roundtrip() {
        let all_types = [
            TipoDte::FacturaAfecta,
            TipoDte::FacturaExenta,
            TipoDte::Boleta,
            TipoDte::BoletaExenta,
            TipoDte::GuiaDespacho,
            TipoDte::NotaDebito,
            TipoDte::NotaCredito,
        ];

        for &tipo in &all_types {
            let code = tipo.code();
            let parsed = TipoDte::try_from(code).expect("should convert from code");
            assert_eq!(tipo, parsed);
            assert_eq!(u16::from(tipo), code);
        }
    }

    #[test]
    fn test_tipo_dte_invalid_code() {
        assert!(TipoDte::try_from(0).is_err());
        assert!(TipoDte::try_from(30).is_err());
        assert!(TipoDte::try_from(999).is_err());
    }

    #[test]
    fn test_tipo_dte_boleta_vs_dte() {
        assert!(TipoDte::Boleta.is_boleta());
        assert!(TipoDte::Boleta.is_envio_boleta());
        assert!(!TipoDte::Boleta.is_envio_dte());

        assert!(TipoDte::BoletaExenta.is_boleta());
        assert!(TipoDte::BoletaExenta.is_envio_boleta());
        assert!(!TipoDte::BoletaExenta.is_envio_dte());

        assert!(!TipoDte::FacturaAfecta.is_boleta());
        assert!(!TipoDte::FacturaAfecta.is_envio_boleta());
        assert!(TipoDte::FacturaAfecta.is_envio_dte());

        assert!(!TipoDte::GuiaDespacho.is_boleta());
        assert!(TipoDte::GuiaDespacho.is_envio_dte());
    }

    #[test]
    fn test_tipo_dte_requires_reference() {
        assert!(TipoDte::NotaCredito.requires_reference());
        assert!(TipoDte::NotaDebito.requires_reference());
        assert!(!TipoDte::FacturaAfecta.requires_reference());
        assert!(!TipoDte::Boleta.requires_reference());
    }

    #[test]
    fn test_ambiente_methods() {
        let cert = Ambiente::Certificacion;
        assert!(cert.is_certificacion());
        assert!(!cert.is_produccion());
        assert_eq!(cert.sii_name(), "maullin");

        let prod = Ambiente::Produccion;
        assert!(prod.is_produccion());
        assert!(!prod.is_certificacion());
        assert_eq!(prod.sii_name(), "palena");
    }

    #[test]
    fn test_ambiente_from_str() {
        assert_eq!(
            "certificacion".parse::<Ambiente>().unwrap(),
            Ambiente::Certificacion
        );
        assert_eq!(
            "maullin".parse::<Ambiente>().unwrap(),
            Ambiente::Certificacion
        );
        assert_eq!(
            "PRODUCCION".parse::<Ambiente>().unwrap(),
            Ambiente::Produccion
        );
        assert_eq!("palena".parse::<Ambiente>().unwrap(), Ambiente::Produccion);
        assert!("invalid".parse::<Ambiente>().is_err());
    }
}
