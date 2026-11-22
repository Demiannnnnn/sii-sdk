//! Electronic tax stamp (`TED`), `DD` structure, RSA-SHA1 signing, and `PDF417` data.
//!
//! Under Chilean SII specifications (`docs/dte-spec.md` §4, `constraints.md` TED-01, TED-02),
//! every Electronic Tax Document (DTE) must include an electronic tax stamp (`<TED version="1.0">`).
//!
//! The stamp consists of:
//! - `<DD>`: Canonical data payload summarizing the document (issuer, type, folio, date,
//!   receiver, amount, first item description, CAF authorization block, and stamp timestamp).
//! - `<FRMT>`: Digital signature generated with RSA-SHA1 over the canonical `<DD>` bytes
//!   using the RSA private key authorized in the CAF (`RSASK`).
//!
//! # PDF417 Data (ADR-06, TED-02)
//!
//! The 2D barcode (PDF417) printed on physical documents encodes the exact serialized `<TED>`
//! XML element. In accordance with ADR-06, this SDK provides the serialized bytes via
//! [`Ted::pdf417_data`]; bitmap rendering is left to the host application.

use crate::caf::{Caf, find_tag_slice};
use crate::codes::TipoDte;
use crate::dte::{Dte, format_date, parse_date};
use crate::error::{Error, Result};
use crate::types::{Monto, Rut};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::VerifyingKey;
use rsa::signature::Verifier as _;
use sha1::Sha1;

/// Truncates a string to at most `max_chars` Unicode characters, removing any trailing whitespace.
#[must_use]
pub fn clean_truncate(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        trimmed
            .chars()
            .take(max_chars)
            .collect::<String>()
            .trim_end()
            .to_string()
    }
}

/// Appends XML-escaped text bytes in ISO-8859-1 compatible representation.
fn push_escaped_xml(buf: &mut Vec<u8>, text: &str) {
    let escaped = quick_xml::escape::escape(text);
    for c in escaped.chars() {
        if (c as u32) <= 255 {
            let b = u8::try_from(c as u32).unwrap_or(b'?');
            buf.push(b);
        } else {
            let mut b = [0u8; 4];
            let s = c.encode_utf8(&mut b);
            buf.extend_from_slice(s.as_bytes());
        }
    }
}

/// Electronic tax stamp data payload (`<DD>` in SII schema).
///
/// Contains the summarized document attributes, the authorized `<CAF>` XML block,
/// and the stamp generation timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dd {
    /// Taxpayer (issuer) RUT (`RE`).
    pub re: Rut,
    /// DTE document type code (`TD`).
    pub td: TipoDte,
    /// Authorized folio number (`F`).
    pub f: u64,
    /// Document issue date (`FE`).
    pub fe: time::Date,
    /// Receiver tax ID (`RR`).
    pub rr: Rut,
    /// Receiver business name (`RSR`), truncated to at most 40 characters without trailing whitespace.
    pub rsr: String,
    /// Total monetary amount (`MNT`).
    pub mnt: Monto,
    /// Description of the first line item (`IT1`), truncated to at most 40 characters without trailing whitespace.
    pub it1: String,
    /// Raw bytes of the `<CAF version="1.0">...</CAF>` block.
    pub caf_xml: Vec<u8>,
    /// Electronic stamp generation timestamp in `YYYY-MM-DDTHH:MM:SS` format (`TSTED`).
    pub tsted: String,
}

impl Dd {
    /// Creates a builder for constructing `Dd` payloads.
    #[must_use]
    pub fn builder() -> DdBuilder {
        DdBuilder::new()
    }

