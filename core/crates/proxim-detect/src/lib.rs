//! ENTER with EXIT hysteresis and APPROACH time-to-anchor detectors (engine-spec 6).
//!
//! `no_std` + `alloc`, integers only. [`detect`] is a pure function of its arguments: the
//! detector state is an explicit input and output, there is no clock (`now_ms` is an argument) and
//! no randomness. Reserved events (`cross dwell pass sequence`, and the internal `exit`) fail
//! closed with `UNSUPPORTED_EVENT`.
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod approach;
mod enter;
pub mod params;
mod prep;

pub use prep::Prepared;

use proxim_path::PathSummary;
use proxim_types::{DetState, EngineError, EventKind, Fix, ReasonCode, Signpost};

/// A detector event (spec 6). It carries the path tier and confidence that produced it so that
/// `decide` can fill the `Decision` and score the candidate without re-deriving them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub t_ms: i64,
    pub d_rem_mm: Option<i64>,
    pub t_hat_ms: Option<i64>,
    /// Zone: distance outside the shape (0 for ENTER, which needs `Inside`); route: cross-track.
    pub dist_to_shape_mm: i64,
    /// 0 for presence (ENTER), 1 for a corridor (APPROACH); 2 and 3 are reserved.
    pub path_tier: u8,
    pub path_conf_milli: u16,
}

/// Result of one detector step: next state, optional event, optional "why not".
///
/// `state == None` means idle (the caller removes the entry). `reason` is non-terminal ("why not")
/// except `TOO_LATE_FOR_LEAD`, `UNSUPPORTED_EVENT` and `TRIGGER_INVALID`, which are terminal
/// drops; terminal reasons are always reported, the others only when `explain` is set. State and
/// event never depend on `explain`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Detected {
    pub state: Option<DetState>,
    pub event: Option<Event>,
    pub reason: Option<ReasonCode>,
}

impl Prepared {
    /// One detector step with a prepared trigger (spec 6): `(st, fix) -> (st', event, reason)`.
    ///
    /// `path` is the summary of `path_update` for this same fix. `now_ms` is only used for the
    /// APPROACH staleness gate. Time going backwards within a detector (`fix.t_ms` not after the
    /// state's `last_t`) leaves the state unchanged with `FIX_OUT_OF_ORDER`.
    ///
    /// # Errors
    /// [`EngineError`] only for arithmetic failures that valid input cannot cause; a shell treats
    /// it as "no decision, state unchanged".
    pub fn step(
        &self,
        st: Option<DetState>,
        path: &PathSummary,
        fix: &Fix,
        now_ms: i64,
        explain: bool,
    ) -> Result<Detected, EngineError> {
        match &self.kind {
            prep::Kind::Route(r) => {
                approach::step_approach(self, r, st, path, fix, now_ms, explain)
            }
            _ => enter::step_enter(self, st, path, fix, explain),
        }
    }
}

/// `detect(st, sp, path, fix, now_ms, explain) -> (st', event, reason)` (spec 6).
///
/// Convenience wrapper that validates and projects the trigger on every call; `decide` should
/// build a [`Prepared`] once per signpost revision. A defective or reserved signpost is a terminal
/// drop (`TRIGGER_INVALID` or `UNSUPPORTED_EVENT`) with the detector removed.
///
/// # Errors
/// As [`Prepared::step`].
pub fn detect(
    st: Option<DetState>,
    sp: &Signpost,
    path: &PathSummary,
    fix: &Fix,
    now_ms: i64,
    explain: bool,
) -> Result<Detected, EngineError> {
    match Prepared::new(sp) {
        Ok(p) => p.step(st, path, fix, now_ms, explain),
        Err(reason) => Ok(Detected {
            state: None,
            event: None,
            reason: Some(reason),
        }),
    }
}
