//! Corridor evidence for tier P1 (engine-spec 5.3): projection of the last three fixes onto a
//! route, the CLOSING test and the confidence ladder. Pure functions of their arguments.

use crate::filter::{PathSummary, min_speed_mmps};
use crate::params::{
    CLOSING_MIN_ADVANCE_MM, P0_CONF, P1_ACC_EFF_MAX_MM, P1_ACC_PENALTY1_MM, P1_ACC_PENALTY2_MM,
    P1_BONUS_FIXES, P1_BONUS_HEADING_TOL_DDEG, P1_CONF_BASE, P1_CONF_BONUS, P1_CONF_FOOT_CAP,
    P1_CONF_PENALTY1, P1_CONF_PENALTY2, P1_HEADING_TOL_DDEG, P1_MIN_FIXES, WINDOW_AHEAD_MM,
    WINDOW_AHEAD_S, WINDOW_BACK_MM,
};
use alloc::vec::Vec;
use proxim_geo::{
    Frame, Polyline, Projection, acc_eff_mm, cos_tol_q30, dot_q30, project_to_polyline,
};
use proxim_types::defaults::KAPPA_PERMILLE;
use proxim_types::{Copy as AdCopy, EngineError, Mode, Position};

/// One recent fix projected onto a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recent {
    pub t_ms: i64,
    pub along_mm: i64,
    /// Signed cross-track (left positive); `abs` is the distance to the route.
    pub xt_mm: i64,
    pub acc_eff_mm: i64,
}

/// The last up to three fixes projected onto a route, oldest first. The last entry is the
/// current fix and `cur` its full projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentProjection {
    pub items: Vec<Recent>,
    pub cur: Projection,
}

/// Project the newest three fixes of `summary` onto `route` (spec 6.2).
///
/// Not armed: each fix is projected onto the whole route independently. Armed: the oldest fix is
/// projected onto the whole route and each later one onto the window
/// `[a_prev - 100 m, a_prev + 500 m + v_eff x 30 s]` after its predecessor, which keeps a parallel
/// leg or the other half of a loop from capturing the track. If a window overlaps no segment the
/// fix falls back to the whole route.
///
/// # Errors
/// [`EngineError::OutOfFrame`] if a fix is more than 150 km from the frame origin;
/// [`EngineError::Overflow`].
pub fn project_recent(
    summary: &PathSummary,
    frame: &Frame,
    route: &Polyline,
    armed: bool,
    v_eff_mmps: u32,
) -> Result<RecentProjection, EngineError> {
    let ahead = WINDOW_AHEAD_MM
        .checked_add(i64::from(v_eff_mmps) * WINDOW_AHEAD_S)
        .ok_or(EngineError::Overflow)?;
    let mut items: Vec<Recent> = Vec::with_capacity(3);
    let mut prev_along: Option<i64> = None;
    let mut cur: Option<Projection> = None;
    for f in summary.recent_fixes() {
        let p = frame.project(Position {
            lat_e7: f.lat_e7,
            lon_e7: f.lon_e7,
        })?;
        let window = if armed {
            prev_along.map(|a| (a.saturating_sub(WINDOW_BACK_MM), a.saturating_add(ahead)))
        } else {
            None
        };
        let proj = match project_to_polyline(route, p, window) {
            Err(EngineError::DegenerateGeometry) if window.is_some() => {
                project_to_polyline(route, p, None)?
            }
            other => other?,
        };
        prev_along = Some(proj.along_mm);
        items.push(Recent {
            t_ms: f.t_ms,
            along_mm: proj.along_mm,
            xt_mm: proj.xt_mm,
            acc_eff_mm: acc_eff_mm(f.acc_mm, KAPPA_PERMILLE)?,
        });
        cur = Some(proj);
    }
    Ok(RecentProjection {
        items,
        cur: cur.ok_or(EngineError::DegenerateGeometry)?,
    })
}