    /// Extracts the `Dd` data payload from a `Dte` and `Caf` for the specified folio and timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if the DTE contains no line items.
    pub fn from_dte(
        dte: &Dte,
        caf: &Caf,
        folio: u64,
        tsted: &time::OffsetDateTime,
    ) -> Result<Self> {
        let first_item = dte
            .detalles()
            .first()
            .map(|d| clean_truncate(&d.nmb_item, 40))
            .ok_or_else(|| {
                Error::DteValidation("at least one detail line is required to stamp DTE".into())
            })?;

        let formatted_tsted = crate::types::format_timestamp(tsted);

        Ok(Self {
            re: dte.encabezado().emisor.rut_emisor,
            td: dte.tipo(),
            f: folio,
            fe: dte.encabezado().id_doc.fch_emis,
            rr: dte.encabezado().receptor.rut_recep,
            rsr: clean_truncate(&dte.encabezado().receptor.rzn_soc_recep, 40),
            mnt: dte.total(),
            it1: first_item,
            caf_xml: caf.raw_xml().to_vec(),
            tsted: formatted_tsted,
        })
    }

    /// Serializes the `Dd` structure into its exact canonical XML representation.
    ///
    /// Strictly adheres to the SII standard without whitespace between tags:
    /// `<DD><RE>...</RE><TD>...</TD><F>...</F><FE>...</FE><RR>...</RR><RSR>...</RSR><MNT>...</MNT><IT1>...</IT1><CAF version="1.0">...</CAF><TSTED>...</TSTED></DD>`
    #[must_use]
    pub fn to_canonical_xml(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(512 + self.caf_xml.len());
        buf.extend_from_slice(b"<DD>");

        buf.extend_from_slice(b"<RE>");
        buf.extend_from_slice(self.re.to_string().as_bytes());
        buf.extend_from_slice(b"</RE>");

        buf.extend_from_slice(b"<TD>");
        buf.extend_from_slice(self.td.code().to_string().as_bytes());
        buf.extend_from_slice(b"</TD>");

        buf.extend_from_slice(b"<F>");
        buf.extend_from_slice(self.f.to_string().as_bytes());
        buf.extend_from_slice(b"</F>");

        buf.extend_from_slice(b"<FE>");
        buf.extend_from_slice(format_date(&self.fe).as_bytes());
        buf.extend_from_slice(b"</FE>");

        buf.extend_from_slice(b"<RR>");
        buf.extend_from_slice(self.rr.to_string().as_bytes());
        buf.extend_from_slice(b"</RR>");

        buf.extend_from_slice(b"<RSR>");
        push_escaped_xml(&mut buf, &self.rsr);
        buf.extend_from_slice(b"</RSR>");

        buf.extend_from_slice(b"<MNT>");
        buf.extend_from_slice(self.mnt.to_string().as_bytes());
        buf.extend_from_slice(b"</MNT>");

        buf.extend_from_slice(b"<IT1>");
        push_escaped_xml(&mut buf, &self.it1);
        buf.extend_from_slice(b"</IT1>");

        buf.extend_from_slice(&self.caf_xml);

        buf.extend_from_slice(b"<TSTED>");
        buf.extend_from_slice(self.tsted.as_bytes());
        buf.extend_from_slice(b"</TSTED>");

        buf.extend_from_slice(b"</DD>");
        buf
    }

    /// Returns the issuer RUT (`RE`).
    #[inline]
    #[must_use]
    pub const fn re(&self) -> &Rut {
        &self.re
    }

    /// Returns the DTE document type (`TD`).
    #[inline]
    #[must_use]
    pub const fn td(&self) -> TipoDte {
        self.td
    }

    /// Returns the authorized folio number (`F`).
    #[inline]
    #[must_use]
    pub const fn f(&self) -> u64 {
        self.f
    }

    /// Returns the document emission date (`FE`).
    #[inline]
    #[must_use]
    pub const fn fe(&self) -> &time::Date {
        &self.fe
    }

    /// Returns the receiver RUT (`RR`).
    #[inline]
    #[must_use]
    pub const fn rr(&self) -> &Rut {
        &self.rr
    }

    /// Returns the receiver business name (`RSR`).
    #[inline]
    #[must_use]
    pub fn rsr(&self) -> &str {
        &self.rsr
    }

