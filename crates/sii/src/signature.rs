//! `XMLDSig` digital signatures, C14N canonicalization, and `Signer`/`Certificate` traits.
//!
//! Implements W3C XML Digital Signature standard compliant with Chilean SII
//! requirements (`docs/dte-spec.md` §6, `constraints.md` SIG-01..03, ADR-02).
//!
//! The digital signature is applied as an **enveloped** `XMLDSig` signature targeting
//! `<Documento ID="...">` or similar payload blocks, normalized with Canonical XML 1.0 (C14N).

use crate::codes::Ambiente;
use crate::error::{Error, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use der::Decode;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs1v15::VerifyingKey;
use rsa::signature::Verifier as _;
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, RsaPublicKey};
use sha1::Sha1;
use sha2::Sha256;
use std::fmt;
use x509_cert::Certificate as X509Cert;

/// Trait for signing data using a private key (ADR-02, SIG-01).
///
/// Implemented by cryptographic key holders such as CAF private keys and issuer certificates.
pub trait Signer: Send + Sync {
    /// Signs input data bytes and returns the resulting digital signature bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the cryptographic signing operation fails.
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;
}

/// Trait for exposing an X.509 digital certificate used in `XMLDSig` signatures (RF-01, ADR-02).
pub trait Certificate: Send + Sync {
    /// Returns the raw DER bytes of the X.509 certificate.
    fn raw_der(&self) -> &[u8];

    /// Returns the public key extracted from the certificate.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the public key cannot be extracted from the certificate.
    fn public_key(&self) -> Result<RsaPublicKey> {
        extract_rsa_public_key_from_der(self.raw_der())
    }
}

/// In-memory X.509 certificate holder implementing [`Certificate`].
#[derive(Clone, PartialEq, Eq)]
pub struct X509Certificate {
    raw_der: Vec<u8>,
    public_key: RsaPublicKey,
}

impl X509Certificate {
    /// Parses an X.509 certificate from raw DER bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Crypto`] if parsing fails or public key cannot be extracted.
    pub fn from_der(der: &[u8]) -> Result<Self> {
        let public_key = extract_rsa_public_key_from_der(der)?;
        Ok(Self {
            raw_der: der.to_vec(),
            public_key,
        })
    }

    /// Parses an X.509 certificate from PEM formatted text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Crypto`] if PEM header/footer is missing or base64 is invalid.
    pub fn from_pem(pem: &str) -> Result<Self> {
        let der = decode_pem_body(pem, "CERTIFICATE")?;
        Self::from_der(&der)
    }

    /// Creates a certificate from existing raw DER and public key components.
    #[must_use]
    pub fn from_parts(raw_der: Vec<u8>, public_key: RsaPublicKey) -> Self {
        Self {
            raw_der,
            public_key,
        }
    }
}

impl Certificate for X509Certificate {
    fn raw_der(&self) -> &[u8] {
        &self.raw_der
    }

    fn public_key(&self) -> Result<RsaPublicKey> {
        Ok(self.public_key.clone())
    }
}

impl fmt::Debug for X509Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X509Certificate")
            .field("der_len", &self.raw_der.len())
            .field("key_bits", &self.public_key.size())
            .finish()
    }
}

/// Digest algorithm used in `XMLDSig` references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DigestAlgorithm {
    /// SHA-1 digest (`http://www.w3.org/2000/09/xmldsig#sha1`).
    #[default]
    Sha1,
    /// SHA-256 digest (`http://www.w3.org/2001/04/xmlenc#sha256`).
    Sha256,
}

impl DigestAlgorithm {
    /// Returns the W3C URI for this digest algorithm.
    #[must_use]
    pub const fn uri(&self) -> &'static str {
        match self {
            Self::Sha1 => "http://www.w3.org/2000/09/xmldsig#sha1",
            Self::Sha256 => "http://www.w3.org/2001/04/xmlenc#sha256",
        }
    }

    /// Computes the cryptographic digest of input bytes.
    #[must_use]
    pub fn digest(&self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha1 => {
                use sha1::Digest;
                Sha1::digest(data).to_vec()
            }
            Self::Sha256 => {
                use sha2::Digest;
                Sha256::digest(data).to_vec()
            }
        }
    }

    /// Parses a digest algorithm from its W3C URI.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Signature`] if the URI is not supported.
    pub fn from_uri(uri: &str) -> Result<Self> {
        match uri.trim() {
            "http://www.w3.org/2000/09/xmldsig#sha1" => Ok(Self::Sha1),
            "http://www.w3.org/2001/04/xmlenc#sha256" => Ok(Self::Sha256),
            other => Err(Error::Signature(format!(
                "unsupported digest algorithm URI: {other}"
            ))),
        }
    }
}

/// Signature algorithm used in `XMLDSig` `<SignedInfo>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignatureAlgorithm {
    /// RSA with SHA-1 (`http://www.w3.org/2000/09/xmldsig#rsa-sha1`).
    #[default]
    RsaSha1,
    /// RSA with SHA-256 (`http://www.w3.org/2001/04/xmldsig-more#rsa-sha256`).
    RsaSha256,
}

