//! Corridor evidence for tier P1 (engine-spec 5.3): projection of the last three fixes onto a
//! route, the CLOSING test and the confidence ladder. Pure functions of their arguments.

use crate::filter::{PathSummary, min_speed_mmps};
use alloc::vec::Vec;
use proxim_geo::{
    Frame, Polyline, Projection, acc_eff_mm, cos_tol_q30, dot_q30, project_to_polyline,
};
use proxim_types::defaults::{
    CLOSING_MAX_SPAN_MS, CLOSING_MIN_ADVANCE_MM, CLOSING_MIN_SPAN_MS, KAPPA_PERMILLE, P0_CONF,
    P1_ACC_EFF_MAX_MM, P1_ACC_PENALTY1_MM, P1_ACC_PENALTY2_MM, P1_BONUS_FIXES,
    P1_BONUS_HEADING_TOL_DDEG, P1_CONF_BASE, P1_CONF_BONUS, P1_CONF_FOOT_CAP, P1_CONF_PENALTY1,
    P1_CONF_PENALTY2, P1_HEADING_TOL_DDEG, P1_MIN_FIXES, WINDOW_AHEAD_MM, WINDOW_AHEAD_S,
    WINDOW_BACK_MM,
};
use proxim_types::{Copy as AdCopy, EngineError, Mode, Position, TraceFix};

/// One recent fix projected onto a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recent {
    pub t_ms: i64,
    pub along_mm: i64,
    /// Signed cross-track (left positive); `abs` is the distance to the route.
    pub xt_mm: i64,
    pub acc_eff_mm: i64,
}

/// The last up to three fixes projected onto a route, oldest first, the CLOSING baseline, and the
/// full projection of the current fix (the last entry of `items`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentProjection {
    pub items: Vec<Recent>,
    /// The CLOSING baseline fix projected onto the route (`PathSummary::closing_base`), if any.
    pub base: Option<Recent>,
    pub cur: Projection,
}

/// Project one fix onto the route inside `window`, falling back to the whole route when the window
/// overlaps no segment (E-G5). A window never replaces an `Err` other than that one.
fn project_in(
    frame: &Frame,
    route: &Polyline,
    f: &TraceFix,
    window: Option<(i64, i64)>,
) -> Result<(Projection, Recent), EngineError> {
    let p = frame.project(Position {
        lat_e7: f.lat_e7,
        lon_e7: f.lon_e7,
    })?;
    let proj = match project_to_polyline(route, p, window) {
        Err(EngineError::DegenerateGeometry) if window.is_some() => {
            project_to_polyline(route, p, None)?
        }
        other => other?,
    };
    let recent = Recent {
        t_ms: f.t_ms,
        along_mm: proj.along_mm,
        xt_mm: proj.xt_mm,
        acc_eff_mm: acc_eff_mm(f.acc_mm, KAPPA_PERMILLE)?,
    };
    Ok((proj, recent))
}

/// Project the newest three fixes of `summary`, and the CLOSING baseline, onto `route` (spec 6.2).
///
/// `a_prev_mm` is the along-route position of the previous accepted fix as stored in
/// `DetState::Armed` (errata E-P13). Without it (idle, or an armed state that carries none) every
/// fix is projected onto the whole route independently. With it, the current fix is projected onto
/// the exact window `[a_prev - 100 m, a_prev + 500 m + v_eff x 30 s]`, which keeps a parallel leg
/// or the other half of a loop from capturing the track. The older fixes (and the baseline) are
/// history rather than the fix being decided: they use the same window with the back margin
/// widened by the distance `v_eff` covers in their age, so a long sampling gap does not push
/// them out of it. A window that overlaps no segment falls back to the whole route.
///
/// # Errors
/// [`EngineError::OutOfFrame`] if a fix is more than 150 km from the frame origin;
/// [`EngineError::Overflow`].
pub fn project_recent(
    summary: &PathSummary,
    frame: &Frame,
    route: &Polyline,
    a_prev_mm: Option<i64>,
    v_eff_mmps: u32,
) -> Result<RecentProjection, EngineError> {
    let ahead = WINDOW_AHEAD_MM
        .checked_add(i64::from(v_eff_mmps) * WINDOW_AHEAD_S)
        .ok_or(EngineError::Overflow)?;
    // Window for a fix of the given age (ms before the current fix).
    let window_for = |age_ms: i64| -> Option<(i64, i64)> {
        a_prev_mm.map(|a| {
            let extra = i64::try_from(i128::from(v_eff_mmps) * i128::from(age_ms.max(0)) / 1000)
                .unwrap_or(i64::MAX);
            (
                a.saturating_sub(WINDOW_BACK_MM).saturating_sub(extra),
                a.saturating_add(ahead),
            )
        })
    };
    let mut items: Vec<Recent> = Vec::with_capacity(3);
    let mut cur: Option<Projection> = None;
    for f in summary.recent_fixes() {
        let age = summary.t_ms.saturating_sub(f.t_ms);
        let (proj, recent) = project_in(frame, route, f, window_for(age))?;
        items.push(recent);
        cur = Some(proj);
    }
    let base = match summary.closing_base.as_ref() {
        Some(b) => {
            let age = summary.t_ms.saturating_sub(b.t_ms);
            Some(project_in(frame, route, b, window_for(age))?.1)
        }
        None => None,
    };
    Ok(RecentProjection {
        items,
        base,
        cur: cur.ok_or(EngineError::DegenerateGeometry)?,
    })
}