    /// Returns the total monetary amount (`MNT`).
    #[inline]
    #[must_use]
    pub const fn mnt(&self) -> Monto {
        self.mnt
    }

    /// Returns the first item description (`IT1`).
    #[inline]
    #[must_use]
    pub fn it1(&self) -> &str {
        &self.it1
    }

    /// Returns the raw `<CAF>` bytes.
    #[inline]
    #[must_use]
    pub fn caf_xml(&self) -> &[u8] {
        &self.caf_xml
    }

    /// Returns the stamp timestamp string (`TSTED`).
    #[inline]
    #[must_use]
    pub fn tsted(&self) -> &str {
        &self.tsted
    }
}

/// Builder for constructing [`Dd`] payloads manually.
#[derive(Debug, Clone, Default)]
pub struct DdBuilder {
    re: Option<Rut>,
    td: Option<TipoDte>,
    f: Option<u64>,
    fe: Option<time::Date>,
    rr: Option<Rut>,
    rsr: Option<String>,
    mnt: Option<Monto>,
    it1: Option<String>,
    caf_xml: Option<Vec<u8>>,
    tsted: Option<String>,
}

impl DdBuilder {
    /// Creates a new `DdBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the issuer RUT (`RE`).
    #[must_use]
    pub const fn re(mut self, re: Rut) -> Self {
        self.re = Some(re);
        self
    }

    /// Sets the document type (`TD`).
    #[must_use]
    pub const fn td(mut self, td: TipoDte) -> Self {
        self.td = Some(td);
        self
    }

    /// Sets the folio (`F`).
    #[must_use]
    pub const fn f(mut self, f: u64) -> Self {
        self.f = Some(f);
        self
    }

    /// Sets the emission date (`FE`).
    #[must_use]
    pub const fn fe(mut self, fe: time::Date) -> Self {
        self.fe = Some(fe);
        self
    }

    /// Sets the receiver RUT (`RR`).
    #[must_use]
    pub const fn rr(mut self, rr: Rut) -> Self {
        self.rr = Some(rr);
        self
    }

    /// Sets the receiver business name (`RSR`).
    #[must_use]
    pub fn rsr(mut self, rsr: impl AsRef<str>) -> Self {
        self.rsr = Some(clean_truncate(rsr.as_ref(), 40));
        self
    }

    /// Sets the total amount (`MNT`).
    #[must_use]
    pub const fn mnt(mut self, mnt: Monto) -> Self {
        self.mnt = Some(mnt);
        self
    }

    /// Sets the first item description (`IT1`).
    #[must_use]
    pub fn it1(mut self, it1: impl AsRef<str>) -> Self {
        self.it1 = Some(clean_truncate(it1.as_ref(), 40));
        self
    }

    /// Sets the raw CAF XML block.
    #[must_use]
    pub fn caf_xml(mut self, caf_xml: Vec<u8>) -> Self {
        self.caf_xml = Some(caf_xml);
        self
    }

    /// Sets the stamp timestamp (`TSTED`).
    #[must_use]
    pub fn tsted(mut self, tsted: impl Into<String>) -> Self {
        self.tsted = Some(tsted.into());
        self
    }

    /// Builds the `Dd` payload.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if required fields are missing.
    pub fn build(self) -> Result<Dd> {
        Ok(Dd {
            re: self
                .re
                .ok_or_else(|| Error::DteValidation("missing RE (issuer RUT)".into()))?,
            td: self
                .td
                .ok_or_else(|| Error::DteValidation("missing TD (document type)".into()))?,
            f: self
                .f
                .ok_or_else(|| Error::DteValidation("missing F (folio)".into()))?,
            fe: self
                .fe
                .ok_or_else(|| Error::DteValidation("missing FE (issue date)".into()))?,
            rr: self
                .rr
                .ok_or_else(|| Error::DteValidation("missing RR (receiver RUT)".into()))?,
            rsr: self.rsr.unwrap_or_default(),
            mnt: self
                .mnt
                .ok_or_else(|| Error::DteValidation("missing MNT (total amount)".into()))?,
            it1: self.it1.unwrap_or_default(),
            caf_xml: self
                .caf_xml
                .ok_or_else(|| Error::DteValidation("missing CAF XML block".into()))?,
            tsted: self
                .tsted
                .ok_or_else(|| Error::DteValidation("missing TSTED (timestamp)".into()))?,
        })
    }
}

