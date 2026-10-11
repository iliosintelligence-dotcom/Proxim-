//! Canonical JSON writer (spec 3.2). Skeleton: the real writer is the next phase.

use crate::error::EngineError;
use alloc::string::String;
use serde::Serialize;

/// Serialize to canonical JSON: no whitespace, keys ascending by UTF-8 bytes,
/// integers only, no `null`, escapes only `"`, `\` and controls as lowercase `\u00xx`.
///
/// # Errors
/// Returns [`EngineError::NotImplemented`] until the writer is built.
pub fn to_canonical_json<T: Serialize>(_value: &T) -> Result<String, EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_typed_error_not_panic() {
        assert_eq!(to_canonical_json(&1_u8), Err(EngineError::NotImplemented));
    }
}
