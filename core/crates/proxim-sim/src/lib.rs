//! proxim-sim: replay harness and golden runner stub (engine-spec 9), and the phone simulator
//! ([`phone`], [`town`], [`report`]; `docs/phone-simulator.md`). May use `std`.
//!
//! Scenario simulator (`docs/simulator.md`): [`rng`], [`geom`], [`trace`] (synthetic, GeoJSON and
//! GPX generators), [`noise`] (AR(1) + jumps, accuracy, dropouts), [`scenario`] (JSON scenario
//! files), [`oracle`] (judges each run against the noise-free truth), [`runner`] (drives the
//! engine and writes the canonical JSON report), [`junit`], [`geocases`] (turf.js comparison
//! cases) and [`stationary`] (the stationary-outside statistical check).

extern crate alloc;

pub mod geocases;
pub mod geom;
pub mod junit;
pub mod noise;
pub mod oracle;
pub mod phone;
pub mod report;
pub mod rng;
pub mod runner;
pub mod scenario;
pub mod stationary;
pub mod town;
pub mod trace;

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