/// Electronic Tax Stamp (`<TED version="1.0">`).
///
/// Encapsulates the canonical `<DD>` payload and the digital signature (`<FRMT>`)
/// created using RSA-SHA1 with the CAF private key.
#[derive(Clone, PartialEq, Eq)]
pub struct Ted {
    dd: Dd,
    raw_dd: Vec<u8>,
    frmt: String,
    raw_xml: Vec<u8>,
}

impl Ted {
    /// Creates a new electronic tax stamp by signing the given `Dd` payload with the CAF private key.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if digital signing fails.
    pub fn new(dd: Dd, caf: &Caf) -> Result<Self> {
        let raw_dd = dd.to_canonical_xml();
        let sig_bytes = caf.sign_ted(&raw_dd)?;
        let frmt = BASE64_STANDARD.encode(&sig_bytes);

        let mut raw_xml = Vec::with_capacity(raw_dd.len() + frmt.len() + 64);
        raw_xml.extend_from_slice(b"<TED version=\"1.0\">");
        raw_xml.extend_from_slice(&raw_dd);
        raw_xml.extend_from_slice(b"<FRMT algoritmo=\"SHA1withRSA\">");
        raw_xml.extend_from_slice(frmt.as_bytes());
        raw_xml.extend_from_slice(b"</FRMT></TED>");

        Ok(Self {
            dd,
            raw_dd,
            frmt,
            raw_xml,
        })
    }

    /// Parses a `<TED version="1.0">` XML block from raw bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if XML is malformed or required tags are missing.
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let raw_xml = find_tag_slice(xml, b"TED")?.to_vec();
        let raw_dd = find_tag_slice(xml, b"DD")?.to_vec();
        let raw_caf = find_tag_slice(&raw_dd, b"CAF")?.to_vec();

        let parsed = parse_ted_fields(xml)?;

        let re = Rut::parse(&parsed.re)?;
        let td_num: u16 = parsed
            .td
            .trim()
            .parse()
            .map_err(|_| Error::InvalidTipoDte(0))?;
        let td = TipoDte::try_from(td_num)?;
        let f: u64 =
            parsed.f.trim().parse().map_err(|_| {
                Error::DteValidation(format!("invalid folio in TED: '{}'", parsed.f))
            })?;
        let fe = parse_date(&parsed.fe)?;
        let rr = Rut::parse(&parsed.rr)?;
        let mnt = Monto::parse(&parsed.mnt)?;

        let dd = Dd {
            re,
            td,
            f,
            fe,
            rr,
            rsr: parsed.rsr,
            mnt,
            it1: parsed.it1,
            caf_xml: raw_caf,
            tsted: parsed.tsted,
        };