impl SignatureAlgorithm {
    /// Returns the W3C URI for this signature method.
    #[must_use]
    pub const fn uri(&self) -> &'static str {
        match self {
            Self::RsaSha1 => "http://www.w3.org/2000/09/xmldsig#rsa-sha1",
            Self::RsaSha256 => "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
        }
    }

    /// Returns the corresponding default digest algorithm.
    #[must_use]
    pub const fn digest_algorithm(&self) -> DigestAlgorithm {
        match self {
            Self::RsaSha1 => DigestAlgorithm::Sha1,
            Self::RsaSha256 => DigestAlgorithm::Sha256,
        }
    }

    /// Parses a signature algorithm from its W3C URI.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Signature`] if the URI is not supported.
    pub fn from_uri(uri: &str) -> Result<Self> {
        match uri.trim() {
            "http://www.w3.org/2000/09/xmldsig#rsa-sha1" => Ok(Self::RsaSha1),
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"
            | "http://www.w3.org/2000/09/xmldsig#rsa-sha256" => Ok(Self::RsaSha256),
            other => Err(Error::Signature(format!(
                "unsupported signature algorithm URI: {other}"
            ))),
        }
    }

    /// Selects the algorithm based on the Chilean SII environment (`Ambiente`).
    ///
    /// Standard SII schema `DTE_v10.xsd` historically mandates `RsaSha1` in both
    /// `Certificacion` and `Produccion`.
    #[must_use]
    pub const fn for_ambiente(ambiente: Ambiente) -> Self {
        match ambiente {
            Ambiente::Certificacion | Ambiente::Produccion => Self::RsaSha1,
        }
    }
}

/// Configuration options for signing XML documents with `XMLDSig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureOptions {
    /// Signature algorithm (RSA-SHA1 or RSA-SHA256).
    pub signature_algorithm: SignatureAlgorithm,
    /// Digest algorithm for the reference payload.
    pub digest_algorithm: DigestAlgorithm,
}

impl Default for SignatureOptions {
    fn default() -> Self {
        Self {
            signature_algorithm: SignatureAlgorithm::RsaSha1,
            digest_algorithm: DigestAlgorithm::Sha1,
        }
    }
}

impl SignatureOptions {
    /// Creates signature options suitable for the given Chilean SII environment.
    #[must_use]
    pub const fn for_ambiente(ambiente: Ambiente) -> Self {
        let signature_algorithm = SignatureAlgorithm::for_ambiente(ambiente);
        let digest_algorithm = signature_algorithm.digest_algorithm();
        Self {
            signature_algorithm,
            digest_algorithm,
        }
    }
}

/// Extracts an RSA public key from raw DER-encoded X.509 certificate bytes.
///
/// # Errors
///
/// Returns [`Error::Crypto`] if the certificate or RSA key cannot be parsed.
pub fn extract_rsa_public_key_from_der(der_bytes: &[u8]) -> Result<RsaPublicKey> {
    let cert = X509Cert::from_der(der_bytes)
        .map_err(|e| Error::Crypto(format!("failed to parse X.509 certificate: {e}")))?;
    let spki = cert.tbs_certificate.subject_public_key_info;
    let raw_spk = spki.subject_public_key.raw_bytes();
    RsaPublicKey::from_pkcs1_der(raw_spk).map_err(|e| {
        Error::Crypto(format!(
            "failed to parse RSA public key from certificate: {e}"
        ))
    })
}

/// Reconstructs an RSA public key from Base64-encoded modulus and exponent strings.
///
/// # Errors
///
/// Returns [`Error::Crypto`] if decoding or key construction fails.
pub fn rsa_public_key_from_components(
    modulus_b64: &str,
    exponent_b64: &str,
) -> Result<RsaPublicKey> {
    let clean_m: String = modulus_b64.chars().filter(|c| !c.is_whitespace()).collect();
    let clean_e: String = exponent_b64
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let m_bytes = BASE64_STANDARD
        .decode(&clean_m)
        .map_err(|e| Error::Crypto(format!("invalid base64 in RSA modulus: {e}")))?;
    let e_bytes = BASE64_STANDARD
        .decode(&clean_e)
        .map_err(|e| Error::Crypto(format!("invalid base64 in RSA exponent: {e}")))?;

    let n = BigUint::from_bytes_be(&m_bytes);
    let e = BigUint::from_bytes_be(&e_bytes);

    RsaPublicKey::new(n, e)
        .map_err(|err| Error::Crypto(format!("invalid RSA public key parameters: {err}")))
}

/// Canonicalizes an entire XML document using Canonical XML 1.0 (C14N without comments).
///
/// # Errors
///
/// Returns [`Error::Xml`] if parsing fails.
pub fn canonicalize(xml: &[u8]) -> Result<Vec<u8>> {
    canonicalize_node(xml, None, None)
}

