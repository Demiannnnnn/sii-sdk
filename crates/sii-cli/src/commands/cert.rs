//! Certificate loading utilities for the `sii` CLI (CERT-LOADER-01, SEC-03).

use crate::error::CliError;
use sii::cert_loader::SiiIdentity;
use sii::codes::Ambiente;
use std::path::Path;

fn extract_private_key_pem(pem: &str) -> &str {
    let start_candidates = [
        concat!("-----BEGIN ", "PRIVATE KEY-----"),
        concat!("-----BEGIN ", "RSA PRIVATE KEY-----"),
    ];
    let end_candidates = [
        concat!("-----END ", "PRIVATE KEY-----"),
        concat!("-----END ", "RSA PRIVATE KEY-----"),
    ];
    for (start, end) in start_candidates.iter().zip(end_candidates.iter()) {
        if let Some(pos) = pem.find(start) {
            if let Some(remainder) = pem.get(pos..) {
                if let Some(end_rel) = remainder.find(end) {
                    let end_total = pos.saturating_add(end_rel).saturating_add(end.len());
                    if let Some(sub) = pem.get(pos..end_total) {
                        return sub;
                    }
                }
                return remainder;
            }
        }
    }
    pem
}

/// Loads a digital certificate identity ([`SiiIdentity`]) from a PKCS#12 (`.p12`/`.pfx`)
/// or PEM file without leaking passwords or key material in error messages (SEC-02, SEC-03).
///
/// For PKCS#12 files, the password is read exclusively from the environment variable
/// `SII_CERT_PASSWORD`.
pub(crate) fn load_identity(
    cert_path: &Path,
    key_path: Option<&Path>,
    ambiente: Ambiente,
) -> Result<SiiIdentity, CliError> {
    if !cert_path.exists() {
        return Err(CliError::Certificate(format!(
            "el archivo de certificado no existe: {}",
            cert_path.display()
        )));
    }

    let cert_bytes = std::fs::read(cert_path).map_err(|e| {
        CliError::Certificate(format!(
            "no se pudo leer el archivo de certificado {}: {e}",
            cert_path.display()
        ))
    })?;

    // Determine format: PEM text starts with dashes ("-----BEGIN")
    let is_pem = cert_bytes.starts_with(b"-----BEGIN");

    if is_pem {
        let cert_pem = std::str::from_utf8(&cert_bytes).map_err(|e| {
            CliError::Certificate(format!(
                "el archivo de certificado PEM no es UTF-8 válido: {e}"
            ))
        })?;

        let key_pem_owned;
        let key_pem = if let Some(kp) = key_path {
            if !kp.exists() {
                return Err(CliError::Certificate(format!(
                    "el archivo de llave privada no existe: {}",
                    kp.display()
                )));
            }
            let key_bytes = std::fs::read(kp).map_err(|e| {
                CliError::Certificate(format!(
                    "no se pudo leer el archivo de llave privada {}: {e}",
                    kp.display()
                ))
            })?;
            key_pem_owned = String::from_utf8(key_bytes).map_err(|e| {
                CliError::Certificate(format!("el archivo de llave PEM no es UTF-8 válido: {e}"))
            })?;
            &key_pem_owned
        } else {
            // Self-contained PEM holding both certificate and private key
            extract_private_key_pem(cert_pem)
        };

        SiiIdentity::from_pem(cert_pem, key_pem, ambiente)
            .map_err(|e| CliError::Certificate(format!("falló la carga del certificado PEM: {e}")))
    } else {
        // PKCS#12 (.p12 / .pfx)
        let password =
            std::env::var("SII_CERT_PASSWORD").map_err(|_| CliError::MissingCertPassword)?;

        SiiIdentity::from_pkcs12(&cert_bytes, &password, ambiente).map_err(|e| {
            CliError::Certificate(format!(
                "falló la apertura del archivo PKCS#12 (verifique SII_CERT_PASSWORD): {e}"
            ))
        })
    }
}