        Ok(Self {
            dd,
            raw_dd,
            frmt: parsed.frmt,
            raw_xml,
        })
    }

    /// Returns the structured `Dd` data payload.
    #[inline]
    #[must_use]
    pub const fn dd(&self) -> &Dd {
        &self.dd
    }

    /// Returns the exact bytes of the `<DD>...</DD>` block that was signed.
    #[inline]
    #[must_use]
    pub fn raw_dd(&self) -> &[u8] {
        &self.raw_dd
    }

    /// Returns the Base64-encoded RSA-SHA1 signature string (`FRMT`).
    #[inline]
    #[must_use]
    pub fn frmt(&self) -> &str {
        &self.frmt
    }

    /// Returns the serialized `<TED version="1.0">...</TED>` XML bytes.
    #[inline]
    #[must_use]
    pub fn raw_ted(&self) -> &[u8] {
        &self.raw_xml
    }

    /// Returns the exact serialized `<TED>` data required for generating the PDF417 barcode (ADR-06, TED-02).
    #[inline]
    #[must_use]
    pub fn pdf417_data(&self) -> &[u8] {
        &self.raw_xml
    }

    /// Verifies the digital signature (`<FRMT>`) against `<DD>` using the authorized CAF public key.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Crypto`] if Base64 decoding fails or the digital signature does not match.
    pub fn verificar_firma(&self, caf_public_key: &RsaPublicKey) -> Result<()> {
        let clean_frmt = self.frmt.replace(['\r', '\n', ' ', '\t'], "");
        let sig_bytes = BASE64_STANDARD
            .decode(&clean_frmt)
            .map_err(|e| Error::Crypto(format!("invalid Base64 in FRMT: {e}")))?;

        let signature = rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice())
            .map_err(|e| Error::Crypto(format!("invalid signature length or format: {e}")))?;

        let verifying_key = VerifyingKey::<Sha1>::new(caf_public_key.clone());
        verifying_key
            .verify(&self.raw_dd, &signature)
            .map_err(|e| {
                Error::Crypto(format!(
                    "TED FRMT digital signature verification failed: {e}"
                ))
            })?;

        Ok(())
    }
}

impl std::fmt::Debug for Ted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ted")
            .field("dd", &self.dd)
            .field("frmt", &self.frmt)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct ParsedTedFields {
    re: String,
    td: String,
    f: String,
    fe: String,
    rr: String,
    rsr: String,
    mnt: String,
    it1: String,
    tsted: String,
    frmt: String,
}

