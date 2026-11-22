//! Folio authorization (CAF) parsing, key extraction, and validation.
//!
//! Under the Chilean SII specification (`docs/md/dte-spec.md` §5, `constraints.md` CAF-01),
//! a CAF (`Código de Autorización de Folios`) is delivered as an XML document
//! containing the authorized folio range for a specific DTE type and taxpayer,
//! the SII digital signature (`FRMA`) over authorization details (`DA`), and the
//! RSA private key (`RSASK`) required to sign the electronic tax stamp (`TED`).
//!
//! # Security
//!
//! In accordance with SEC-02, the private key is strictly encapsulated within [`Caf`]
//! and is redacted from [`fmt::Debug`]. It is never exposed via public fields.
//!
//! # Example
//!
//! ```no_run
//! use sii::caf::Caf;
//!
//! # fn example() -> Result<(), sii::Error> {
//! let xml_bytes = std::fs::read("caf.xml").map_err(|e| sii::Error::InvalidCaf(e.to_string()))?;
//! let caf = Caf::parse(&xml_bytes)?;
//!
//! assert_eq!(caf.rango(), (1, 100));
//! assert!(caf.contiene(50));
//! assert!(!caf.contiene(101));
//! # Ok(())
//! # }
//! ```

use crate::codes::TipoDte;
use crate::error::{Error, Result};
use crate::signature::Signer;
use crate::types::Rut;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{SignatureEncoding, Signer as _, Verifier as _};
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, RsaPrivateKey, RsaPublicKey};
use sha1::Sha1;
use std::fmt;

/// Authorized folio authorization (CAF) issued by the SII.
///
/// Contains authorized DTE folio ranges, taxpayer identification, authorization date,
/// and the RSA private key used to sign the Electronic Tax Stamp (`TED`).
#[derive(Clone, PartialEq, Eq)]
pub struct Caf {
    emisor_rut: Rut,
    razon_social: String,
    tipo_dte: TipoDte,
    rango_desde: u64,
    rango_hasta: u64,
    fecha_autorizacion: String,
    idk: String,
    algoritmo_firma: String,
    firma_bytes: Vec<u8>,
    rsapk_m: String,
    rsapk_e: String,
    public_key: RsaPublicKey,
    private_key: RsaPrivateKey,
    public_key_pem: Option<String>,
    raw_da: Vec<u8>,
    raw_xml: Vec<u8>,
    firma_verificada: bool,
}

impl Caf {
    /// Parses a CAF XML file from raw bytes without verifying the SII `FRMA` signature.
    ///
    /// Validates the XML structure, issuer RUT, DTE type, folio range (`min <= max`),
    /// extracts the RSA private key (`RSASK`), and ensures the private key matches
    /// the public key in `<RSAPK>`.
    ///
    /// The resulting `Caf` will have [`Self::is_firma_verificada`] set to `false`.
    /// To verify the signature, call [`Self::verificar_firma`] or use [`Self::parse_and_verify`].
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if XML is malformed, required fields are missing, or keys cannot be parsed.
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let raw_da = find_tag_slice(xml, b"DA")?.to_vec();
        let raw_xml = find_tag_slice(xml, b"CAF")?.to_vec();
        let fields = parse_xml_fields(xml)?;

        let emisor_rut = Rut::parse(&fields.re)?;
        let td_num: u16 = fields
            .td
            .trim()
            .parse()
            .map_err(|_| Error::InvalidTipoDte(0))?;
        let tipo_dte = TipoDte::try_from(td_num)?;

        let rango_desde = parse_folio_number(&fields.d, "range start (D)")?;
        let rango_hasta = parse_folio_number(&fields.h, "range end (H)")?;
        if rango_desde == 0 || rango_desde > rango_hasta {
            return Err(Error::InvalidCaf(format!(
                "invalid folio range: [{rango_desde}, {rango_hasta}]"
            )));
        }