/// Canonicalizes an XML element or subtree using Canonical XML 1.0 (C14N without comments).
///
/// If `target_id` is specified, locates the element having `ID="{target_id}"`.
/// If `in_scope_default_ns` is given, ensures the canonical root element declares it.
/// If an enveloped `<Signature>` is encountered within the target, it is omitted.
///
/// # Errors
///
/// Returns [`Error`] if XML parsing fails or target element cannot be found.
pub fn canonicalize_node(
    xml: &[u8],
    target_id: Option<&str>,
    in_scope_default_ns: Option<&str>,
) -> Result<Vec<u8>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut out = Vec::new();
    let mut buf = Vec::new();
    let mut capturing = target_id.is_none();
    let mut target_depth: usize = 0;
    let mut skip_sig_depth: usize = 0;
    let mut is_canonical_root = true;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                handle_start_event(
                    e,
                    target_id,
                    in_scope_default_ns,
                    &mut capturing,
                    &mut target_depth,
                    &mut skip_sig_depth,
                    &mut is_canonical_root,
                    &mut out,
                )?;
            }
            Ok(Event::Empty(ref e)) => {
                handle_empty_event(
                    e,
                    target_id,
                    in_scope_default_ns,
                    &mut capturing,
                    skip_sig_depth,
                    &mut is_canonical_root,
                    &mut out,
                )?;
                if target_id.is_some() && capturing && target_depth == 0 {
                    break;
                }
            }
            Ok(Event::End(ref e)) => {
                if !capturing {
                    continue;
                }
                if skip_sig_depth > 0 {
                    skip_sig_depth = skip_sig_depth.saturating_sub(1);
                    continue;
                }
                out.extend_from_slice(b"</");
                out.extend_from_slice(e.name().as_ref());
                out.push(b'>');
                target_depth = target_depth.saturating_sub(1);
                if target_depth == 0 && target_id.is_some() {
                    break;
                }
            }
            Ok(Event::Text(ref t)) => {
                if capturing && skip_sig_depth == 0 {
                    let unescaped = t.unescape().map_err(|e| Error::Xml(e.to_string()))?;
                    escape_c14n_text_into(&unescaped, &mut out);
                }
            }
            Ok(Event::CData(ref t)) => {
                if capturing && skip_sig_depth == 0 {
                    let text = String::from_utf8_lossy(t.as_ref());
                    escape_c14n_text_into(&text, &mut out);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(e.to_string())),
            _ => {}
        }
    }

    if target_id.is_some() && !capturing && out.is_empty() {
        return Err(Error::Signature(format!(
            "target element with ID '{target_id:?}' not found for canonicalization"
        )));
    }

    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn handle_start_event(
    e: &quick_xml::events::BytesStart<'_>,
    target_id: Option<&str>,
    in_scope_default_ns: Option<&str>,
    capturing: &mut bool,
    target_depth: &mut usize,
    skip_sig_depth: &mut usize,
    is_canonical_root: &mut bool,
    out: &mut Vec<u8>,
) -> Result<()> {
    if !*capturing {
        if is_target_match(e, target_id) {
            *capturing = true;
            *target_depth = 1;
            write_c14n_start_tag(e, *is_canonical_root, in_scope_default_ns, out)?;
            *is_canonical_root = false;
        }
        return Ok(());
    }

    if *skip_sig_depth > 0 {
        *skip_sig_depth += 1;
        return Ok(());
    }

    if is_signature_tag(e) {
        *skip_sig_depth = 1;
        return Ok(());
    }

    *target_depth += 1;
    write_c14n_start_tag(e, *is_canonical_root, in_scope_default_ns, out)?;
    *is_canonical_root = false;
    Ok(())
}

fn handle_empty_event(
    e: &quick_xml::events::BytesStart<'_>,
    target_id: Option<&str>,
    in_scope_default_ns: Option<&str>,
    capturing: &mut bool,
    skip_sig_depth: usize,
    is_canonical_root: &mut bool,
    out: &mut Vec<u8>,
) -> Result<()> {
    if !*capturing {
        if is_target_match(e, target_id) {
            *capturing = true;
            write_c14n_start_tag(e, *is_canonical_root, in_scope_default_ns, out)?;
            out.extend_from_slice(b"</");
            out.extend_from_slice(e.name().as_ref());
            out.push(b'>');
            *is_canonical_root = false;
        }
        return Ok(());
    }

    if skip_sig_depth > 0 || is_signature_tag(e) {
        return Ok(());
    }

    write_c14n_start_tag(e, *is_canonical_root, in_scope_default_ns, out)?;
    out.extend_from_slice(b"</");
    out.extend_from_slice(e.name().as_ref());
    out.push(b'>');
    *is_canonical_root = false;
    Ok(())
}

fn is_target_match(e: &quick_xml::events::BytesStart<'_>, target_id: Option<&str>) -> bool {
    let Some(id_val) = target_id else {
        return true;
    };
    for attr in e.attributes().flatten() {
        if attr.key.as_ref().eq_ignore_ascii_case(b"id") {
            let val = String::from_utf8_lossy(attr.value.as_ref());
            if val == id_val {
                return true;
            }
        }
    }
    false
}

fn is_signature_tag(e: &quick_xml::events::BytesStart<'_>) -> bool {
    let name = e.name();
    let name_bytes = name.as_ref();
    if name_bytes == b"Signature" || name_bytes.ends_with(b":Signature") {
        for attr in e.attributes().flatten() {
            if attr.key.as_ref() == b"xmlns" || attr.key.as_ref().starts_with(b"xmlns:") {
                let val = String::from_utf8_lossy(attr.value.as_ref());
                if val == "http://www.w3.org/2000/09/xmldsig#" {
                    return true;
                }
            }
        }
    }
    false
}

