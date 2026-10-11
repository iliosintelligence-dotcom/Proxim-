//! The decision function (engine-spec 7). Pure: a function of its arguments only. Skeleton.
#![no_std]

#[cfg(test)]
extern crate std;

use proxim_types::{Clock, Consent, EngineError, Fix, Output, Policy, Signpost, State};

/// `decide(state, fix, signposts, policy, consent, clock, seed) -> (Output, State)`.
///
/// `seed` is carried but UNUSED in M1. A shell MUST treat any `Err` as
/// "no decision, state unchanged".
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn decide(
    _state: &State,
    _fix: &Fix,
    _signposts: &[Signpost],
    _policy: &Policy,
    _consent: &Consent,
    _clock: &Clock,
    _seed: u64,
) -> Result<(Output, State), EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_returns_typed_error_and_leaves_state_alone() {
        let fix = Fix {
            t_ms: 0,
            lat_e7: 0,
            lon_e7: 0,
            acc_mm: 5000,
            speed_mmps: None,
            speed_acc_mmps: None,
            course_ddeg: None,
            course_acc_ddeg: None,
            is_mock: false,
        };
        let st = State::empty();
        let clock = Clock {
            now_ms: 0,
            tz_offset_min: 0,
        };
        let r = decide(
            &st,
            &fix,
            &[],
            &Policy::default(),
            &Consent::default(),
            &clock,
            0,
        );
        assert_eq!(r, Err(EngineError::NotImplemented));
        assert_eq!(st, State::empty());
    }
}
