//! SII authentication, seed acquisition, and session token management.
//!
//! Implements Chilean SII authentication protocol (`docs/md/dte-spec.md` §7, AUTH-01):
//! 1. Acquire seed (`Semilla`) from `crTicket.getSeed` / `CrSeed.jws`.
//! 2. Digitally sign the seed using enveloped `XMLDSig` over `<getToken>` (`firmar_semilla`).
//! 3. Exchange signed seed for session token (`Token`) via `GetTokenFromSeed.jws`.
//! 4. Use `Token` as authorization credential for subsequent operations (SEC-02).

use crate::error::{Error, Result};
use crate::signature::{
    Certificate, DigestAlgorithm, SignatureAlgorithm, SignatureOptions, Signer,
};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fmt;

/// Chilean SII seed string received during authentication session initialization (`crTicket.getSeed`).
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Semilla(String);

impl Semilla {
    /// Creates a new `Semilla` from a string or numeric value.
    pub fn new(valor: impl Into<String>) -> Self {
        Self(valor.into().trim().to_string())
    }

    /// Parses a seed from raw SII XML seed response bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if XML parsing fails or the SII returned an error status.
    pub fn parse_xml(xml: &[u8]) -> Result<Self> {
        RespuestaSemilla::parse(xml)?.into_semilla()
    }

    /// Returns the string representation of the seed value.
    #[inline]
    #[must_use]
    pub fn valor(&self) -> &str {
        &self.0
    }

    /// Returns the string representation of the seed value.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Signs this seed using the provided signer and certificate to produce a [`SolicitudToken`].
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if signing or cryptographic key extraction fails.
    pub fn firmar(&self, signer: &dyn Signer, cert: &dyn Certificate) -> Result<SolicitudToken> {
        firmar_semilla(self, signer, cert)
    }
}

impl AsRef<str> for Semilla {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Semilla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Semilla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Semilla").field(&self.0).finish()
    }
}

/// Chilean SII authentication session token (SEC-02).
///
/// Treated as a sensitive secret credential: its value is redacted in both
/// [`fmt::Debug`] and [`fmt::Display`] to prevent leakage in log files or diagnostic traces.
/// Access the raw token string with [`Token::as_str`] or [`Token::valor`] when constructing
/// HTTP authorization headers.
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token(String);

impl Token {
    /// Creates a new `Token` from a raw token string.
    pub fn new(valor: impl Into<String>) -> Self {
        Self(valor.into().trim().to_string())
    }

    /// Parses an authentication token from raw SII XML token response bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if XML parsing fails or the SII returned an error status.
    pub fn parse_xml(xml: &[u8]) -> Result<Self> {
        RespuestaToken::parse(xml)?.into_token()
    }

    /// Returns the raw token string slice.
    #[inline]
    #[must_use]
    pub fn valor(&self) -> &str {
        &self.0
    }

    /// Returns the raw token string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Token").field(&"[REDACTED]").finish()
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Signed `getToken` XML document ready to be submitted to the Chilean SII.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolicitudToken {
    xml: String,
    semilla: Semilla,
}

impl SolicitudToken {
    /// Creates a new `SolicitudToken` with its signed XML and originating seed.
    #[must_use]
    pub fn new(xml: String, semilla: Semilla) -> Self {
        Self { xml, semilla }
    }

    /// Returns the signed XML string.
    #[inline]
    #[must_use]
    pub fn xml(&self) -> &str {
        &self.xml
    }

    /// Returns the signed XML bytes.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.xml.as_bytes()
    }

    /// Consumes the request and returns the signed XML string.
    #[inline]
    #[must_use]
    pub fn into_xml(self) -> String {
        self.xml
    }

    /// Returns the seed used to construct this token request.
    #[inline]
    #[must_use]
    pub fn semilla(&self) -> &Semilla {
        &self.semilla
    }

    /// Verifies the digital signature contained in this token request (SIG-02).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if verification fails due to malformed XML or cryptographic errors.
    pub fn verify(&self) -> Result<bool> {
        crate::signature::verify(self.as_bytes())
    }
}

/// Parsed response from the Chilean SII seed acquisition service.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RespuestaSemilla {
    /// Status code returned by the SII ("00" or "0" indicates success).
    pub estado: String,
    /// Explanatory message or description.
    pub glosa: String,
    /// Acquired seed if the response was successful.
    pub semilla: Option<Semilla>,
}

impl RespuestaSemilla {
    /// Parses an SII XML seed response.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] if the XML is malformed or lacks an `<ESTADO>` tag.
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let raw = parse_sii_envelope_response(xml, b"SEMILLA")?;
        let semilla = raw.body_val.map(Semilla::new);
        Ok(Self {
            estado: raw.estado,
            glosa: raw.glosa,
            semilla,
        })
    }

    /// Returns `true` if the SII status code indicates success ("00" or "0").
    #[inline]
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.estado == "00" || self.estado == "0"
    }

    /// Returns a reference to the seed if the response was successful.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`] if the status code indicates failure or no seed was returned.
    pub fn semilla(&self) -> Result<&Semilla> {
        if !self.is_ok() {
            let msg = if self.glosa.is_empty() {
                format!("SII returned error status {}", self.estado)
            } else {
                format!("SII returned error status {}: {}", self.estado, self.glosa)
            };
            return Err(Error::Auth(msg));
        }
        self.semilla.as_ref().ok_or_else(|| {
            Error::Auth("SII response indicated success but contained no seed value".to_string())
        })
    }

    /// Consumes the response and returns the seed if successful.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`] if the status code indicates failure or no seed was returned.
    pub fn into_semilla(self) -> Result<Semilla> {
        if !self.is_ok() {
            let msg = if self.glosa.is_empty() {
                format!("SII returned error status {}", self.estado)
            } else {
                format!("SII returned error status {}: {}", self.estado, self.glosa)
            };
            return Err(Error::Auth(msg));
        }
        self.semilla.ok_or_else(|| {
            Error::Auth("SII response indicated success but contained no seed value".to_string())
        })
    }
}