fn write_c14n_start_tag(
    e: &quick_xml::events::BytesStart<'_>,
    is_root: bool,
    in_scope_default_ns: Option<&str>,
    out: &mut Vec<u8>,
) -> Result<()> {
    out.push(b'<');
    out.extend_from_slice(e.name().as_ref());

    let mut ns_decls = Vec::new();
    let mut regular_attrs = Vec::new();

    for attr in e.attributes() {
        let a = attr.map_err(|err| Error::Xml(err.to_string()))?;
        let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
        let val = a
            .unescape_value()
            .map_err(|err| Error::Xml(err.to_string()))?
            .to_string();

        if key == "xmlns" || key.starts_with("xmlns:") {
            ns_decls.push((key, val));
        } else {
            regular_attrs.push((key, val));
        }
    }

    if is_root {
        if let Some(default_ns) = in_scope_default_ns {
            if !ns_decls.iter().any(|(k, _)| k == "xmlns") {
                ns_decls.push(("xmlns".to_string(), default_ns.to_string()));
            }
        }
    }

    ns_decls.sort_by(|(k1, _), (k2, _)| {
        if k1 == "xmlns" {
            std::cmp::Ordering::Less
        } else if k2 == "xmlns" {
            std::cmp::Ordering::Greater
        } else {
            k1.cmp(k2)
        }
    });

    regular_attrs.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

    for (k, v) in ns_decls {
        out.push(b' ');
        out.extend_from_slice(k.as_bytes());
        out.extend_from_slice(b"=\"");
        escape_c14n_attr_into(&v, out);
        out.push(b'"');
    }

    for (k, v) in regular_attrs {
        out.push(b' ');
        out.extend_from_slice(k.as_bytes());
        out.extend_from_slice(b"=\"");
        escape_c14n_attr_into(&v, out);
        out.push(b'"');
    }

    out.push(b'>');
    Ok(())
}

fn escape_c14n_attr_into(val: &str, out: &mut Vec<u8>) {
    for c in val.chars() {
        match c {
            '&' => out.extend_from_slice(b"&amp;"),
            '<' => out.extend_from_slice(b"&lt;"),
            '"' => out.extend_from_slice(b"&quot;"),
            '\t' => out.extend_from_slice(b"&#x9;"),
            '\n' => out.extend_from_slice(b"&#xA;"),
            '\r' => out.extend_from_slice(b"&#xD;"),
            other => {
                let mut buf = [0u8; 4];
                let s = other.encode_utf8(&mut buf);
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
}

fn escape_c14n_text_into(text: &str, out: &mut Vec<u8>) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '&' => out.extend_from_slice(b"&amp;"),
            '<' => out.extend_from_slice(b"&lt;"),
            '>' => out.extend_from_slice(b"&gt;"),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    // Let \n be emitted in next iteration
                } else {
                    out.extend_from_slice(b"&#xD;");
                }
            }
            '\n' => out.push(b'\n'),
            other => {
                let mut buf = [0u8; 4];
                let s = other.encode_utf8(&mut buf);
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
}

/// Signs an XML document targeting a specific element ID using `XMLDSig`.
///
/// Computes C14N digest of the referenced target, creates `<SignedInfo>`, signs
/// it with `signer`, constructs `<KeyInfo>` with `cert`, and inserts `<Signature>`
/// into the XML.
///
/// # Errors
///
/// Returns [`Error`] if target is not found, signing fails, or XML is invalid.
pub fn sign(
    xml: &[u8],
    target_id: &str,
    signer: &dyn Signer,
    cert: &dyn Certificate,
    options: SignatureOptions,
) -> Result<Vec<u8>> {
    let canonical_target = canonicalize_node(xml, Some(target_id), None)?;
    let digest = options.digest_algorithm.digest(&canonical_target);
    let digest_b64 = BASE64_STANDARD.encode(&digest);

    let signed_info_xml = format!(
        "<SignedInfo><CanonicalizationMethod Algorithm=\"http://www.w3.org/TR/2001/REC-xml-c14n-20010315\"/><SignatureMethod Algorithm=\"{}\"/><Reference URI=\"#{target_id}\"><Transforms><Transform Algorithm=\"http://www.w3.org/2000/09/xmldsig#enveloped-signature\"/></Transforms><DigestMethod Algorithm=\"{}\"/><DigestValue>{digest_b64}</DigestValue></Reference></SignedInfo>",
        options.signature_algorithm.uri(),
        options.digest_algorithm.uri()
    );

    let canonical_signed_info = canonicalize_node(
        signed_info_xml.as_bytes(),
        None,
        Some("http://www.w3.org/2000/09/xmldsig#"),
    )?;

    let sig_bytes = signer.sign(&canonical_signed_info)?;
    let sig_b64 = BASE64_STANDARD.encode(&sig_bytes);

    let pubkey = cert.public_key()?;
    let m_b64 = BASE64_STANDARD.encode(pubkey.n().to_bytes_be());
    let e_b64 = BASE64_STANDARD.encode(pubkey.e().to_bytes_be());
    let cert_b64 = BASE64_STANDARD.encode(cert.raw_der());

    let key_info_xml = format!(
        "<KeyInfo><KeyValue><RSAKeyValue><Modulus>{m_b64}</Modulus><Exponent>{e_b64}</Exponent></RSAKeyValue></KeyValue><X509Data><X509Certificate>{cert_b64}</X509Certificate></X509Data></KeyInfo>"
    );

    let signature_xml = format!(
        "<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\">{signed_info_xml}<SignatureValue>{sig_b64}</SignatureValue>{key_info_xml}</Signature>"
    );

    insert_signature_into_xml(xml, &signature_xml)
}

/// Signs a DTE XML document using the default SII signature algorithm (RSA-SHA1).
///
/// Automatically locates the `<Documento ID="...">` tag in the DTE.
///
/// # Errors
///
/// Returns [`Error`] if `<Documento>` tag or its `ID` attribute is missing.
pub fn sign_dte(xml: &[u8], signer: &dyn Signer, cert: &dyn Certificate) -> Result<Vec<u8>> {
    sign_dte_with_options(xml, signer, cert, SignatureOptions::default())
}

/// Signs a DTE XML document with explicit [`SignatureOptions`].
///
/// # Errors
///
/// Returns [`Error`] if `<Documento>` tag or its `ID` attribute is missing.
pub fn sign_dte_with_options(
    xml: &[u8],
    signer: &dyn Signer,
    cert: &dyn Certificate,
    options: SignatureOptions,
) -> Result<Vec<u8>> {
    let doc_id = extract_documento_id(xml)?;
    sign(xml, &doc_id, signer, cert, options)
}

/// Signs an entire XML document using an enveloped `XMLDSig` signature with `URI=""`.
///
/// Complies with Chilean SII authentication standard for `getToken` requests.
///
/// # Errors
///
/// Returns [`Error`] if signing or key extraction fails.
pub fn sign_enveloped(
    xml: &[u8],
    signer: &dyn Signer,
    cert: &dyn Certificate,
    options: SignatureOptions,
) -> Result<Vec<u8>> {
    let canonical_target = canonicalize_node(xml, None, None)?;
    let digest = options.digest_algorithm.digest(&canonical_target);
    let digest_b64 = BASE64_STANDARD.encode(&digest);

    let signed_info_xml = format!(
        "<SignedInfo><CanonicalizationMethod Algorithm=\"http://www.w3.org/TR/2001/REC-xml-c14n-20010315\"/><SignatureMethod Algorithm=\"{}\"/><Reference URI=\"\"><Transforms><Transform Algorithm=\"http://www.w3.org/2000/09/xmldsig#enveloped-signature\"/></Transforms><DigestMethod Algorithm=\"{}\"/><DigestValue>{digest_b64}</DigestValue></Reference></SignedInfo>",
        options.signature_algorithm.uri(),
        options.digest_algorithm.uri()
    );

    let canonical_signed_info = canonicalize_node(
        signed_info_xml.as_bytes(),
        None,
        Some("http://www.w3.org/2000/09/xmldsig#"),
    )?;

    let sig_bytes = signer.sign(&canonical_signed_info)?;
    let sig_b64 = BASE64_STANDARD.encode(&sig_bytes);

    let pubkey = cert.public_key()?;
    let m_b64 = BASE64_STANDARD.encode(pubkey.n().to_bytes_be());
    let e_b64 = BASE64_STANDARD.encode(pubkey.e().to_bytes_be());
    let cert_b64 = BASE64_STANDARD.encode(cert.raw_der());

    let key_info_xml = format!(
        "<KeyInfo><KeyValue><RSAKeyValue><Modulus>{m_b64}</Modulus><Exponent>{e_b64}</Exponent></RSAKeyValue></KeyValue><X509Data><X509Certificate>{cert_b64}</X509Certificate></X509Data></KeyInfo>"
    );

    let signature_xml = format!(
        "<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\">{signed_info_xml}<SignatureValue>{sig_b64}</SignatureValue>{key_info_xml}</Signature>"
    );

    insert_signature_into_xml(xml, &signature_xml)
}

fn extract_documento_id(xml: &[u8]) -> Result<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == b"Documento" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref().eq_ignore_ascii_case(b"id") {
                            let val = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                            return Ok(val);
                        }
                    }
                    return Err(Error::Signature(
                        "Documento element found but missing ID attribute".into(),
                    ));
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(e.to_string())),
            _ => {}
        }
    }

    Err(Error::Signature(
        "no Documento element found in DTE XML".into(),
    ))
}

