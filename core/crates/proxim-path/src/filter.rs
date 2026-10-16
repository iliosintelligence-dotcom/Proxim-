//! Speed and heading filters (engine-spec 5.1, 5.2) and the per-fix update `path_update`.
//!
//! Everything is integer: the EWMA weight is the rational `dt / (tau + dt)` in Q16, headings are
//! Q30 unit vectors, so no `exp`, `atan2` or float exists here. `Mem` is the only state.

use alloc::vec::Vec;
use proxim_geo::{acc_eff_mm, dist_mm, frame_for, isqrt, normalize_q30, rsr, unit_from_course};
use proxim_types::defaults::{
    CLOSING_MAX_SPAN_MS, CLOSING_MIN_SPAN_MS, COURSE_ACC_MAX_DDEG, DT_CLAMP_MS,
    HEADING_DISP_MAX_AGE_MS, HEADING_DISP_MIN_AGE_MS, HEADING_MIN_DISP_MM, HEADING_MIN_SPEED_MMPS,
    HEADING_TAU_MS, KAPPA_PERMILLE, MAX_TRACE, P0_CONF, P1_WINDOW_MS, RESEED_GAP_MS,
    SPEED_ACC_MAX_MMPS, SPEED_DISP_MAX_AGE_MS, SPEED_DISP_MIN_AGE_MS, SPEED_TAU_MS, STALE_HIDE_MS,
};
use proxim_types::{EngineError, Enu, Fix, Mem, Mode, PathInfo, Position, TraceFix};

/// Output of one `path_update` call.
///
/// `info` is the signpost-independent baseline (tier 0, confidence 0: no corridor has been
/// evaluated); tiers and confidences are per signpost and travel on the detector `Event`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathSummary {
    /// Output block `path` (spec 3.3): tier 0, confidence 0, visible `vf`, heading validity.
    pub info: PathInfo,
    /// `max(vf, vehicle floor)`. Per mode use [`v_eff_for`] with `vf_mmps`.
    pub v_eff_mmps: u32,
    /// Heading unit vector (east, north), Q30, only when valid (updated within 10 s).
    pub heading_q30: Option<[i32; 2]>,
    /// Filtered speed, absent when never measured or not refreshed for 10 s.
    pub vf_mmps: Option<u32>,
    /// Accepted fixes (including this one) with `t >= fix.t - 60 s`.
    pub fixes_60s: u8,
    /// The newest three accepted fixes (including this one) in time order; unused slots are the
    /// leading `None`s. Carries what the per-signpost corridor tests need, so `detect` needs no
    /// access to `Mem`.
    pub recent: [Option<TraceFix>; 3],
    /// CLOSING baseline (5.3, errata E-P12): the oldest retained trace fix that is at least 4 s
    /// and at most 60 s older than this fix, `None` when no fix qualifies (CLOSING is then false).
    pub closing_base: Option<TraceFix>,
    /// Time of the current fix, ms (the same as `recent[2].t_ms`).
    pub t_ms: i64,
}

impl PathSummary {
    /// The recent fixes, oldest first (1 to 3 entries).
    pub fn recent_fixes(&self) -> impl Iterator<Item = &TraceFix> {
        self.recent.iter().flatten()
    }
}

/// `v_eff = max(vf, floor(mode))` (5.1); a hidden `vf` counts as 0.
#[must_use]
pub fn v_eff_for(mode: Mode, vf_mmps: Option<u32>) -> u32 {
    vf_mmps.unwrap_or(0).max(floor_mmps(mode))
}

/// Speed floor of a mode (5.1).
#[must_use]
pub const fn floor_mmps(mode: Mode) -> u32 {
    match mode {
        Mode::Vehicle => proxim_types::defaults::FLOOR_VEHICLE_MMPS,
        Mode::Cycle => proxim_types::defaults::FLOOR_CYCLE_MMPS,
        Mode::Foot => proxim_types::defaults::FLOOR_FOOT_MMPS,
    }
}

/// Minimum filtered speed for tier P1 of a mode (5.3).
#[must_use]
pub const fn min_speed_mmps(mode: Mode) -> u32 {
    match mode {
        Mode::Vehicle => proxim_types::defaults::MIN_VEHICLE_MMPS,
        Mode::Cycle => proxim_types::defaults::MIN_CYCLE_MMPS,
        Mode::Foot => proxim_types::defaults::MIN_FOOT_MMPS,
    }
}