/// Parsed response from the Chilean SII token acquisition service.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RespuestaToken {
    /// Status code returned by the SII ("00" or "0" indicates success).
    pub estado: String,
    /// Explanatory message or description.
    pub glosa: String,
    /// Acquired authorization token if the response was successful.
    pub token: Option<Token>,
}

impl RespuestaToken {
    /// Parses an SII XML token response.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] if the XML is malformed or lacks an `<ESTADO>` tag.
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let raw = parse_sii_envelope_response(xml, b"TOKEN")?;
        let token = raw.body_val.map(Token::new);
        Ok(Self {
            estado: raw.estado,
            glosa: raw.glosa,
            token,
        })
    }

    /// Returns `true` if the SII status code indicates success ("00" or "0").
    #[inline]
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.estado == "00" || self.estado == "0"
    }

    /// Returns a reference to the token if the response was successful.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`] if the status code indicates failure or no token was returned.
    pub fn token(&self) -> Result<&Token> {
        if !self.is_ok() {
            let msg = if self.glosa.is_empty() {
                format!("SII returned error status {}", self.estado)
            } else {
                format!("SII returned error status {}: {}", self.estado, self.glosa)
            };
            return Err(Error::Auth(msg));
        }
        self.token.as_ref().ok_or_else(|| {
            Error::Auth("SII response indicated success but contained no token value".to_string())
        })
    }

    /// Consumes the response and returns the token if successful.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`] if the status code indicates failure or no token was returned.
    pub fn into_token(self) -> Result<Token> {
        if !self.is_ok() {
            let msg = if self.glosa.is_empty() {
                format!("SII returned error status {}", self.estado)
            } else {
                format!("SII returned error status {}: {}", self.estado, self.glosa)
            };
            return Err(Error::Auth(msg));
        }
        self.token.ok_or_else(|| {
            Error::Auth("SII response indicated success but contained no token value".to_string())
        })
    }
}

impl fmt::Debug for RespuestaToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RespuestaToken")
            .field("estado", &self.estado)
            .field("glosa", &self.glosa)
            .field("token", &self.token)
            .finish()
    }
}

/// Signs an SII authentication seed (`Semilla`) using `XMLDSig` to generate a [`SolicitudToken`].
///
/// Implements Chilean SII `XMLDSig` authentication standard (`docs/md/dte-spec.md` §7, RF-02):
/// Constructs `<getToken><item><Semilla>...</Semilla></item></getToken>` and applies an
/// enveloped `XMLDSig` signature using RSA-SHA1 with `URI=""`.
/// The resulting signed XML is reproducible and deterministic given the same seed and key.
///
/// # Errors
///
/// Returns [`Error`] if the seed is empty, signing fails, or cryptographic key extraction fails.
pub fn firmar_semilla(
    semilla: &Semilla,
    signer: &dyn Signer,
    cert: &dyn Certificate,
) -> Result<SolicitudToken> {
    if semilla.valor().is_empty() {
        return Err(Error::Auth("cannot sign an empty seed".to_string()));
    }
    let unsigned_xml = format!(
        "<getToken><item><Semilla>{}</Semilla></item></getToken>",
        semilla.valor()
    );
    let options = SignatureOptions {
        signature_algorithm: SignatureAlgorithm::RsaSha1,
        digest_algorithm: DigestAlgorithm::Sha1,
    };
    let signed_bytes =
        crate::signature::sign_enveloped(unsigned_xml.as_bytes(), signer, cert, options)?;
    let signed_str = String::from_utf8(signed_bytes)
        .map_err(|e| Error::Xml(format!("invalid UTF-8 in signed XML: {e}")))?;
    Ok(SolicitudToken::new(signed_str, semilla.clone()))
}

struct RawSiiResponse {
    estado: String,
    glosa: String,
    body_val: Option<String>,
}

fn parse_sii_envelope_response(xml: &[u8], body_tag_name: &[u8]) -> Result<RawSiiResponse> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut estado = String::new();
    let mut glosa = String::new();
    let mut body_val = None;

    let mut current_tag = Vec::new();
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_tag = e.name().local_name().as_ref().to_ascii_uppercase();
            }
            Ok(Event::Text(ref t)) => {
                let text = t.unescape().map_err(|e| Error::Xml(e.to_string()))?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    match current_tag.as_slice() {
                        b"ESTADO" => {
                            if estado.is_empty() {
                                estado = trimmed.to_string();
                            }
                        }
                        b"GLOSA" => {
                            if glosa.is_empty() {
                                glosa = trimmed.to_string();
                            }
                        }
                        tag if tag == body_tag_name && body_val.is_none() => {
                            body_val = Some(trimmed.to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(ref t)) => {
                let text = String::from_utf8_lossy(t.as_ref());
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    match current_tag.as_slice() {
                        b"ESTADO" => {
                            if estado.is_empty() {
                                estado = trimmed.to_string();
                            }
                        }
                        b"GLOSA" => {
                            if glosa.is_empty() {
                                glosa = trimmed.to_string();
                            }
                        }
                        tag if tag == body_tag_name && body_val.is_none() => {
                            body_val = Some(trimmed.to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => {
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(format!("failed to parse XML response: {e}"))),
            _ => {}
        }
    }

    if estado.is_empty() {
        return Err(Error::Xml(
            "missing required <ESTADO> tag in SII response".to_string(),
        ));
    }

    Ok(RawSiiResponse {
        estado,
        glosa,
        body_val,
    })
}