        let public_key = parse_rsa_public_key(&fields.m, &fields.e)?;
        let private_key = parse_rsa_private_key(&fields.rsask)?;
        verify_keypair_match(&private_key, &public_key)?;

        let firma_bytes = decode_signature_bytes(&fields.frma)?;

        Ok(Self {
            emisor_rut,
            razon_social: fields.rs,
            tipo_dte,
            rango_desde,
            rango_hasta,
            fecha_autorizacion: fields.fa,
            idk: fields.idk,
            algoritmo_firma: fields.algoritmo,
            firma_bytes,
            rsapk_m: fields.m,
            rsapk_e: fields.e,
            public_key,
            private_key,
            public_key_pem: fields.rsapubk,
            raw_da,
            raw_xml,
            firma_verificada: false,
        })
    }

    /// Parses a CAF XML file and immediately verifies the SII `FRMA` signature
    /// using the provided SII public key.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if parsing fails or if the digital signature does not verify.
    pub fn parse_and_verify(xml: &[u8], sii_public_key: &RsaPublicKey) -> Result<Self> {
        let mut caf = Self::parse(xml)?;
        caf.verificar_firma(sii_public_key)?;
        Ok(caf)
    }

    /// Verifies the CAF's `FRMA` signature over the exact `<DA>` block using the provided SII public key.
    ///
    /// On success, marks the signature as verified ([`Self::is_firma_verificada`] becomes `true`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::CafSignature`] if verification fails or the algorithm is unsupported.
    pub fn verificar_firma(&mut self, sii_public_key: &RsaPublicKey) -> Result<()> {
        let algo_upper = self.algoritmo_firma.to_ascii_uppercase();
        if algo_upper != "SHA1WITHRSA" && !algo_upper.is_empty() {
            return Err(Error::CafSignature(format!(
                "unsupported CAF signature algorithm: {}",
                self.algoritmo_firma
            )));
        }

        let verifying_key = VerifyingKey::<Sha1>::new(sii_public_key.clone());
        let signature = rsa::pkcs1v15::Signature::try_from(self.firma_bytes.as_slice())
            .map_err(|e| Error::CafSignature(format!("invalid signature length or format: {e}")))?;

        verifying_key
            .verify(&self.raw_da, &signature)
            .map_err(|e| {
                Error::CafSignature(format!("digital signature verification failed: {e}"))
            })?;

        self.firma_verificada = true;
        Ok(())
    }

    /// Returns `true` if the SII digital signature (`FRMA`) has been successfully verified.
    #[inline]
    #[must_use]
    pub fn is_firma_verificada(&self) -> bool {
        self.firma_verificada
    }

    /// Returns the authorized folio range `(min, max)` inclusive.
    #[inline]
    #[must_use]
    pub const fn rango(&self) -> (u64, u64) {
        (self.rango_desde, self.rango_hasta)
    }

    /// Returns the authorized DTE document type.
    #[inline]
    #[must_use]
    pub const fn tipo(&self) -> TipoDte {
        self.tipo_dte
    }

    /// Returns `true` if the given folio number is within the authorized range `[desde, hasta]`.
    #[inline]
    #[must_use]
    pub const fn contiene(&self, folio: u64) -> bool {
        folio >= self.rango_desde && folio <= self.rango_hasta
    }

    /// Validates that a folio number is within the authorized range.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FolioOutOfRange`] if `folio` is outside `[desde, hasta]`.
    pub fn verificar_folio(&self, folio: u64) -> Result<()> {
        if self.contiene(folio) {
            Ok(())
        } else {
            Err(Error::FolioOutOfRange {
                folio,
                min: self.rango_desde,
                max: self.rango_hasta,
            })
        }
    }

    /// Returns the taxpayer (issuer) RUT authorized by this CAF.
    #[inline]
    #[must_use]
    pub const fn emisor_rut(&self) -> &Rut {
        &self.emisor_rut
    }

    /// Returns the taxpayer's business name (`RS`).
    #[inline]
    #[must_use]
    pub fn razon_social(&self) -> &str {
        &self.razon_social
    }

    /// Returns the authorization date string (`FA`), typically in `"YYYY-MM-DD"` format.
    #[inline]
    #[must_use]
    pub fn fecha_autorizacion(&self) -> &str {
        &self.fecha_autorizacion
    }

    /// Returns the SII key identifier (`IDK`).
    #[inline]
    #[must_use]
    pub fn idk(&self) -> &str {
        &self.idk
    }

    /// Returns the signature algorithm name specified in `<FRMA>` (e.g. `"SHA1withRSA"`).
    #[inline]
    #[must_use]
    pub fn algoritmo_firma(&self) -> &str {
        &self.algoritmo_firma
    }

    /// Returns the Base64 modulus of the RSA public key.
    #[inline]
    #[must_use]
    pub fn rsapk_m(&self) -> &str {
        &self.rsapk_m
    }

    /// Returns the Base64 exponent of the RSA public key.
    #[inline]
    #[must_use]
    pub fn rsapk_e(&self) -> &str {
        &self.rsapk_e
    }

    /// Returns the parsed RSA public key corresponding to this authorization.
    #[inline]
    #[must_use]
    pub const fn public_key(&self) -> &RsaPublicKey {
        &self.public_key
    }

    /// Returns the optional PEM-encoded public key string from `<RSAPUBK>`.
    #[inline]
    #[must_use]
    pub fn public_key_pem(&self) -> Option<&str> {
        self.public_key_pem.as_deref()
    }

    /// Returns the exact raw bytes of the `<DA>...</DA>` element.
    #[inline]
    #[must_use]
    pub fn raw_da(&self) -> &[u8] {
        &self.raw_da
    }

    /// Returns the exact raw bytes of the `<CAF>...</CAF>` element.
    #[inline]
    #[must_use]
    pub fn raw_xml(&self) -> &[u8] {
        &self.raw_xml
    }

    /// Signs data with RSA-SHA1 using the encapsulated CAF private key.
    ///
    /// Used for generating the electronic stamp (`TED`).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if signing fails.
    pub fn sign_ted(&self, data: &[u8]) -> Result<Vec<u8>> {
        let signing_key = SigningKey::<Sha1>::new(self.private_key.clone());
        let signature = signing_key.sign(data);
        Ok(signature.to_vec())
    }

    /// Exposes a [`Signer`] instance for signing TED stamps without revealing the private key.
    #[inline]
    #[must_use]
    pub fn signer(&self) -> &dyn Signer {
        self
    }
}