/// Displacement from a trace entry to a fix in the local frame centred on the older entry.
/// `None` when the frame cannot be built or the displacement leaves it (fail closed: no
/// measurement).
fn displacement(from: &TraceFix, to: &Fix) -> Option<Enu> {
    let frame = frame_for(Position {
        lat_e7: from.lat_e7,
        lon_e7: from.lon_e7,
    })
    .ok()?;
    frame.project(to.position()).ok()
}

/// Newest trace entry whose age relative to `now` lies in `min_age..=max_age`.
fn newest_aged(trace: &[TraceFix], now: i64, min_age: i64, max_age: i64) -> Option<&TraceFix> {
    trace.iter().rev().find(|e| {
        let age = now.saturating_sub(e.t_ms);
        (min_age..=max_age).contains(&age)
    })
}

/// The CLOSING baseline (5.3, errata E-P12): the oldest retained fix whose age relative to `now` is
/// within `CLOSING_MIN_SPAN_MS..=CLOSING_MAX_SPAN_MS`. `trace` is oldest first and may contain the
/// current fix (age 0, never qualifies). Public so the simulator can state the rule.
#[must_use]
pub fn closing_baseline(trace: &[TraceFix], now: i64) -> Option<TraceFix> {
    trace
        .iter()
        .find(|e| (CLOSING_MIN_SPAN_MS..=CLOSING_MAX_SPAN_MS).contains(&now.saturating_sub(e.t_ms)))
        .copied()
}

/// `v_meas` (5.1): Doppler if trusted, else distance over the newest 3..10 s old trace entry.
fn measure_speed(mem: &Mem, fix: &Fix) -> Option<u32> {
    if let Some(v) = fix.speed_mmps
        && fix.speed_acc_mmps.is_none_or(|a| a <= SPEED_ACC_MAX_MMPS)
    {
        return Some(v);
    }
    let e = newest_aged(
        &mem.trace,
        fix.t_ms,
        SPEED_DISP_MIN_AGE_MS,
        SPEED_DISP_MAX_AGE_MS,
    )?;
    let d = dist_mm(Enu { e_mm: 0, n_mm: 0 }, displacement(e, fix)?).ok()?;
    let age = fix.t_ms.saturating_sub(e.t_ms);
    // age >= 3000 here, so no division by zero; floor division.
    let v = i128::from(d).checked_mul(1000)? / i128::from(age);
    u32::try_from(v).ok()
}

/// Median of up to three values: `sorted[(n - 1) / 2]`.
fn median3(vals: &[u32]) -> Option<u32> {
    let mut v: Vec<u32> = vals.iter().copied().take(3).collect();
    v.sort_unstable();
    v.get(v.len().checked_sub(1)? / 2).copied()
}

/// Rational EWMA step of the speed filter (5.1).
fn speed_filter(mem: &Mem, fix: &Fix, v_meas: Option<u32>) -> Result<Option<u32>, EngineError> {
    let Some(v) = v_meas else {
        return Ok(mem.vf_mmps); // unchanged; hiding is decided by the caller
    };
    let dt = mem
        .last_t_ms
        .map_or(0, |l| fix.t_ms.saturating_sub(l))
        .clamp(0, DT_CLAMP_MS);
    // A speed that has not been refreshed for 10 s is hidden (5.1); it must not weigh into a new
    // measurement, so it counts as absent here (conservative reading, see the errata).
    let fresh_before = mem
        .trace
        .iter()
        .rev()
        .find(|e| e.v_mmps.is_some())
        .is_some_and(|e| fix.t_ms.saturating_sub(e.t_ms) <= STALE_HIDE_MS);
    let Some(vf) = mem.vf_mmps.filter(|_| fresh_before) else {
        return Ok(Some(v));
    };
    if dt > RESEED_GAP_MS {
        // After a gap: median of this measurement and the (at most two) newest earlier
        // measurements that are still no older than 10 s. A gap above 10 s leaves just `v`.
        let mut vals: Vec<u32> = Vec::with_capacity(3);
        vals.push(v);
        vals.extend(
            mem.trace
                .iter()
                .rev()
                .filter(|e| fix.t_ms.saturating_sub(e.t_ms) <= STALE_HIDE_MS)
                .filter_map(|e| e.v_mmps)
                .take(2),
        );
        return Ok(Some(median3(&vals).unwrap_or(v)));
    }
    let alpha = dt
        .checked_mul(65_536)
        .ok_or(EngineError::Overflow)?
        .checked_div(SPEED_TAU_MS + dt)
        .ok_or(EngineError::Overflow)?;
    let delta = (i128::from(v) - i128::from(vf)) * i128::from(alpha);
    let next = i128::from(vf) + rsr(delta, 16)?;
    Ok(Some(
        u32::try_from(next).map_err(|_| EngineError::Overflow)?,
    ))
}

