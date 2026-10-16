//! Validation of untrusted inputs that are not part of the per-signpost drop logic.
//! A defect here is an `Err` (whole call is "no decision, state unchanged").

use crate::params::TZ_MAX_MIN;
use proxim_types::defaults::{MAX_DETECTORS, MAX_SIGNPOSTS, MAX_TRACE};
use proxim_types::{Clock, EngineError, Fix, State};

/// Largest magnitude of a Q30 unit-vector component.
const Q30_ONE: i32 = 1 << 30;

/// Structural validity of a `State` handed in by a shell.
///
/// # Errors
/// [`EngineError::Parse`] for a wrong version, unsorted or duplicate ids, a non-increasing trace
/// or a heading vector outside the unit range; [`EngineError::Limit`] for oversize lists.
pub(crate) fn state(s: &State) -> Result<(), EngineError> {
    if s.v != 1 || s.caps.hwm_ms < 0 {
        return Err(EngineError::Parse);
    }
    if s.caps.signposts.len() > MAX_SIGNPOSTS || s.mem.det.len() > MAX_DETECTORS {
        return Err(EngineError::Limit);
    }
    if s.mem.trace.len() > MAX_TRACE {
        return Err(EngineError::Limit);
    }
    if !s
        .caps
        .signposts
        .windows(2)
        .all(|w| matches!(w, [a, b] if a.id < b.id))
    {
        return Err(EngineError::Parse);
    }
    if !s
        .mem
        .det
        .windows(2)
        .all(|w| matches!(w, [a, b] if a.id < b.id))
    {
        return Err(EngineError::Parse);
    }
    if !s
        .mem
        .trace
        .windows(2)
        .all(|w| matches!(w, [a, b] if a.t_ms < b.t_ms))
    {
        return Err(EngineError::Parse);
    }
    if let (Some(last), Some(newest)) = (s.mem.last_t_ms, s.mem.trace.last())
        && last < newest.t_ms
    {
        return Err(EngineError::Parse);
    }
    if s.mem.jump_rejects >= crate::params::JUMP_RESET_AT {
        return Err(EngineError::Parse);
    }
    if let Some(h) = s.mem.h
        && h.iter().any(|c| c.unsigned_abs() > Q30_ONE.unsigned_abs())
    {
        return Err(EngineError::Parse);
    }
    Ok(())
}

/// `now_ms >= 0` and a plausible timezone offset.
///
/// # Errors
/// [`EngineError::Parse`].
pub(crate) fn clock(c: &Clock) -> Result<(), EngineError> {
    if c.now_ms < 0 || c.tz_offset_min.unsigned_abs() > TZ_MAX_MIN.unsigned_abs() {
        return Err(EngineError::Parse);
    }
    Ok(())
}

/// F03 `FIX_INVALID` (range): coordinates, course, non-negative time.
#[must_use]
pub(crate) fn fix_is_invalid(f: &Fix) -> bool {
    f.lat_e7.unsigned_abs() > 900_000_000
        || f.lon_e7.unsigned_abs() > 1_800_000_000
        || f.course_ddeg.is_some_and(|c| c >= 3600)
        || f.t_ms < 0
}