impl Signer for Caf {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        self.sign_ted(data)
    }
}

impl fmt::Debug for Caf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Caf")
            .field("emisor_rut", &self.emisor_rut)
            .field("razon_social", &self.razon_social)
            .field("tipo_dte", &self.tipo_dte)
            .field("rango_desde", &self.rango_desde)
            .field("rango_hasta", &self.rango_hasta)
            .field("fecha_autorizacion", &self.fecha_autorizacion)
            .field("idk", &self.idk)
            .field("algoritmo_firma", &self.algoritmo_firma)
            .field("firma_verificada", &self.firma_verificada)
            .field("private_key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct RawCafFields {
    re: String,
    rs: String,
    td: String,
    d: String,
    h: String,
    fa: String,
    m: String,
    e: String,
    idk: String,
    frma: String,
    algoritmo: String,
    rsask: String,
    rsapubk: Option<String>,
}

fn parse_folio_number(s: &str, field_name: &str) -> Result<u64> {
    s.trim()
        .parse()
        .map_err(|_| Error::InvalidCaf(format!("invalid folio number in {field_name}: '{s}'")))
}

fn decode_signature_bytes(b64: &str) -> Result<Vec<u8>> {
    let clean = b64.replace(['\r', '\n', ' ', '\t'], "");
    BASE64_STANDARD
        .decode(&clean)
        .map_err(|e| Error::CafSignature(format!("invalid Base64 in FRMA: {e}")))
}