/// Heading measurement `m` (5.2): OS course vector, else displacement unit vector.
fn measure_heading(
    mem: &Mem,
    fix: &Fix,
    vf_visible: Option<u32>,
    acc_eff: i64,
) -> Option<[i32; 2]> {
    if let Some(c) = fix.course_ddeg
        && fix.course_acc_ddeg.is_none_or(|a| a <= COURSE_ACC_MAX_DDEG)
        && vf_visible.is_some_and(|v| v >= HEADING_MIN_SPEED_MMPS)
        && let Ok(u) = unit_from_course(c)
    {
        return Some(u);
    }
    let e = newest_aged(
        &mem.trace,
        fix.t_ms,
        HEADING_DISP_MIN_AGE_MS,
        HEADING_DISP_MAX_AGE_MS,
    )?;
    let d = displacement(e, fix)?;
    let n2 = i128::from(d.e_mm) * i128::from(d.e_mm) + i128::from(d.n_mm) * i128::from(d.n_mm);
    let need = HEADING_MIN_DISP_MM.max(acc_eff.saturating_mul(3));
    let n = isqrt(n2.unsigned_abs());
    if i128::from(n) < i128::from(need) {
        return None;
    }
    normalize_q30(i64::from(d.e_mm), i64::from(d.n_mm))
}

/// Heading filter (5.2). Returns `(h, h_t)`.
fn heading_filter(
    mem: &Mem,
    fix: &Fix,
    m: Option<[i32; 2]>,
) -> Result<(Option<[i32; 2]>, Option<i64>), EngineError> {
    let Some(m) = m else {
        return Ok((mem.h, mem.h_t_ms));
    };
    let (Some(h), Some(h_t)) = (mem.h, mem.h_t_ms) else {
        return Ok((Some(m), Some(fix.t_ms)));
    };
    let dt = fix.t_ms.saturating_sub(h_t).clamp(0, DT_CLAMP_MS);
    if fix.t_ms.saturating_sub(h_t) > RESEED_GAP_MS {
        return Ok((Some(m), Some(fix.t_ms)));
    }
    let a = dt
        .checked_mul(65_536)
        .ok_or(EngineError::Overflow)?
        .checked_div(HEADING_TAU_MS + dt)
        .ok_or(EngineError::Overflow)?;
    let wa = i128::from(a);
    let wh = i128::from(65_536_i64) - wa;
    let bx = rsr(i128::from(h[0]) * wh + i128::from(m[0]) * wa, 16)?;
    let by = rsr(i128::from(h[1]) * wh + i128::from(m[1]) * wa, 16)?;
    let blended = match (i64::try_from(bx), i64::try_from(by)) {
        (Ok(x), Ok(y)) => normalize_q30(x, y),
        _ => None,
    };
    Ok((Some(blended.unwrap_or(m)), Some(fix.t_ms)))
}

