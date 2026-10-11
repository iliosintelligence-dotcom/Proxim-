//! ENTER/EXIT and APPROACH detectors (engine-spec 6). Skeleton.
#![no_std]

#[cfg(test)]
extern crate std;

use proxim_path::PathSummary;
use proxim_types::{DetState, EngineError, EventKind, Fix, ReasonCode, Signpost};

/// A detector event (spec 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub t_ms: i64,
    pub d_rem_mm: Option<i64>,
    pub t_hat_ms: Option<i64>,
    pub dist_to_shape_mm: i64,
}

/// Result of one detector step: next state, optional event, optional non-terminal "why not".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Detected {
    pub state: Option<DetState>,
    pub event: Option<Event>,
    pub reason: Option<ReasonCode>,
}

/// `detect(st, sp, path, fix, now_ms, explain)`; state is explicit input and output.
///
/// # Errors
/// Skeleton: `NotImplemented`.
pub fn detect(
    _st: Option<DetState>,
    _sp: &Signpost,
    _path: &PathSummary,
    _fix: &Fix,
    _now_ms: i64,
    _explain: bool,
) -> Result<Detected, EngineError> {
    Err(EngineError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_is_plain_data() {
        let e = Event {
            kind: EventKind::Enter,
            t_ms: 1,
            d_rem_mm: None,
            t_hat_ms: None,
            dist_to_shape_mm: 0,
        };
        assert_eq!(e, e);
    }
}
