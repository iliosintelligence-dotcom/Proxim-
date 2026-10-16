//! proxim-sim: replay harness and golden runner stub (engine-spec 9), and the phone simulator
//! ([`phone`], [`town`], [`report`]; `docs/phone-simulator.md`). May use `std`.

extern crate alloc;

pub mod phone;
pub mod report;
pub mod town;

use proxim_types::EngineError;

/// Run a golden vector file; `bless` regenerates it (needs `PROXIM_BLESS=1`, reviewed diff).
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn run_golden(_path: &str, _bless: bool) -> Result<(), EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_typed_error() {
        assert_eq!(run_golden("x", false), Err(EngineError::NotImplemented));
    }
}