/// Update filters and trace with an accepted fix (spec 5).
///
/// "Accepted" means `decide` has already applied F03 to F10. This function is still total: an
/// out-of-contract fix is an error, never a panic or a corrupted `Mem`.
///
/// # Errors
/// [`EngineError::Parse`] if `fix.t_ms` is not strictly after `mem.last_t_ms` (the caller maps
/// that to F08); [`EngineError::Overflow`] on arithmetic overflow (unreachable for valid input).
pub fn path_update(mem: &Mem, fix: &Fix) -> Result<(Mem, PathSummary), EngineError> {
    if mem.last_t_ms.is_some_and(|l| fix.t_ms <= l) {
        return Err(EngineError::Parse);
    }
    let acc_eff = acc_eff_mm(fix.acc_mm, KAPPA_PERMILLE)?;

    let v_meas = measure_speed(mem, fix);
    let vf = speed_filter(mem, fix, v_meas)?;
    // The speed is visible while a measurement is at most 10 s old.
    let last_v_t = if v_meas.is_some() {
        Some(fix.t_ms)
    } else {
        mem.trace
            .iter()
            .rev()
            .find(|e| e.v_mmps.is_some())
            .map(|e| e.t_ms)
    };
    let vf_visible =
        vf.filter(|_| last_v_t.is_some_and(|t| fix.t_ms.saturating_sub(t) <= STALE_HIDE_MS));

    let m = measure_heading(mem, fix, vf_visible, acc_eff);
    let (h, h_t_ms) = heading_filter(mem, fix, m)?;

    let mut next = mem.clone();
    next.last_t_ms = Some(fix.t_ms);
    next.vf_mmps = vf;
    next.h = h;
    next.h_t_ms = h_t_ms;
    next.trace.push(TraceFix {
        t_ms: fix.t_ms,
        lat_e7: fix.lat_e7,
        lon_e7: fix.lon_e7,
        acc_mm: fix.acc_mm,
        v_mmps: v_meas,
    });
    while next.trace.len() > MAX_TRACE {
        next.trace.remove(0);
    }

    let heading_valid = h_t_ms.is_some_and(|t| fix.t_ms.saturating_sub(t) <= STALE_HIDE_MS);
    let heading_q30 = h.filter(|_| heading_valid);
    let fixes_60s = u8::try_from(
        next.trace
            .iter()
            .filter(|e| fix.t_ms.saturating_sub(e.t_ms) <= P1_WINDOW_MS)
            .count(),
    )
    .unwrap_or(u8::MAX);
    let mut recent: [Option<TraceFix>; 3] = [None; 3];
    for (slot, e) in recent.iter_mut().rev().zip(next.trace.iter().rev()) {
        *slot = Some(*e);
    }
    let closing_base = closing_baseline(&next.trace, fix.t_ms);
    let summary = PathSummary {
        info: PathInfo {
            tier: 0,
            conf_milli: 0,
            speed_mmps: vf_visible,
            heading_valid,
        },
        v_eff_mmps: v_eff_for(Mode::Vehicle, vf_visible),
        heading_q30,
        vf_mmps: vf_visible,
        fixes_60s,
        recent,
        closing_base,
        t_ms: fix.t_ms,
    };
    Ok((next, summary))
}