/// CLOSING (5.3, errata E-P12), a TIME-SPAN rule so that a walker can satisfy it.
///
/// `recent` are the newest fixes oldest first (the last is the current fix); `base` is the oldest
/// retained fix that is at least `CLOSING_MIN_SPAN_MS` older than the current one. With
/// `tol = max(acc_eff over recent and base) / 2` all of these must hold:
/// - `base` exists and the span `cur.t - base.t` is within 4 s..=60 s;
/// - the net advance `cur.along - base.along >= max(5 m, tol)`;
/// - the last step does not retreat: `cur.along - prev.along >= -tol` (kept from the three-fix
///   rule so that a U-turn is caught on its first fix instead of after the baseline catches up).
///
/// Fewer than two recent fixes, or no baseline, is not closing. U-turn, opposite carriageway and a
/// stopped vehicle fail. A stationary device whose fixes scatter by less than `acc_eff / 4` around
/// its true position never passes (two such fixes differ by less than `acc_eff / 2`).
#[must_use]
pub fn closing(recent: &[Recent], base: Option<&Recent>) -> bool {
    let (Some(base), [.., prev, cur]) = (base, recent) else {
        return false;
    };
    let span = cur.t_ms.saturating_sub(base.t_ms);
    if !(CLOSING_MIN_SPAN_MS..=CLOSING_MAX_SPAN_MS).contains(&span) {
        return false;
    }
    let tol = recent
        .iter()
        .map(|r| r.acc_eff_mm)
        .chain(core::iter::once(base.acc_eff_mm))
        .max()
        .unwrap_or(i64::MAX)
        / 2;
    let net = cur.along_mm.saturating_sub(base.along_mm);
    let step = cur.along_mm.saturating_sub(prev.along_mm);
    net >= CLOSING_MIN_ADVANCE_MM.max(tol) && step >= tol.saturating_neg()
}

/// Evidence for tier P1 (5.3).
#[derive(Clone, Copy, Debug)]
pub struct P1Evidence<'a> {
    pub recent: &'a [Recent],
    /// The projected CLOSING baseline (`RecentProjection::base`).
    pub base: Option<&'a Recent>,
    /// Accepted fixes within 60 s.
    pub fixes_60s: u8,
    /// Visible filtered speed.
    pub vf_mmps: Option<u32>,
    pub mode: Mode,
    /// `dot(h, tangent)` in Q30; `None` when the heading is invalid.
    pub heading_dot_q30: Option<i64>,
    pub buffer_mm: i64,
}

