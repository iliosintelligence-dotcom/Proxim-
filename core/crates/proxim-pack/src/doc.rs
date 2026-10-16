//! Shared decoding for the signed JSON documents.

use crate::PackResult;
use proxim_types::ReasonCode;
use proxim_types::strict::{JsonLimits, from_canonical_json_with};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Largest JWS token, and the array bound that lets a revocation list hold 65,536 entries.
pub const MAX_JWS_BYTES: usize = 262_144;
const MAX_DOC_ARRAY: usize = 65_536;

/// Strict, canonical decode of a JWS payload; any failure is `SIGNATURE_INVALID` (malformed
/// signed content is treated as untrusted, pack-format 2).
pub fn decode<T: DeserializeOwned + Serialize>(payload: &[u8]) -> PackResult<T> {
    let limits = JsonLimits {
        max_bytes: MAX_JWS_BYTES,
        max_array_len: MAX_DOC_ARRAY,
        ..JsonLimits::default()
    };
    from_canonical_json_with(payload, &limits).map_err(|_| ReasonCode::SignatureInvalid)
}

/// `issued_at_ms <= now < expires_at_ms`, else `PACK_EXPIRED` (a document from the future is
/// treated like an expired one: the clock and the document disagree, so neither is trusted).
pub fn check_window(issued_at_ms: i64, expires_at_ms: i64, now_ms: i64) -> PackResult<()> {
    if issued_at_ms <= now_ms && now_ms < expires_at_ms {
        Ok(())
    } else {
        Err(ReasonCode::PackExpired)
    }
}