fn parse_ted_fields(xml: &[u8]) -> Result<ParsedTedFields> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut fields = ParsedTedFields::default();
    let mut current_tag = Vec::new();
    let mut inside_caf = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_vec();
                if name.as_slice() == b"CAF" {
                    inside_caf = true;
                }
                current_tag = name;
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"CAF" {
                    inside_caf = false;
                }
                current_tag.clear();
            }
            Ok(Event::Text(t)) => {
                if inside_caf {
                    continue;
                }
                let text = match std::str::from_utf8(t.as_ref()) {
                    Ok(s) => s.to_string(),
                    Err(_) => t.as_ref().iter().map(|&b| b as char).collect(),
                };
                match current_tag.as_slice() {
                    b"RE" => fields.re.push_str(&text),
                    b"TD" => fields.td.push_str(&text),
                    b"F" => fields.f.push_str(&text),
                    b"FE" => fields.fe.push_str(&text),
                    b"RR" => fields.rr.push_str(&text),
                    b"RSR" => fields.rsr.push_str(&text),
                    b"MNT" => fields.mnt.push_str(&text),
                    b"IT1" => fields.it1.push_str(&text),
                    b"TSTED" => fields.tsted.push_str(&text),
                    b"FRMT" => fields.frmt.push_str(&text),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Clock, ManualClock};
    use rsa::RsaPrivateKey;
    use rsa::pkcs1::{EncodeRsaPrivateKey, LineEnding};
    use rsa::rand_core::OsRng;
    use rsa::signature::{SignatureEncoding as _, Signer as _};
    use rsa::traits::PublicKeyParts;

    fn generate_test_caf() -> (Caf, RsaPrivateKey) {
        let mut rng = OsRng;
        let caf_priv = RsaPrivateKey::new(&mut rng, 1024).expect("key gen");
        let caf_pub = RsaPublicKey::from(&caf_priv);
        let m_b64 = BASE64_STANDARD.encode(caf_pub.n().to_bytes_be());
        let e_b64 = BASE64_STANDARD.encode(caf_pub.e().to_bytes_be());
        let rsask_pem = caf_priv.to_pkcs1_pem(LineEnding::LF).expect("pem");
        let rsask_str = rsask_pem.as_str();

        let da_xml = format!(
            "<DA><RE>76086428-5</RE><RS>EMPRESA DE PRUEBA SPA</RS><TD>33</TD><RNG><D>1</D><H>100</H></RNG><FA>2026-01-01</FA><RSAPK><M>{m_b64}</M><E>{e_b64}</E></RSAPK><IDK>100</IDK></DA>"
        );

        let sii_signer = rsa::pkcs1v15::SigningKey::<Sha1>::new(caf_priv.clone());
        let firma_bytes = sii_signer.sign(da_xml.as_bytes()).to_vec();
        let frma_b64 = BASE64_STANDARD.encode(&firma_bytes);

        let xml = format!(
            "<AUTORIZACION>\n<CAF version=\"1.0\">\n{da_xml}\n<FRMA algoritmo=\"SHA1withRSA\">{frma_b64}</FRMA>\n</CAF>\n<RSASK>\n{rsask_str}\n</RSASK>\n</AUTORIZACION>"
        );

        let caf = Caf::parse(xml.as_bytes()).expect("parse caf");
        (caf, caf_priv)
    }

    #[test]
    fn test_clean_truncate() {
        assert_eq!(clean_truncate("Short", 40), "Short");
        assert_eq!(clean_truncate("   Trim me   ", 40), "Trim me");
        let long = "1234567890123456789012345678901234567890EXTRA";
        assert_eq!(
            clean_truncate(long, 40),
            "1234567890123456789012345678901234567890"
        );
        let with_trailing_space = "123456789012345678901234567890123456789     EXTRA";
        assert_eq!(
            clean_truncate(with_trailing_space, 40),
            "123456789012345678901234567890123456789"
        );
    }

    #[test]
    fn test_dd_canonical_xml_format() {
        let (caf, _) = generate_test_caf();
        let fch_emis = time::Date::from_calendar_date(2026, time::Month::October, 21).unwrap();
        let dd = Dd::builder()
            .re(Rut::parse("76086428-5").unwrap())
            .td(TipoDte::FacturaAfecta)
            .f(60)
            .fe(fch_emis)
            .rr(Rut::parse("77777777-7").unwrap())
            .rsr("CLIENTE DE PRUEBAS SA")
            .mnt(Monto::from_pesos(119_000))
            .it1("Servidor de aplicaciones")
            .caf_xml(caf.raw_xml().to_vec())
            .tsted("2026-10-21T15:30:00")
            .build()
            .expect("build dd");

        let xml_bytes = dd.to_canonical_xml();
        let xml_str = String::from_utf8(xml_bytes).expect("valid utf-8");

        assert!(xml_str.starts_with("<DD><RE>76086428-5</RE><TD>33</TD><F>60</F>"));
        assert!(xml_str.contains("<FE>2026-10-21</FE>"));
        assert!(xml_str.contains("<RR>77777777-7</RR>"));
        assert!(xml_str.contains("<RSR>CLIENTE DE PRUEBAS SA</RSR>"));
        assert!(xml_str.contains("<MNT>119000</MNT>"));
        assert!(xml_str.contains("<IT1>Servidor de aplicaciones</IT1>"));
        assert!(xml_str.contains("<CAF version=\"1.0\">"));
        assert!(xml_str.ends_with("<TSTED>2026-10-21T15:30:00</TSTED></DD>"));
    }

    #[test]
    fn test_ted_signing_and_verification() {
        let (caf, _) = generate_test_caf();
        let fch_emis = time::Date::from_calendar_date(2026, time::Month::October, 21).unwrap();
        let dd = Dd::builder()
            .re(Rut::parse("76086428-5").unwrap())
            .td(TipoDte::FacturaAfecta)
            .f(60)
            .fe(fch_emis)
            .rr(Rut::parse("77777777-7").unwrap())
            .rsr("CLIENTE DE PRUEBAS SA")
            .mnt(Monto::from_pesos(119_000))
            .it1("Servidor de aplicaciones")
            .caf_xml(caf.raw_xml().to_vec())
            .tsted("2026-10-21T15:30:00")
            .build()
            .unwrap();

        let ted = Ted::new(dd, &caf).expect("create ted");
        assert_ne!(ted.frmt(), "");

        let raw_ted_str = String::from_utf8(ted.raw_ted().to_vec()).unwrap();
        assert!(raw_ted_str.starts_with("<TED version=\"1.0\"><DD>"));
        assert!(raw_ted_str.contains("<FRMT algoritmo=\"SHA1withRSA\">"));
        assert!(raw_ted_str.ends_with("</FRMT></TED>"));

        assert_eq!(ted.pdf417_data(), ted.raw_ted());

        // Digital signature verification using CAF public key
        ted.verificar_firma(caf.public_key())
            .expect("signature must verify");

        // Parse round-trip
        let parsed = Ted::parse(ted.raw_ted()).expect("parse ted");
        assert_eq!(parsed.dd().re(), ted.dd().re());
        assert_eq!(parsed.dd().f(), 60);
        assert_eq!(parsed.frmt(), ted.frmt());
        parsed
            .verificar_firma(caf.public_key())
            .expect("parsed ted must verify");
    }

    #[test]
    fn test_ted_tampered_fails_verification() {
        let (caf, _) = generate_test_caf();
        let fch_emis = time::Date::from_calendar_date(2026, time::Month::October, 21).unwrap();
        let dd = Dd::builder()
            .re(Rut::parse("76086428-5").unwrap())
            .td(TipoDte::FacturaAfecta)
            .f(60)
            .fe(fch_emis)
            .rr(Rut::parse("77777777-7").unwrap())
            .rsr("CLIENTE DE PRUEBAS SA")
            .mnt(Monto::from_pesos(119_000))
            .it1("Servidor de aplicaciones")
            .caf_xml(caf.raw_xml().to_vec())
            .tsted("2026-10-21T15:30:00")
            .build()
            .unwrap();

        let mut ted = Ted::new(dd, &caf).expect("create ted");

        // Tamper with DD: alter amount from 119000 to 219000
        let tampered_raw_dd = String::from_utf8(ted.raw_dd.clone())
            .unwrap()
            .replace("<MNT>119000</MNT>", "<MNT>219000</MNT>")
            .into_bytes();
        ted.raw_dd = tampered_raw_dd;

        let result = ted.verificar_firma(caf.public_key());
        assert!(result.is_err());
    }

    #[test]
    fn test_clock_injection_reproducible_ted() {
        let (caf, _) = generate_test_caf();
        let fch_emis = time::Date::from_calendar_date(2026, time::Month::October, 21).unwrap();
        let time = time::Time::from_hms(15, 30, 0).unwrap();
        let fixed_now = fch_emis.with_time(time).assume_utc();
        let clock = ManualClock::new(fixed_now);

        let dte = Dte::factura_afecta()
            .folio(60)
            .fch_emis(fch_emis)
            .emisor(crate::dte::Emisor::new(
                Rut::parse("76086428-5").unwrap(),
                "EMPRESA DE PRUEBA SPA",
                "GIRO",
                12345,
                "DIR",
                "CMNA",
            ))
            .receptor(crate::dte::Receptor::new(
                Rut::parse("77777777-7").unwrap(),
                "CLIENTE",
                "GIRO",
                "DIR",
                "CMNA",
            ))
            .item(crate::dte::Detalle::new(
                1,
                "Item",
                Monto::from_pesos(100_000),
            ))
            .build()
            .unwrap();

        let dd1 = Dd::from_dte(&dte, &caf, 60, &clock.now()).unwrap();
        let dd2 = Dd::from_dte(&dte, &caf, 60, &clock.now()).unwrap();

        assert_eq!(dd1.to_canonical_xml(), dd2.to_canonical_xml());

        let ted1 = Ted::new(dd1, &caf).unwrap();
        let ted2 = Ted::new(dd2, &caf).unwrap();

        assert_eq!(ted1.frmt(), ted2.frmt());
        assert_eq!(ted1.raw_ted(), ted2.raw_ted());
    }
}
