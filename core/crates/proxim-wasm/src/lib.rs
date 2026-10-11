//! proxim-wasm: binding surface stub (engine-spec 9 conformance entry).
//! Skeleton: JSON in, JSON out; the real shim parses canonical JSON, calls `decide`, writes canonical JSON.

use proxim_types::EngineError;

/// `decide_json(state, fix, signposts, policy, consent, clock, seed) -> {output, state}`.
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn decide_json(
    _state: &str,
    _fix: &str,
    _signposts: &str,
    _policy: &str,
    _consent: &str,
    _clock: &str,
    _seed: u64,
) -> Result<String, EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_typed_error() {
        assert_eq!(
            decide_json("", "", "", "", "", "", 0),
            Err(EngineError::NotImplemented)
        );
    }
}