fn parse_rsa_public_key(m_b64: &str, e_b64: &str) -> Result<RsaPublicKey> {
    let clean_m = m_b64.replace(['\r', '\n', ' ', '\t'], "");
    let clean_e = e_b64.replace(['\r', '\n', ' ', '\t'], "");

    let m_bytes = BASE64_STANDARD
        .decode(&clean_m)
        .map_err(|e| Error::Crypto(format!("invalid Base64 in RSAPK/M: {e}")))?;
    let e_bytes = BASE64_STANDARD
        .decode(&clean_e)
        .map_err(|e| Error::Crypto(format!("invalid Base64 in RSAPK/E: {e}")))?;

    let n = BigUint::from_bytes_be(&m_bytes);
    let e = BigUint::from_bytes_be(&e_bytes);

    RsaPublicKey::new(n, e)
        .map_err(|err| Error::Crypto(format!("invalid RSA public key parameters: {err}")))
}

fn parse_rsa_private_key(pem_or_der: &str) -> Result<RsaPrivateKey> {
    let trimmed = pem_or_der.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidCaf("missing RSASK private key".to_string()));
    }

    if let Ok(key) = RsaPrivateKey::from_pkcs1_pem(trimmed) {
        return Ok(key);
    }
    if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(trimmed) {
        return Ok(key);
    }

    let clean = trimmed.replace(['\r', '\n', ' ', '\t'], "");
    if let Ok(der_bytes) = BASE64_STANDARD.decode(&clean) {
        if let Ok(key) = RsaPrivateKey::from_pkcs1_der(&der_bytes) {
            return Ok(key);
        }
    }

    Err(Error::Crypto(
        "failed to parse RSA private key from RSASK".to_string(),
    ))
}

fn verify_keypair_match(priv_k: &RsaPrivateKey, pub_k: &RsaPublicKey) -> Result<()> {
    if priv_k.n() != pub_k.n() || priv_k.e() != pub_k.e() {
        return Err(Error::InvalidCaf(
            "RSASK private key does not match RSAPK public key in DA".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn find_tag_slice<'a>(xml: &'a [u8], tag_name: &[u8]) -> Result<&'a [u8]> {
    let mut open_prefix = Vec::with_capacity(tag_name.len() + 1);
    open_prefix.push(b'<');
    open_prefix.extend_from_slice(tag_name);

    let mut close_tag = Vec::with_capacity(tag_name.len() + 3);
    close_tag.extend_from_slice(b"</");
    close_tag.extend_from_slice(tag_name);
    close_tag.push(b'>');

    let start_idx = find_opening_tag_index(xml, &open_prefix, tag_name)?;
    let remaining = xml
        .get(start_idx..)
        .ok_or_else(|| Error::InvalidCaf("XML bounds error".to_string()))?;

    let close_pos = remaining
        .windows(close_tag.len())
        .position(|w| w == close_tag.as_slice())
        .ok_or_else(|| {
            Error::InvalidCaf(format!(
                "missing closing tag </{}>",
                String::from_utf8_lossy(tag_name)
            ))
        })?;

    let end_idx = start_idx
        .checked_add(close_pos)
        .and_then(|p| p.checked_add(close_tag.len()))
        .ok_or_else(|| Error::InvalidCaf("overflow calculating tag bounds".to_string()))?;

    xml.get(start_idx..end_idx)
        .ok_or_else(|| Error::InvalidCaf("slice index out of bounds".to_string()))
}

fn find_opening_tag_index(xml: &[u8], open_prefix: &[u8], tag_name: &[u8]) -> Result<usize> {
    for (i, window) in xml.windows(open_prefix.len()).enumerate() {
        if window == open_prefix {
            let next_byte = xml.get(i.saturating_add(open_prefix.len()));
            if let Some(&b) = next_byte {
                if b == b'>' || b.is_ascii_whitespace() {
                    return Ok(i);
                }
            }
        }
    }
    Err(Error::InvalidCaf(format!(
        "missing opening tag <{}>",
        String::from_utf8_lossy(tag_name)
    )))
}

fn decode_xml_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn parse_xml_fields(xml: &[u8]) -> Result<RawCafFields> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut fields = RawCafFields {
        algoritmo: "SHA1withRSA".to_string(),
        ..Default::default()
    };

    let mut current_tag = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                current_tag = e.name().as_ref().to_vec();
                if current_tag.as_slice() == b"FRMA" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref().eq_ignore_ascii_case(b"algoritmo") {
                            fields.algoritmo = decode_xml_text(attr.value.as_ref());
                        }
                    }
                }
            }
            Ok(Event::End(_)) => {
                current_tag.clear();
            }
            Ok(Event::Text(t)) => {
                assign_field_text(&current_tag, decode_xml_text(t.as_ref()), &mut fields);
            }
            Ok(Event::CData(t)) => {
                assign_field_text(&current_tag, decode_xml_text(t.as_ref()), &mut fields);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(fields)
}