fn insert_signature_into_xml(xml: &[u8], signature_xml: &str) -> Result<Vec<u8>> {
    let xml_str = std::str::from_utf8(xml).map_err(|e| Error::Xml(e.to_string()))?;

    // Preferred insertion before root closing tag (e.g. </DTE>, </EnvioDTE>, </Documento>, </getToken>)
    let closing_candidates = [
        "</DTE>",
        "</EnvioDTE>",
        "</EnvioBOLETA>",
        "</Documento>",
        "</getToken>",
    ];

    for candidate in &closing_candidates {
        if let Some(pos) = xml_str.rfind(candidate) {
            let prefix = xml_str
                .get(..pos)
                .ok_or_else(|| Error::Xml("internal error extracting xml prefix".into()))?;
            let suffix = xml_str
                .get(pos..)
                .ok_or_else(|| Error::Xml("internal error extracting xml suffix".into()))?;
            let mut result = Vec::with_capacity(xml.len() + signature_xml.len());
            result.extend_from_slice(prefix.as_bytes());
            result.extend_from_slice(signature_xml.as_bytes());
            result.extend_from_slice(suffix.as_bytes());
            return Ok(result);
        }
    }

    // Fallback: append signature before last closing tag '</'
    if let Some(pos) = xml_str.rfind("</") {
        let prefix = xml_str
            .get(..pos)
            .ok_or_else(|| Error::Xml("internal error extracting fallback prefix".into()))?;
        let suffix = xml_str
            .get(pos..)
            .ok_or_else(|| Error::Xml("internal error extracting fallback suffix".into()))?;
        let mut result = Vec::with_capacity(xml.len() + signature_xml.len());
        result.extend_from_slice(prefix.as_bytes());
        result.extend_from_slice(signature_xml.as_bytes());
        result.extend_from_slice(suffix.as_bytes());
        return Ok(result);
    }

    Err(Error::Xml(
        "unable to find closing root tag to insert signature".into(),
    ))
}