/// Confidence of tier P0 (presence, confident `Inside`), permille.
#[must_use]
pub const fn p0_conf_milli() -> u16 {
    P0_CONF
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tf(t: i64, v: Option<u32>) -> TraceFix {
        TraceFix {
            t_ms: t,
            lat_e7: 450_000_000,
            lon_e7: 70_000_000,
            acc_mm: 5000,
            v_mmps: v,
        }
    }

    #[test]
    fn median3_is_the_lower_median() {
        assert_eq!(median3(&[]), None);
        assert_eq!(median3(&[7]), Some(7));
        assert_eq!(median3(&[9, 3]), Some(3), "n = 2 takes sorted[0]");
        assert_eq!(median3(&[9, 3, 5]), Some(5));
        assert_eq!(median3(&[1, 1, 9]), Some(1));
        assert_eq!(
            median3(&[5, 4, 3, 2, 1]),
            Some(4),
            "only the first three values count"
        );
    }

    #[test]
    fn newest_aged_picks_the_newest_entry_in_range() {
        let tr = [
            tf(0, None),
            tf(4000, None),
            tf(8000, None),
            tf(10_000, None),
        ];
        // now = 12000: ages 12000, 8000, 4000, 2000 -> newest in 3000..=10000 is t = 8000.
        assert_eq!(
            newest_aged(&tr, 12_000, 3000, 10_000).map(|e| e.t_ms),
            Some(8000)
        );
        // Bounds are inclusive.
        assert_eq!(
            newest_aged(&tr, 13_000, 3000, 10_000).map(|e| e.t_ms),
            Some(10_000)
        );
        // Inclusive bounds on both ends, single entry.
        let one = [tf(0, None)];
        assert_eq!(
            newest_aged(&one, 10_000, 3000, 10_000).map(|e| e.t_ms),
            Some(0)
        );
        assert_eq!(
            newest_aged(&one, 10_001, 3000, 10_000).map(|e| e.t_ms),
            None
        );
        assert_eq!(
            newest_aged(&one, 3000, 3000, 10_000).map(|e| e.t_ms),
            Some(0)
        );
        assert_eq!(newest_aged(&one, 2999, 3000, 10_000).map(|e| e.t_ms), None);
        assert_eq!(
            newest_aged(&tr, 10_000, 3000, 10_000).map(|e| e.t_ms),
            Some(4000)
        );
        assert_eq!(
            newest_aged(&tr, 10_001, 3000, 10_000).map(|e| e.t_ms),
            Some(4000)
        );
        assert_eq!(newest_aged(&tr, 2999, 3000, 10_000).map(|e| e.t_ms), None);
        assert_eq!(newest_aged(&[], 0, 0, 10).map(|e| e.t_ms), None);
        // Saturating ages never panic.
        assert_eq!(newest_aged(&tr, i64::MIN, 0, 10).map(|e| e.t_ms), None);
    }

    #[test]
    fn displacement_is_none_for_unframeable_origins() {
        let mut polar = tf(0, None);
        polar.lat_e7 = 850_000_000;
        let fix = Fix {
            t_ms: 5000,
            lat_e7: 850_000_000,
            lon_e7: 70_000_000,
            acc_mm: 5000,
            speed_mmps: None,
            speed_acc_mmps: None,
            course_ddeg: None,
            course_acc_ddeg: None,
            is_mock: false,
        };
        assert_eq!(
            displacement(&polar, &fix),
            None,
            "latitude above 80 degrees: no measurement"
        );
        let mem = Mem {
            trace: alloc::vec![polar],
            ..Mem::default()
        };
        assert_eq!(measure_speed(&mem, &fix), None);
    }

    #[test]
    fn speed_filter_leaves_vf_alone_without_a_measurement() {
        let mem = Mem {
            vf_mmps: Some(1234),
            last_t_ms: Some(0),
            ..Mem::default()
        };
        let fix = Fix {
            t_ms: 1000,
            lat_e7: 0,
            lon_e7: 0,
            acc_mm: 5000,
            speed_mmps: None,
            speed_acc_mmps: None,
            course_ddeg: None,
            course_acc_ddeg: None,
            is_mock: false,
        };
        assert_eq!(speed_filter(&mem, &fix, None), Ok(Some(1234)));
    }

    #[test]
    fn closing_baseline_is_the_oldest_fix_aged_4_to_60_s() {
        let tr = [tf(0, None), tf(1000, None), tf(5000, None), tf(6000, None)];
        // now = 6000: ages 6000, 5000, 1000, 0 -> the oldest eligible is t = 0.
        assert_eq!(closing_baseline(&tr, 6000).map(|e| e.t_ms), Some(0));
        // now = 5000: ages 5000, 4000, 0, -1000 -> t = 0 is still the oldest eligible.
        assert_eq!(closing_baseline(&tr, 5000).map(|e| e.t_ms), Some(0));
        // now = 4999: t = 0 is 4999 old (eligible); now = 3999: nothing is old enough.
        assert_eq!(closing_baseline(&tr, 4999).map(|e| e.t_ms), Some(0));
        assert_eq!(closing_baseline(&tr, 3999).map(|e| e.t_ms), None);
        assert_eq!(closing_baseline(&tr, 4000).map(|e| e.t_ms), Some(0));
        // The cap: now = 60_000 keeps t = 0 (age 60_000), now = 60_001 drops it for t = 1000.
        assert_eq!(closing_baseline(&tr, 60_000).map(|e| e.t_ms), Some(0));
        assert_eq!(closing_baseline(&tr, 60_001).map(|e| e.t_ms), Some(1000));
        assert_eq!(closing_baseline(&tr, 66_000).map(|e| e.t_ms), Some(6000));
        assert_eq!(closing_baseline(&tr, 66_001).map(|e| e.t_ms), None);
        // Degenerate inputs never panic.
        assert_eq!(closing_baseline(&[], 0).map(|e| e.t_ms), None);
        assert_eq!(closing_baseline(&tr, i64::MIN).map(|e| e.t_ms), None);
        assert_eq!(closing_baseline(&tr, i64::MAX).map(|e| e.t_ms), None);
    }

    #[test]
    fn p0_confidence_is_500() {
        assert_eq!(p0_conf_milli(), 500);
    }
}