/// Confidence (permille) of tier P1 if every criterion holds, else `None` (5.3): `acc_eff <= 65 m`
/// and `|xt| <= buffer` on the last three fixes, at least three fixes within 60 s, `vf` at least
/// the mode minimum, heading valid and within 45 degrees of the tangent, and CLOSING.
/// `600 + 100 [>= 5 fixes in 60 s] + 100 [within 15 degrees] - 100 [worst acc_eff > 30 m]
/// - 200 [> 50 m]`, capped at 600 for foot.
///
/// # Errors
/// Only if a tolerance table lookup fails (unreachable for the constants used).
pub fn p1_confidence(ev: &P1Evidence<'_>) -> Result<Option<u16>, EngineError> {
    if ev.recent.len() != 3 || ev.fixes_60s < P1_MIN_FIXES {
        return Ok(None);
    }
    let worst = ev
        .recent
        .iter()
        .map(|r| r.acc_eff_mm)
        .max()
        .unwrap_or(i64::MAX);
    if worst > P1_ACC_EFF_MAX_MM {
        return Ok(None);
    }
    if ev
        .recent
        .iter()
        .any(|r| r.xt_mm.saturating_abs() > ev.buffer_mm)
    {
        return Ok(None);
    }
    if !ev.vf_mmps.is_some_and(|v| v >= min_speed_mmps(ev.mode)) {
        return Ok(None);
    }
    let Some(dot) = ev.heading_dot_q30 else {
        return Ok(None);
    };
    if dot < i64::from(cos_tol_q30(P1_HEADING_TOL_DDEG)?) || !closing(ev.recent, ev.base) {
        return Ok(None);
    }
    let mut conf = P1_CONF_BASE;
    if ev.fixes_60s >= P1_BONUS_FIXES {
        conf += P1_CONF_BONUS;
    }
    if dot >= i64::from(cos_tol_q30(P1_BONUS_HEADING_TOL_DDEG)?) {
        conf += P1_CONF_BONUS;
    }
    if worst > P1_ACC_PENALTY2_MM {
        conf -= P1_CONF_PENALTY2;
    } else if worst > P1_ACC_PENALTY1_MM {
        conf -= P1_CONF_PENALTY1;
    }
    if ev.mode == Mode::Foot {
        conf = conf.min(P1_CONF_FOOT_CAP);
    }
    Ok(Some(conf))
}

/// `dot(h, tangent)` in Q30, or `None` for an invalid heading.
#[must_use]
pub fn heading_dot(heading_q30: Option<[i32; 2]>, tangent_q30: [i32; 2]) -> Option<i64> {
    heading_q30.map(|h| dot_q30(h, tangent_q30))
}

/// Copy ladder (5.3): P0 `near_you`, P1 `ahead`, P2 `on_route` (reserved).
#[must_use]
pub const fn copy_for_tier(tier: u8) -> AdCopy {
    match tier {
        0 => AdCopy::NearYou,
        1 => AdCopy::Ahead,
        _ => AdCopy::OnRoute,
    }
}

/// Confidence of tier P0 (presence), permille.
pub const P0_CONFIDENCE_MILLI: u16 = P0_CONF;

#[cfg(test)]
mod tests {
    use super::*;

    /// A projected fix at time `t` ms, `along` mm, accuracy `acc` mm (already `acc_eff`).
    fn r(t: i64, along: i64, acc: i64) -> Recent {
        Recent {
            t_ms: t,
            along_mm: along,
            xt_mm: 0,
            acc_eff_mm: acc,
        }
    }

    const A: i64 = 10_000; // acc_eff 10 m: tol 5 m, threshold max(5 m, 5 m) = 5 m

    #[test]
    fn closing_accepts_a_steady_advance_over_the_span() {
        let base = r(0, 0, A);
        // 15 m/s at 1 Hz: base 4 s old, 60 m net.
        assert!(closing(
            &[r(2000, 30_000, A), r(3000, 45_000, A), r(4000, 60_000, A)],
            Some(&base)
        ));
    }

    #[test]
    fn closing_span_bounds_are_exact() {
        let net = |t: i64| {
            let base = r(0, 0, A);
            closing(&[r(t - 1000, 9000, A), r(t, 10_000, A)], Some(&base))
        };
        assert!(!net(3999), "3.999 s is too short");
        assert!(net(4000), "4 s passes");
        assert!(net(60_000), "60 s passes");
        assert!(!net(60_001), "a baseline older than 60 s is no evidence");
        assert!(!net(0));
        assert!(!net(-5000), "a baseline from the future");
    }

    #[test]
    fn closing_net_advance_threshold_is_exact() {
        let c = |along: i64, acc: i64| {
            let b = r(0, 0, acc);
            closing(&[r(4500, along - 1, acc), r(5000, along, acc)], Some(&b))
        };
        // acc_eff 10 m: tol 5 m, threshold 5000.
        assert!(c(5000, A));
        assert!(!c(4999, A));
        // acc_eff 40 m: tol 20 m.
        assert!(c(20_000, 40_000));
        assert!(!c(19_999, 40_000));
        // The 5 m floor never binds because acc_eff >= 10 m, but the constant is honoured: a
        // smaller accuracy value (callers never produce one) still needs 5 m.
        assert!(!c(4999, 2000));
        assert!(c(5000, 2000));
    }