/// Verifies an `XMLDSig` enveloped signature in an XML document (RF-03, SIG-02).
///
/// Verifies both the referenced payload digest and the cryptographic signature.
/// Returns `Ok(false)` if payload or signature has been tampered with.
///
/// # Errors
///
/// Returns [`Error`] if XML is malformed or lacks necessary cryptographic elements.
pub fn verify(xml: &[u8]) -> Result<bool> {
    let Ok(xml_str) = std::str::from_utf8(xml) else {
        return Ok(false);
    };

    let sig_info = parse_signature_components(xml_str)?;

    let target_id = sig_info.reference_uri.strip_prefix('#');
    let Ok(canonical_target) = canonicalize_node(xml, target_id, None) else {
        return Ok(false);
    };

    let computed_digest = sig_info.digest_algorithm.digest(&canonical_target);
    if computed_digest != sig_info.expected_digest {
        return Ok(false);
    }

    let canonical_signed_info = canonicalize_node(
        sig_info.signed_info_raw.as_bytes(),
        None,
        Some("http://www.w3.org/2000/09/xmldsig#"),
    )?;

    Ok(verify_signed_info_crypto(
        &canonical_signed_info,
        &sig_info.signature_bytes,
        &sig_info.public_key,
        sig_info.signature_algorithm,
    ))
}

struct ParsedSignature {
    signed_info_raw: String,
    reference_uri: String,
    digest_algorithm: DigestAlgorithm,
    expected_digest: Vec<u8>,
    signature_algorithm: SignatureAlgorithm,
    signature_bytes: Vec<u8>,
    public_key: RsaPublicKey,
}

fn parse_signature_components(xml: &str) -> Result<ParsedSignature> {
    let signed_info_raw = extract_xml_tag_content(xml, "SignedInfo", true)?;
    let sig_val_b64 = extract_xml_tag_content(xml, "SignatureValue", false)?;
    let clean_sig: String = sig_val_b64.chars().filter(|c| !c.is_whitespace()).collect();
    let signature_bytes = BASE64_STANDARD
        .decode(&clean_sig)
        .map_err(|e| Error::Signature(format!("invalid SignatureValue base64: {e}")))?;

    let c14n_algo =
        extract_attribute_value(&signed_info_raw, "CanonicalizationMethod", "Algorithm")?;
    if !c14n_algo.starts_with("http://www.w3.org/TR/2001/REC-xml-c14n-20010315") {
        return Err(Error::Signature(format!(
            "unsupported CanonicalizationMethod Algorithm: {c14n_algo}"
        )));
    }

    let sig_algo_uri = extract_attribute_value(&signed_info_raw, "SignatureMethod", "Algorithm")?;
    let signature_algorithm = SignatureAlgorithm::from_uri(&sig_algo_uri)?;

    let reference_uri = extract_attribute_value(&signed_info_raw, "Reference", "URI")?;
    let digest_algo_uri = extract_attribute_value(&signed_info_raw, "DigestMethod", "Algorithm")?;
    let digest_algorithm = DigestAlgorithm::from_uri(&digest_algo_uri)?;

    let digest_val_b64 = extract_xml_tag_content(&signed_info_raw, "DigestValue", false)?;
    let clean_digest: String = digest_val_b64
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let expected_digest = BASE64_STANDARD
        .decode(&clean_digest)
        .map_err(|e| Error::Signature(format!("invalid DigestValue base64: {e}")))?;

    let key_info_raw = extract_xml_tag_content(xml, "KeyInfo", true)?;
    let public_key = extract_public_key_from_key_info(&key_info_raw)?;

    Ok(ParsedSignature {
        signed_info_raw,
        reference_uri,
        digest_algorithm,
        expected_digest,
        signature_algorithm,
        signature_bytes,
        public_key,
    })
}

fn extract_public_key_from_key_info(info_xml: &str) -> Result<RsaPublicKey> {
    if let Ok(cert_b64) = extract_xml_tag_content(info_xml, "X509Certificate", false) {
        let clean_cert: String = cert_b64.chars().filter(|c| !c.is_whitespace()).collect();
        if let Ok(der) = BASE64_STANDARD.decode(&clean_cert) {
            if let Ok(key) = extract_rsa_public_key_from_der(&der) {
                return Ok(key);
            }
        }
    }

    let mod_b64 = extract_xml_tag_content(info_xml, "Modulus", false)?;
    let exp_b64 = extract_xml_tag_content(info_xml, "Exponent", false)?;
    rsa_public_key_from_components(&mod_b64, &exp_b64)
}

fn verify_signed_info_crypto(
    canonical_signed_info: &[u8],
    sig_bytes: &[u8],
    public_key: &RsaPublicKey,
    algo: SignatureAlgorithm,
) -> bool {
    match algo {
        SignatureAlgorithm::RsaSha1 => {
            let verifying_key = VerifyingKey::<Sha1>::new(public_key.clone());
            let Ok(rsa_sig) = rsa::pkcs1v15::Signature::try_from(sig_bytes) else {
                return false;
            };
            verifying_key
                .verify(canonical_signed_info, &rsa_sig)
                .is_ok()
        }
        SignatureAlgorithm::RsaSha256 => {
            let verifying_key = VerifyingKey::<Sha256>::new(public_key.clone());
            let Ok(rsa_sig) = rsa::pkcs1v15::Signature::try_from(sig_bytes) else {
                return false;
            };
            verifying_key
                .verify(canonical_signed_info, &rsa_sig)
                .is_ok()
        }
    }
}

