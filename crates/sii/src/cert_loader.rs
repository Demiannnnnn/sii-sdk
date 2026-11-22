//! Loading the issuer's digital certificate and private key (CERT-LOADER-01).
//!
//! The core crate is sans-IO and never opens a certificate on its own (ADR-02). This
//! module, behind the `pkcs12` feature, provides a concrete [`Signer`] +
//! [`Certificate`] holder, [`SiiIdentity`], loaded from PEM or PKCS#12 (`.pfx`/`.p12`).
//!
//! The private key is held in memory, never logged and redacted from [`fmt::Debug`]
//! (SEC-02). Real certificates and their passwords come from the caller (an env var or a
//! file the user points at), never hard-coded (SEC-03).

use crate::codes::Ambiente;
use crate::error::{Error, Result};
use crate::signature::{Certificate, SignatureAlgorithm, Signer, X509Certificate};
use rsa::RsaPrivateKey;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{SignatureEncoding, Signer as _};
use sha1::Sha1;
use sha2::Sha256;
use std::fmt;

/// An issuer identity: the X.509 certificate plus its RSA private key.
///
/// Implements [`Signer`] (signs DTEs, envelopes and the authentication seed with the
/// algorithm matching the target [`Ambiente`]) and [`Certificate`] (exposes the X.509
/// certificate for `KeyInfo`). Load it with [`SiiIdentity::from_pem`] or
/// [`SiiIdentity::from_pkcs12`].
pub struct SiiIdentity {
    private_key: RsaPrivateKey,
    algorithm: SignatureAlgorithm,
    certificate: X509Certificate,
}

impl SiiIdentity {
    /// Builds an identity from a certificate and private key already in memory.
    #[must_use]
    pub fn new(
        certificate: X509Certificate,
        private_key: RsaPrivateKey,
        ambiente: Ambiente,
    ) -> Self {
        Self {
            private_key,
            algorithm: SignatureAlgorithm::for_ambiente(ambiente),
            certificate,
        }
    }

    /// Loads an identity from PEM text: an X.509 `CERTIFICATE` and an RSA private key
    /// in either PKCS#8 (`PRIVATE KEY`) or PKCS#1 (`RSA PRIVATE KEY`) form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Crypto`] if the certificate or private key cannot be parsed.
    pub fn from_pem(cert_pem: &str, key_pem: &str, ambiente: Ambiente) -> Result<Self> {
        let certificate = X509Certificate::from_pem(cert_pem)?;
        let private_key = parse_private_key_pem(key_pem)?;
        Ok(Self::new(certificate, private_key, ambiente))
    }

    /// Loads an identity from PKCS#12 (`.pfx` / `.p12`) bytes protected by `password`.
    ///
    /// This is the format the SII hands out for the taxpayer's digital certificate. The
    /// `password` must come from a secure source (an environment variable or a prompt),
    /// never a hard-coded literal (SEC-03).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Crypto`] if the container cannot be decrypted with `password`, or
    /// does not contain exactly one RSA key and certificate.
    pub fn from_pkcs12(pfx: &[u8], password: &str, ambiente: Ambiente) -> Result<Self> {
        let (cert_der, key_der) = pkcs12::extract_cert_and_key(pfx, password)?;
        let certificate = X509Certificate::from_der(&cert_der)?;
        let private_key = parse_private_key_der(&key_der)?;
        Ok(Self::new(certificate, private_key, ambiente))
    }

    /// Returns the signature algorithm this identity signs with.
    #[must_use]
    pub const fn signature_algorithm(&self) -> SignatureAlgorithm {
        self.algorithm
    }

    /// Returns the underlying certificate.
    #[must_use]
    pub const fn certificate(&self) -> &X509Certificate {
        &self.certificate
    }
}

impl Signer for SiiIdentity {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
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

impl Certificate for SiiIdentity {
    fn raw_der(&self) -> &[u8] {
        self.certificate.raw_der()
    }
}

impl fmt::Debug for SiiIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SiiIdentity")
            .field("algorithm", &self.algorithm)
            .field("private_key", &"<redacted>")
            .field("certificate", &"<x509>")
            .finish()
    }
}

/// Parses an RSA private key from PEM, trying PKCS#8 then PKCS#1.
///
/// # Errors
///
/// Returns [`Error::Crypto`] if neither form parses.
fn parse_private_key_pem(key_pem: &str) -> Result<RsaPrivateKey> {
    let trimmed = key_pem.trim();
    if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(trimmed) {
        return Ok(key);
    }
    RsaPrivateKey::from_pkcs1_pem(trimmed)
        .map_err(|e| Error::Crypto(format!("failed to parse RSA private key from PEM: {e}")))
}

/// Parses an RSA private key from DER, trying PKCS#8 then PKCS#1.
///
/// # Errors
///
/// Returns [`Error::Crypto`] if neither form parses.
fn parse_private_key_der(key_der: &[u8]) -> Result<RsaPrivateKey> {
    if let Ok(key) = RsaPrivateKey::from_pkcs8_der(key_der) {
        return Ok(key);
    }
    RsaPrivateKey::from_pkcs1_der(key_der)
        .map_err(|e| Error::Crypto(format!("failed to parse RSA private key from DER: {e}")))
}

/// PKCS#12 (`.pfx` / `.p12`) extraction behind the `p12` dependency.
mod pkcs12 {
    use crate::error::{Error, Result};

    /// Extracts the first certificate (DER) and private key (DER) from a PKCS#12 container.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Crypto`] if parsing or decryption with `password` fails, or if a
    /// certificate or key is missing.
    pub(super) fn extract_cert_and_key(pfx: &[u8], password: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        let container = p12::PFX::parse(pfx)
            .map_err(|e| Error::Crypto(format!("failed to parse PKCS#12 container: {e}")))?;

        if !container.verify_mac(password) {
            return Err(Error::Crypto(
                "PKCS#12 MAC verification failed (wrong password?)".into(),
            ));
        }

        let cert = container
            .cert_x509_bags(password)
            .map_err(|e| Error::Crypto(format!("failed to read PKCS#12 certificate bags: {e}")))?
            .into_iter()
            .next()
            .ok_or_else(|| Error::Crypto("PKCS#12 container has no certificate".into()))?;

        let key = container
            .key_bags(password)
            .map_err(|e| Error::Crypto(format!("failed to read PKCS#12 key bags: {e}")))?
            .into_iter()
            .next()
            .ok_or_else(|| Error::Crypto("PKCS#12 container has no private key".into()))?;

        Ok((cert, key))
    }
}