    #[test]
    fn closing_tolerance_uses_the_worst_accuracy_of_recent_and_baseline() {
        // Net 12 m: passes at tol 5 m, fails at tol 20 m wherever the poor fix sits.
        let good = |acc_base, acc_prev, acc_cur| {
            let b = r(0, 0, acc_base);
            closing(
                &[r(4000, 11_000, acc_prev), r(5000, 12_000, acc_cur)],
                Some(&b),
            )
        };
        assert!(good(A, A, A));
        assert!(!good(40_000, A, A), "baseline accuracy counts");
        assert!(!good(A, 40_000, A), "previous fix accuracy counts");
        assert!(!good(A, A, 40_000), "current fix accuracy counts");
        // An older recent fix counts too (three recent fixes).
        let b = r(0, 0, A);
        assert!(!closing(
            &[
                r(3000, 10_000, 40_000),
                r(4000, 11_000, A),
                r(5000, 12_000, A)
            ],
            Some(&b)
        ));
    }

    #[test]
    fn closing_last_step_must_not_retreat_beyond_tol() {
        let b = r(0, 0, A);
        // Net 20 m over the span, last step -5 m (= -tol) passes, -5.001 m fails.
        assert!(closing(&[r(4000, 25_000, A), r(5000, 20_000, A)], Some(&b)));
        assert!(!closing(
            &[r(4000, 25_001, A), r(5000, 20_000, A)],
            Some(&b)
        ));
    }

    #[test]
    fn closing_rejects_u_turn_stopped_missing_inputs() {
        let b = r(0, 0, A);
        // U-turn: far ahead of the baseline, but now going back at 15 m/s.
        assert!(!closing(
            &[r(4000, 60_000, A), r(5000, 45_000, A)],
            Some(&b)
        ));
        // Stopped: no net advance.
        assert!(!closing(&[r(4000, 0, A), r(5000, 0, A)], Some(&b)));
        // Driving away: negative net.
        assert!(!closing(
            &[r(4000, -15_000, A), r(5000, -30_000, A)],
            Some(&b)
        ));
        // No baseline, no recent fixes, a single recent fix.
        assert!(!closing(&[r(4000, 0, A), r(5000, 60_000, A)], None));
        assert!(!closing(&[], Some(&b)));
        assert!(!closing(&[r(5000, 60_000, A)], Some(&b)));
        assert!(!closing(&[], None));
    }

    #[test]
    fn closing_walker_gains_enough_over_four_seconds_only_at_low_noise() {
        // 1.4 m/s for 4 s is 5.6 m: passes at acc_eff 10 m (5 m), not at 12 m (6 m).
        let c = |acc: i64| {
            let b = r(0, 0, acc);
            closing(&[r(3000, 4200, acc), r(4000, 5600, acc)], Some(&b))
        };
        assert!(c(10_000));
        assert!(c(11_200));
        assert!(!c(11_202));
    }

    #[test]
    fn closing_jitter_bound_is_tight() {
        // Scatter of +-J around a standing point: two fixes differ by up to 2 J. With acc_eff 20 m
        // the threshold is 10 m, so J < 5 m (acc_eff / 4) can never pass and J = 5 m can.
        let acc = 20_000;
        let pass = |j: i64| {
            let b = r(0, -j, acc);
            closing(&[r(4000, -j, acc), r(5000, j, acc)], Some(&b))
        };
        assert!(!pass(4999), "scatter below acc_eff / 4 never closes");
        assert!(pass(5000), "the bound is exact: +-acc_eff / 4 already can");
    }

    #[test]
    fn closing_never_overflows() {
        let b = r(i64::MIN, i64::MIN, i64::MAX);
        assert!(!closing(&[r(0, 0, 0), r(i64::MAX, i64::MAX, 0)], Some(&b)));
        let b = r(0, i64::MIN, 0);
        let _ = closing(&[r(0, 0, 0), r(5000, i64::MAX, 0)], Some(&b));
        let b = r(i64::MIN, 0, 0);
        let _ = closing(
            &[r(i64::MIN, 0, 0), r(i64::MAX, i64::MIN, i64::MIN)],
            Some(&b),
        );
    }

    #[test]
    fn copy_ladder() {
        assert_eq!(copy_for_tier(0), AdCopy::NearYou);
        assert_eq!(copy_for_tier(1), AdCopy::Ahead);
        assert_eq!(copy_for_tier(2), AdCopy::OnRoute);
        assert_eq!(copy_for_tier(200), AdCopy::OnRoute);
    }
}
