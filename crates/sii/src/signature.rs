//! `XMLDSig` digital signature, C14N canonicalization, and `Signer`/`Certificate` traits.

use crate::error::Result;

/// Trait for signing data using a private key (ADR-02, SIG-01).
///
/// Implemented by cryptographic key holders such as CAF private keys and issuer certificates.
pub trait Signer: Send + Sync {
    /// Signs input data bytes and returns the resulting digital signature bytes.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] if the cryptographic signing operation fails.
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;
}
