//! proxim-pack: stub (not built in M1). See docs/PLAN.md section 3.
#![no_std]

#[cfg(test)]
extern crate std;

use proxim_types::EngineError;

/// Placeholder entry point so dependants can link; returns a typed error.
///
/// # Errors
/// Always `NotImplemented`.
pub const fn not_built() -> Result<(), EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_typed_error() {
        assert_eq!(not_built(), Err(EngineError::NotImplemented));
    }
}