/// CLOSING over the last three fixes (5.3): with `tol = max(acc_eff) / 2`,
/// `a2 - a0 >= max(5 m, tol)`, `a1 - a0 >= -tol` and `a2 - a1 >= -tol`. Needs exactly three
/// fixes (fewer is not closing).
#[must_use]
pub fn closing(items: &[Recent]) -> bool {
    let [r0, r1, r2] = items else {
        return false;
    };
    let tol = r0.acc_eff_mm.max(r1.acc_eff_mm).max(r2.acc_eff_mm) / 2;
    let net = r2.along_mm.saturating_sub(r0.along_mm);
    let first = r1.along_mm.saturating_sub(r0.along_mm);
    let second = r2.along_mm.saturating_sub(r1.along_mm);
    let neg = tol.saturating_neg();
    net >= CLOSING_MIN_ADVANCE_MM.max(tol) && first >= neg && second >= neg
}

/// Evidence for tier P1 (5.3).
#[derive(Clone, Copy, Debug)]
pub struct P1Evidence<'a> {
    pub recent: &'a [Recent],
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
    if dot < i64::from(cos_tol_q30(P1_HEADING_TOL_DDEG)?) || !closing(ev.recent) {
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

    fn r(along: i64, acc: i64) -> Recent {
        Recent {
            t_ms: 0,
            along_mm: along,
            xt_mm: 0,
            acc_eff_mm: acc,
        }
    }

    #[test]
    fn closing_accepts_steady_advance() {
        assert!(closing(&[
            r(0, 10_000),
            r(15_000, 10_000),
            r(30_000, 10_000)
        ]));
    }

    #[test]
    fn closing_boundaries_are_exact() {
        // acc_eff 10 m -> tol 5 m: net must reach max(5 m, 5 m) = 5000.
        assert!(closing(&[r(0, 10_000), r(2500, 10_000), r(5000, 10_000)]));
        assert!(!closing(&[r(0, 10_000), r(2500, 10_000), r(4999, 10_000)]));
        // acc_eff 40 m -> tol 20 m: net 20 m passes, 19.999 m fails.
        assert!(closing(&[
            r(0, 40_000),
            r(10_000, 40_000),
            r(20_000, 40_000)
        ]));
        assert!(!closing(&[
            r(0, 40_000),
            r(10_000, 40_000),
            r(19_999, 40_000)
        ]));
        // Backwards steps up to tol are tolerated, beyond tol are not.
        assert!(closing(&[
            r(0, 10_000),
            r(-5000, 10_000),
            r(10_000, 10_000)
        ]));
        assert!(!closing(&[
            r(0, 10_000),
            r(-5001, 10_000),
            r(10_000, 10_000)
        ]));
        assert!(closing(&[
            r(0, 10_000),
            r(20_000, 10_000),
            r(15_000, 10_000)
        ]));
        assert!(!closing(&[
            r(0, 10_000),
            r(20_000, 10_000),
            r(14_999, 10_000)
        ]));
    }

    #[test]
    fn closing_rejects_u_turn_stopped_and_short_input() {
        assert!(!closing(&[
            r(30_000, 10_000),
            r(15_000, 10_000),
            r(0, 10_000)
        ]));
        assert!(!closing(&[r(0, 10_000), r(0, 10_000), r(0, 10_000)]));
        // The worst accuracy among the three sets tol.
        assert!(!closing(&[r(0, 10_000), r(3000, 10_000), r(6000, 60_000)]));
        assert!(!closing(&[r(0, 10_000), r(5000, 10_000)]));
        assert!(!closing(&[]));
        assert!(!closing(&[r(0, 1), r(1, 1), r(9999, 1), r(20_000, 1)]));
    }

    #[test]
    fn closing_never_overflows() {
        assert!(!closing(&[
            r(i64::MIN, i64::MAX),
            r(i64::MAX, 0),
            r(i64::MIN, 0)
        ]));
        let _ = closing(&[r(i64::MIN, 0), r(0, 0), r(i64::MAX, 0)]);
    }

    #[test]
    fn copy_ladder() {
        assert_eq!(copy_for_tier(0), AdCopy::NearYou);
        assert_eq!(copy_for_tier(1), AdCopy::Ahead);
        assert_eq!(copy_for_tier(2), AdCopy::OnRoute);
        assert_eq!(copy_for_tier(200), AdCopy::OnRoute);
    }
}