fn extract_xml_tag_content(xml: &str, tag_name: &str, include_tag: bool) -> Result<String> {
    let open_tag = format!("<{tag_name}");
    let close_tag = format!("</{tag_name}>");

    let start_idx = xml
        .find(&open_tag)
        .ok_or_else(|| Error::Signature(format!("missing tag <{tag_name}>")))?;

    let close_idx = xml
        .find(&close_tag)
        .ok_or_else(|| Error::Signature(format!("missing closing tag </{tag_name}>")))?;

    if include_tag {
        let end_idx = close_idx.saturating_add(close_tag.len());
        let slice = xml
            .get(start_idx..end_idx)
            .ok_or_else(|| Error::Signature("internal slice error extracting xml tag".into()))?;
        Ok(slice.to_string())
    } else {
        let tag_header_slice = xml
            .get(start_idx..)
            .ok_or_else(|| Error::Signature("internal slice error extracting tag header".into()))?;
        let header_end = tag_header_slice
            .find('>')
            .ok_or_else(|| Error::Signature(format!("malformed tag <{tag_name}>")))?;
        let content_start = start_idx.saturating_add(header_end).saturating_add(1);
        let content = xml.get(content_start..close_idx).ok_or_else(|| {
            Error::Signature("internal slice error extracting tag content".into())
        })?;
        Ok(content.to_string())
    }
}

fn extract_attribute_value(xml: &str, element_name: &str, attr_name: &str) -> Result<String> {
    let elem_tag = format!("<{element_name}");
    let elem_start = xml
        .find(&elem_tag)
        .ok_or_else(|| Error::Signature(format!("element <{element_name}> not found")))?;
    let elem_slice = xml
        .get(elem_start..)
        .ok_or_else(|| Error::Signature("slice error reading element".into()))?;
    let elem_end = elem_slice
        .find('>')
        .ok_or_else(|| Error::Signature(format!("unclosed element <{element_name}>")))?;
    let tag_header = elem_slice
        .get(..elem_end)
        .ok_or_else(|| Error::Signature("slice error reading tag header".into()))?;

    let attr_pattern = format!("{attr_name}=\"");
    let attr_start = tag_header.find(&attr_pattern).ok_or_else(|| {
        Error::Signature(format!(
            "attribute '{attr_name}' not found on <{element_name}>"
        ))
    })?;

    let val_start = attr_start.saturating_add(attr_pattern.len());
    let val_slice = tag_header
        .get(val_start..)
        .ok_or_else(|| Error::Signature("slice error reading attribute value".into()))?;
    let val_end = val_slice
        .find('"')
        .ok_or_else(|| Error::Signature(format!("unclosed quote for attribute '{attr_name}'")))?;

    let val = val_slice
        .get(..val_end)
        .ok_or_else(|| Error::Signature("slice error reading attribute content".into()))?;
    Ok(val.to_string())
}