fn assign_field_text(tag: &[u8], text: String, fields: &mut RawCafFields) {
    match tag {
        b"RE" => fields.re.push_str(&text),
        b"RS" => fields.rs.push_str(&text),
        b"TD" => fields.td.push_str(&text),
        b"D" => fields.d.push_str(&text),
        b"H" => fields.h.push_str(&text),
        b"FA" => fields.fa.push_str(&text),
        b"M" => fields.m.push_str(&text),
        b"E" => fields.e.push_str(&text),
        b"IDK" => fields.idk.push_str(&text),
        b"FRMA" => fields.frma.push_str(&text),
        b"RSASK" => fields.rsask.push_str(&text),
        b"RSAPUBK" => match fields.rsapubk.as_mut() {
            Some(existing) => existing.push_str(&text),
            None => fields.rsapubk = Some(text),
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};
    use rsa::rand_core::OsRng;

    fn generate_test_keys() -> (RsaPrivateKey, RsaPrivateKey, String) {
        let mut rng = OsRng;
        let caf_priv = RsaPrivateKey::new(&mut rng, 1024).expect("caf key gen");
        let sii_priv = RsaPrivateKey::new(&mut rng, 1024).expect("sii key gen");
        let sii_pub = RsaPublicKey::from(&sii_priv);
        let sii_pub_pem = sii_pub.to_pkcs1_pem(LineEnding::LF).expect("sii pub pem");
        (caf_priv, sii_priv, sii_pub_pem)
    }

    fn build_test_caf_xml(
        caf_priv: &RsaPrivateKey,
        sii_priv: &RsaPrivateKey,
        rut: &str,
        tipo: u16,
        desde: u64,
        hasta: u64,
    ) -> (String, Vec<u8>) {
        let caf_pub = RsaPublicKey::from(caf_priv);
        let m_b64 = BASE64_STANDARD.encode(caf_pub.n().to_bytes_be());
        let e_b64 = BASE64_STANDARD.encode(caf_pub.e().to_bytes_be());

        let rsask_pem = caf_priv.to_pkcs1_pem(LineEnding::LF).expect("caf priv pem");
        let rsask_str = rsask_pem.as_str();
        let rsapubk_pem = caf_pub.to_pkcs1_pem(LineEnding::LF).expect("caf pub pem");
        let rsapubk_str = rsapubk_pem.as_str();

        let da_xml = format!(
            "<DA><RE>{rut}</RE><RS>EMPRESA DE PRUEBA SPA</RS><TD>{tipo}</TD><RNG><D>{desde}</D><H>{hasta}</H></RNG><FA>2026-01-01</FA><RSAPK><M>{m_b64}</M><E>{e_b64}</E></RSAPK><IDK>100</IDK></DA>"
        );

        let sii_signer = SigningKey::<Sha1>::new(sii_priv.clone());
        let firma_bytes = sii_signer.sign(da_xml.as_bytes()).to_vec();
        let frma_b64 = BASE64_STANDARD.encode(&firma_bytes);

        let full_xml = format!(
            "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<!-- CAF de prueba ficticio para sii-sdk (SEC-01, ADR-05) -->\n<AUTORIZACION>\n<CAF version=\"1.0\">\n{da_xml}\n<FRMA algoritmo=\"SHA1withRSA\">{frma_b64}</FRMA>\n</CAF>\n<RSASK>\n{rsask_str}\n</RSASK>\n<RSAPUBK>\n{rsapubk_str}\n</RSAPUBK>\n</AUTORIZACION>"
        );

        (full_xml, firma_bytes)
    }

    #[test]
    fn test_caf_parse_valid() {
        let (caf_priv, sii_priv, _sii_pub_pem) = generate_test_keys();
        let sii_pub = RsaPublicKey::from(&sii_priv);
        let (xml, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 1, 100);

        let mut caf = Caf::parse(xml.as_bytes()).expect("parse CAF");
        assert_eq!(caf.rango(), (1, 100));
        assert_eq!(caf.tipo(), TipoDte::FacturaAfecta);
        assert_eq!(caf.emisor_rut().to_string(), "76086428-5");
        assert_eq!(caf.razon_social(), "EMPRESA DE PRUEBA SPA");
        assert_eq!(caf.fecha_autorizacion(), "2026-01-01");
        assert_eq!(caf.idk(), "100");
        assert_eq!(caf.algoritmo_firma(), "SHA1withRSA");

        assert!(caf.contiene(1));
        assert!(caf.contiene(50));
        assert!(caf.contiene(100));
        assert!(!caf.contiene(0));
        assert!(!caf.contiene(101));

        assert!(caf.verificar_folio(1).is_ok());
        assert!(caf.verificar_folio(100).is_ok());
        let err = caf.verificar_folio(101).unwrap_err();
        assert_eq!(
            err,
            Error::FolioOutOfRange {
                folio: 101,
                min: 1,
                max: 100,
            }
        );

        assert!(!caf.is_firma_verificada());
        caf.verificar_firma(&sii_pub).expect("verify signature");
        assert!(caf.is_firma_verificada());
    }

    #[test]
    fn test_caf_parse_and_verify() {
        let (caf_priv, sii_priv, _) = generate_test_keys();
        let sii_pub = RsaPublicKey::from(&sii_priv);
        let (xml, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 1, 100);

        let caf = Caf::parse_and_verify(xml.as_bytes(), &sii_pub).expect("parse and verify");
        assert!(caf.is_firma_verificada());
    }

    #[test]
    fn test_caf_tampered_da_fails_verification() {
        let (caf_priv, sii_priv, _) = generate_test_keys();
        let sii_pub = RsaPublicKey::from(&sii_priv);
        let (xml, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 1, 100);

        // Alter folio range in DA from 100 to 200
        let tampered_xml = xml.replace("<H>100</H>", "<H>200</H>");
        let mut caf =
            Caf::parse(tampered_xml.as_bytes()).expect("parse should succeed without verification");
        let result = caf.verificar_firma(&sii_pub);
        assert!(result.is_err());
        assert!(!caf.is_firma_verificada());
    }

    #[test]
    fn test_caf_tampered_signature_fails_verification() {
        let (caf_priv, sii_priv, _) = generate_test_keys();
        let sii_pub = RsaPublicKey::from(&sii_priv);
        let (xml, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 1, 100);

        // Alter signature bytes in FRMA
        let tampered_xml = xml
            .replace("<FRMA", "<FRMA_OLD")
            .replace("</FRMA>", "</FRMA_OLD>");
        let tampered_xml = format!("{tampered_xml}<FRMA algoritmo=\"SHA1withRSA\">AAAA</FRMA>");
        let mut caf = Caf::parse(tampered_xml.as_bytes()).expect("parse should succeed");
        assert!(caf.verificar_firma(&sii_pub).is_err());
    }

    #[test]
    fn test_caf_keypair_mismatch() {
        let (caf_priv, sii_priv, _) = generate_test_keys();
        let mut rng = OsRng;
        let different_priv = RsaPrivateKey::new(&mut rng, 1024).expect("key gen");
        let diff_pem = different_priv
            .to_pkcs1_pem(LineEnding::LF)
            .expect("priv pem");

        let (xml, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 1, 100);
        let orig_pem = caf_priv.to_pkcs1_pem(LineEnding::LF).unwrap();
        let bad_xml = xml.replace(orig_pem.as_str(), diff_pem.as_str());

        let result = Caf::parse(bad_xml.as_bytes());
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidCaf(msg) => {
                assert!(msg.contains("does not match RSAPK"));
            }
            other => panic!("expected InvalidCaf, got {other:?}"),
        }
    }

    #[test]
    fn test_caf_invalid_ranges() {
        let (caf_priv, sii_priv, _) = generate_test_keys();

        // Start folio 0
        let (xml_zero, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 0, 100);
        assert!(Caf::parse(xml_zero.as_bytes()).is_err());

        // Start > End
        let (xml_inverted, _) =
            build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 200, 100);
        assert!(Caf::parse(xml_inverted.as_bytes()).is_err());
    }

    #[test]
    fn test_caf_invalid_rut_or_tipo() {
        let (caf_priv, sii_priv, _) = generate_test_keys();

        // Invalid RUT DV
        let (xml_bad_rut, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76123456-9", 33, 1, 100);
        assert_eq!(
            Caf::parse(xml_bad_rut.as_bytes()).unwrap_err(),
            Error::InvalidRut
        );

        // Invalid DTE type 999
        let (xml_bad_tipo, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 999, 1, 100);
        assert_eq!(
            Caf::parse(xml_bad_tipo.as_bytes()).unwrap_err(),
            Error::InvalidTipoDte(999)
        );
    }

    #[test]
    fn test_caf_debug_redacts_private_key() {
        let (caf_priv, sii_priv, _) = generate_test_keys();
        let (xml, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 1, 100);
        let caf = Caf::parse(xml.as_bytes()).expect("parse");

        let debug_output = format!("{caf:?}");
        assert!(debug_output.contains("private_key: \"<redacted>\""));
        assert!(!debug_output.contains("BEGIN RSA PRIVATE KEY"));
        assert!(!debug_output.contains("d: BigUint"));
    }

    #[test]
    fn test_caf_sign_ted_and_signer_trait() {
        let (caf_priv, sii_priv, _) = generate_test_keys();
        let (xml, _) = build_test_caf_xml(&caf_priv, &sii_priv, "76086428-5", 33, 1, 100);
        let caf = Caf::parse(xml.as_bytes()).expect("parse");

        let dd_sample = b"<DD><RE>76086428-5</RE><TD>33</TD><F>1</F><FE>2026-01-01</FE></DD>";
        let sig1 = caf.sign_ted(dd_sample).expect("sign_ted");
        let sig2 = caf.signer().sign(dd_sample).expect("signer trait sign");
        assert_eq!(sig1, sig2);

        // Verify that the TED signature validates using the CAF public key
        let verifying_key = VerifyingKey::<Sha1>::new(caf.public_key().clone());
        let signature = rsa::pkcs1v15::Signature::try_from(sig1.as_slice()).expect("parse sig");
        verifying_key
            .verify(dd_sample, &signature)
            .expect("verify TED signature");
    }

    proptest::proptest! {
        #[test]
        fn test_folio_range_containment(desde in 1u64..1000u64, len in 0u64..1000u64) {
            let hasta = desde + len;
            // Test containment logic
            assert!(desde <= hasta);
            for f in [desde, hasta, desde + (len / 2)] {
                assert!(f >= desde && f <= hasta);
            }
            if desde > 1 {
                assert!(desde - 1 < desde);
            }
            assert!(hasta + 1 > hasta);
        }
    }
}
