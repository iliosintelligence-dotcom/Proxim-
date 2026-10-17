//! ES256 (ECDSA P-256 with SHA-256) verification, low-S only (pack-spec 2).

use crate::error::PackError;
use crate::hex;
use p256::ecdsa::signature::Verifier as _;
use p256::ecdsa::{Signature, VerifyingKey};

/// A P-256 public key parsed from a SEC1 uncompressed point (65 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey(VerifyingKey);

impl PublicKey {
    /// # Errors
    /// [`PackError::Malformed`] unless `bytes` is a 65-byte uncompressed point on the curve.
    pub fn from_sec1(bytes: &[u8]) -> Result<Self, PackError> {
        if bytes.len() != 65 || bytes.first() != Some(&0x04) {
            return Err(PackError::Malformed);
        }
        VerifyingKey::from_sec1_bytes(bytes)
            .map(Self)
            .map_err(|_| PackError::Malformed)
    }

    /// # Errors
    /// As [`Self::from_sec1`], or if `s` is not 130 lowercase hex characters.
    pub fn from_hex(s: &str) -> Result<Self, PackError> {
        Self::from_sec1(&hex::decode_array::<65>(s)?)
    }

    /// Verify a raw `r || s` signature over `msg`.
    ///
    /// # Errors
    /// [`PackError::BadSignature`] if the signature is malformed, high-S or does not verify.
    pub fn verify(&self, msg: &[u8], sig: &[u8; 64]) -> Result<(), PackError> {
        let sig = Signature::from_slice(sig).map_err(|_| PackError::BadSignature)?;
        if sig.normalize_s().to_bytes() != sig.to_bytes() {
            return Err(PackError::BadSignature);
        }
        self.0
            .verify(msg, &sig)
            .map_err(|_| PackError::BadSignature)
    }
}