fn decode_pem_body(pem: &str, label: &str) -> Result<Vec<u8>> {
    let begin_marker = format!("-----BEGIN {label}-----");
    let end_marker = format!("-----END {label}-----");

    let start = pem
        .find(&begin_marker)
        .ok_or_else(|| Error::Crypto(format!("missing PEM start marker '{begin_marker}'")))?;
    let end = pem
        .find(&end_marker)
        .ok_or_else(|| Error::Crypto(format!("missing PEM end marker '{end_marker}'")))?;

    let body_start = start.saturating_add(begin_marker.len());
    let body = pem
        .get(body_start..end)
        .ok_or_else(|| Error::Crypto("internal error slicing PEM body".into()))?;

    let clean: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    BASE64_STANDARD
        .decode(&clean)
        .map_err(|e| Error::Crypto(format!("invalid base64 in PEM body: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::RsaPrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::rand_core::OsRng;
    use rsa::signature::{SignatureEncoding, Signer as _};

    struct TestToySigner {
        private_key: RsaPrivateKey,
        algo: SignatureAlgorithm,
    }

    impl Signer for TestToySigner {
        fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
            match self.algo {
                SignatureAlgorithm::RsaSha1 => {
                    let key = SigningKey::<Sha1>::new(self.private_key.clone());
                    Ok(key.sign(data).to_vec())
                }
                SignatureAlgorithm::RsaSha256 => {
                    let key = SigningKey::<Sha256>::new(self.private_key.clone());
                    Ok(key.sign(data).to_vec())
                }
            }
        }
    }

    struct TestToyCert {
        raw_der: Vec<u8>,
        public_key: RsaPublicKey,
    }

    impl Certificate for TestToyCert {
        fn raw_der(&self) -> &[u8] {
            &self.raw_der
        }

        fn public_key(&self) -> Result<RsaPublicKey> {
            Ok(self.public_key.clone())
        }
    }

    fn generate_toy_keypair() -> (RsaPrivateKey, RsaPublicKey) {
        let mut rng = OsRng;
        let priv_key = RsaPrivateKey::new(&mut rng, 1024).expect("rsa keygen");
        let pub_key = RsaPublicKey::from(&priv_key);
        (priv_key, pub_key)
    }

    #[test]
    fn test_c14n_whitespace_and_attribute_reordering() {
        let xml1 =
            b"<Documento   b=\"2\"   ID=\"F60T33\"   a=\"1\"  ><Encabezado/><Detalle/></Documento>";
        let xml2 = b"<Documento ID=\"F60T33\" a=\"1\" b=\"2\"><Encabezado></Encabezado><Detalle></Detalle></Documento>";

        let c14n1 = canonicalize_node(xml1, Some("F60T33"), None).expect("c14n1");
        let c14n2 = canonicalize_node(xml2, Some("F60T33"), None).expect("c14n2");

        assert_eq!(c14n1, c14n2);
        assert_eq!(
            std::str::from_utf8(&c14n1).unwrap(),
            "<Documento ID=\"F60T33\" a=\"1\" b=\"2\"><Encabezado></Encabezado><Detalle></Detalle></Documento>"
        );
    }

    #[test]
    fn test_c14n_namespace_propagation_on_signed_info() {
        let si_xml = b"<SignedInfo><CanonicalizationMethod Algorithm=\"http://www.w3.org/TR/2001/REC-xml-c14n-20010315\"/></SignedInfo>";
        let c14n = canonicalize_node(si_xml, None, Some("http://www.w3.org/2000/09/xmldsig#"))
            .expect("c14n");

        assert_eq!(
            std::str::from_utf8(&c14n).unwrap(),
            "<SignedInfo xmlns=\"http://www.w3.org/2000/09/xmldsig#\"><CanonicalizationMethod Algorithm=\"http://www.w3.org/TR/2001/REC-xml-c14n-20010315\"></CanonicalizationMethod></SignedInfo>"
        );
    }

    #[test]
    fn test_sign_and_verify_roundtrip_sha1() {
        let (priv_key, pub_key) = generate_toy_keypair();
        let signer = TestToySigner {
            private_key: priv_key,
            algo: SignatureAlgorithm::RsaSha1,
        };
        let cert = TestToyCert {
            raw_der: vec![0x30, 0x03, 0x02, 0x01, 0x01],
            public_key: pub_key,
        };

        let xml = b"<DTE version=\"1.0\"><Documento ID=\"F60T33\"><Encabezado><IdDoc><Folio>60</Folio></IdDoc></Encabezado></Documento></DTE>";
        let signed_xml = sign_dte(xml, &signer, &cert).expect("sign_dte");

        let ok = verify(&signed_xml).expect("verify");
        assert!(ok, "signature verification should succeed");
    }

    #[test]
    fn test_sign_and_verify_roundtrip_sha256() {
        let (priv_key, pub_key) = generate_toy_keypair();
        let signer = TestToySigner {
            private_key: priv_key,
            algo: SignatureAlgorithm::RsaSha256,
        };
        let cert = TestToyCert {
            raw_der: vec![0x30, 0x03, 0x02, 0x01, 0x01],
            public_key: pub_key,
        };

        let xml = b"<DTE version=\"1.0\"><Documento ID=\"F60T33\"><Encabezado><IdDoc><Folio>60</Folio></IdDoc></Encabezado></Documento></DTE>";
        let options = SignatureOptions {
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            digest_algorithm: DigestAlgorithm::Sha256,
        };

        let signed_xml =
            sign_dte_with_options(xml, &signer, &cert, options).expect("sign_dte_sha256");
        let ok = verify(&signed_xml).expect("verify sha256");
        assert!(ok, "SHA-256 signature verification should succeed");
    }

    #[test]
    fn test_verify_detects_payload_tampering() {
        let (priv_key, pub_key) = generate_toy_keypair();
        let signer = TestToySigner {
            private_key: priv_key,
            algo: SignatureAlgorithm::RsaSha1,
        };
        let cert = TestToyCert {
            raw_der: vec![0x30, 0x03, 0x02, 0x01, 0x01],
            public_key: pub_key,
        };

        let xml = b"<DTE version=\"1.0\"><Documento ID=\"F60T33\"><Encabezado><IdDoc><Folio>60</Folio></IdDoc></Encabezado></Documento></DTE>";
        let signed_xml = sign_dte(xml, &signer, &cert).expect("sign_dte");

        // Tamper with payload (change Folio 60 -> 61)
        let tampered_str = String::from_utf8(signed_xml)
            .unwrap()
            .replace("<Folio>60</Folio>", "<Folio>61</Folio>");
        let ok = verify(tampered_str.as_bytes()).expect("verify tampered");
        assert!(!ok, "tampered payload must fail verification");
    }

    #[test]
    fn test_verify_detects_signature_tampering() {
        let (priv_key, pub_key) = generate_toy_keypair();
        let signer = TestToySigner {
            private_key: priv_key,
            algo: SignatureAlgorithm::RsaSha1,
        };
        let cert = TestToyCert {
            raw_der: vec![0x30, 0x03, 0x02, 0x01, 0x01],
            public_key: pub_key,
        };

        let xml = b"<DTE version=\"1.0\"><Documento ID=\"F60T33\"><Encabezado><IdDoc><Folio>60</Folio></IdDoc></Encabezado></Documento></DTE>";
        let signed_xml = sign_dte(xml, &signer, &cert).expect("sign_dte");

        // Tamper with SignatureValue (flip first character)
        let signed_str = String::from_utf8(signed_xml).unwrap();
        let (before, rest) = signed_str.split_once("<SignatureValue>").unwrap();
        let (sig_content, after) = rest.split_once("</SignatureValue>").unwrap();
        let mut tampered_sig = sig_content.to_string();
        if let Some(first) = tampered_sig.chars().next() {
            let replacement = if first == 'A' { 'B' } else { 'A' };
            tampered_sig.replace_range(..1, &replacement.to_string());
        }
        let tampered_str =
            format!("{before}<SignatureValue>{tampered_sig}</SignatureValue>{after}");

        let ok = verify(tampered_str.as_bytes()).expect("verify tampered sig");
        assert!(!ok, "tampered signature must fail verification");
    }
}
