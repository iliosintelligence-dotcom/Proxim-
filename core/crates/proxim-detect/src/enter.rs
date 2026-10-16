//! ENTER with internal EXIT hysteresis (engine-spec 6.1) for circles and polygons.
//!
//! idle -> `cand` on an `Inside` fix -> `in` (ENTER fires) -> `cool` after a confirmed exit.
//! Only `Inside` counts toward ENTER and only `Outside` toward EXIT; a `Straddle` (the accuracy
//! circle crosses the boundary) counts for neither, so boundary jitter within the accuracy
//! radius can neither enter nor exit.

use crate::prep::{Kind, Prepared};
use crate::{Detected, Event};
use proxim_geo::{Depth, acc_eff_mm, classify, signed_depth_circle_mm, signed_depth_polygon_mm};
use proxim_path::{PathSummary, p0_conf_milli};
use proxim_types::defaults::{
    CIRCLE_MAX_ACC_MM, COOL_MS, ENTER_MAX_GAP_MS, ENTER_MIN_FIXES, ENTER_MIN_SPAN_MS,
    ENTER_MIN_SPAN_SLOW_MS, ENTER_SLOW_MMPS, EXIT_FEW_FIXES, EXIT_FIXES, EXIT_SPAN_MS,
    IN_MAX_GAP_MS, IN_SURVIVE_GAP_MS, K_IN_PERMILLE, K_OUT_PERMILLE, KAPPA_PERMILLE,
};
use proxim_types::{DetState, EngineError, EventKind, Fix, ReasonCode};

/// Classify a fix against the shape. A fix that cannot be evaluated is a `Straddle` (counts for
/// nothing); a valid coordinate beyond the 150 km frame is `Outside`.
fn classify_fix(
    p: &Prepared,
    fix: &Fix,
    acc_eff: i64,
    k_permille: u32,
) -> Result<Depth, EngineError> {
    let valid =
        fix.lat_e7.unsigned_abs() <= 900_000_000 && fix.lon_e7.unsigned_abs() <= 1_800_000_000;
    if !valid {
        return Ok(Depth::Straddle);
    }
    let Ok(q) = p.frame.project(fix.position()) else {
        return Ok(Depth::Outside);
    };
    let depth = match &p.kind {
        Kind::Circle { center, radius_mm } => signed_depth_circle_mm(*center, *radius_mm, q),
        Kind::Polygon { vertices, .. } => signed_depth_polygon_mm(vertices, q),
        Kind::Route(_) => return Err(EngineError::Parse),
    };
    match depth {
        Ok(d) => classify(d, acc_eff, k_permille),
        Err(_) => Ok(Depth::Straddle),
    }
}

pub(crate) fn step_enter(
    p: &Prepared,
    st: Option<DetState>,
    path: &PathSummary,
    fix: &Fix,
    explain: bool,
) -> Result<Detected, EngineError> {
    let note = |c: ReasonCode| explain.then_some(c);
    let t = fix.t_ms;
    let unchanged = |reason: Option<ReasonCode>| Detected {
        state: st,
        event: None,
        reason,
    };

    // P0 is the only tier ENTER can reach: tier 0, confidence 500.
    if p.min_tier > 0 {
        return Ok(unchanged(note(ReasonCode::PathTierLow)));
    }
    if p.min_conf_milli > p0_conf_milli() {
        return Ok(unchanged(note(ReasonCode::PathConfidenceLow)));
    }

    // Accuracy gate: no transition at all on a fix that is too coarse for this shape.
    let acc_eff = acc_eff_mm(fix.acc_mm, KAPPA_PERMILLE)?;
    let (inradius_mm, is_circle) = match &p.kind {
        Kind::Circle { radius_mm, .. } => (Some(*radius_mm), true),
        Kind::Polygon { inradius_mm, .. } => (*inradius_mm, false),
        Kind::Route(_) => return Err(EngineError::Parse),
    };
    let too_coarse = (is_circle && fix.acc_mm > CIRCLE_MAX_ACC_MM)
        || inradius_mm.is_some_and(|r| {
            i128::from(r) * 1000 < i128::from(K_IN_PERMILLE) * i128::from(acc_eff)
        });
    if too_coarse {
        return Ok(unchanged(note(ReasonCode::AccTooCoarse)));
    }

    // Foreign state variants (corrupt or from another event kind) restart from idle.
    let st = match st {
        Some(s @ (DetState::Cand { .. } | DetState::In { .. } | DetState::Cool { .. })) => Some(s),
        _ => None,
    };
    let none_if = |reason: ReasonCode| Detected {
        state: None,
        event: None,
        reason: note(reason),
    };
    let out_of_order = |last: i64| t <= last;
    let ooo = || Detected {
        state: st,
        event: None,
        reason: note(ReasonCode::FixOutOfOrder),
    };

    match st {
        None => {
            if classify_fix(p, fix, acc_eff, K_IN_PERMILLE)? == Depth::Inside {
                Ok(Detected {
                    state: Some(DetState::Cand {
                        n: 1,
                        first_t: t,
                        last_t: t,
                    }),
                    event: None,
                    reason: note(ReasonCode::NoEvent),
                })
            } else {
                Ok(none_if(ReasonCode::NoEvent))
            }
        }
        Some(DetState::Cand { n, first_t, last_t }) => {
            if out_of_order(last_t) {
                return Ok(ooo());
            }
            let inside = classify_fix(p, fix, acc_eff, K_IN_PERMILLE)? == Depth::Inside;
            let gap = t.saturating_sub(last_t);
            if !inside {
                return Ok(none_if(ReasonCode::NoEvent));
            }
            if gap > ENTER_MAX_GAP_MS {
                return Ok(Detected {
                    state: Some(DetState::Cand {
                        n: 1,
                        first_t: t,
                        last_t: t,
                    }),
                    event: None,
                    reason: note(ReasonCode::NoEvent),
                });
            }
            let n2 = n.saturating_add(1);
            let slow = path.vf_mmps.is_none_or(|v| v < ENTER_SLOW_MMPS);
            let span_needed = if slow {
                ENTER_MIN_SPAN_SLOW_MS
            } else {
                ENTER_MIN_SPAN_MS
            };
            if n2 >= ENTER_MIN_FIXES && t.saturating_sub(first_t) >= span_needed {
                return Ok(Detected {
                    state: Some(DetState::In {
                        xn: 0,
                        x_first_t: None,
                        last_t: t,
                    }),
                    event: Some(Event {
                        kind: EventKind::Enter,
                        t_ms: t,
                        d_rem_mm: None,
                        t_hat_ms: None,
                        // ENTER needs `Inside`, so the fix is at least acc_eff inside the shape.
                        dist_to_shape_mm: 0,
                        path_tier: 0,
                        path_conf_milli: p0_conf_milli(),
                    }),
                    reason: None,
                });
            }
            Ok(Detected {
                state: Some(DetState::Cand {
                    n: n2,
                    first_t,
                    last_t: t,
                }),
                event: None,
                reason: note(ReasonCode::NoEvent),
            })
        }
        Some(DetState::In {
            xn,
            x_first_t,
            last_t,
        }) => {
            if out_of_order(last_t) {
                return Ok(ooo());
            }
            let gap = t.saturating_sub(last_t);
            if gap > IN_MAX_GAP_MS {
                // Too long unobserved: assume left and stay quiet for the cool-down.
                return Ok(cool_step(last_t.saturating_add(COOL_MS), t, note));
            }
            let (xn, x_first_t) = if gap > IN_SURVIVE_GAP_MS {
                (0, None) // an outside run does not survive a gap above 120 s
            } else {
                (xn, x_first_t)
            };
            let outside = classify_fix(p, fix, acc_eff, K_OUT_PERMILLE)? == Depth::Outside;
            let (xn, x_first_t) = if outside {
                (xn.saturating_add(1), x_first_t.or(Some(t)))
            } else {
                (0, None)
            };
            let confirmed = xn >= EXIT_FIXES
                || (xn >= EXIT_FEW_FIXES
                    && x_first_t.is_some_and(|f| t.saturating_sub(f) >= EXIT_SPAN_MS));
            if confirmed {
                // EXIT is internal: no event, nothing delivered.
                return Ok(Detected {
                    state: Some(DetState::Cool {
                        until_ms: t.saturating_add(COOL_MS),
                    }),
                    event: None,
                    reason: note(ReasonCode::NoEvent),
                });
            }
            Ok(Detected {
                state: Some(DetState::In {
                    xn,
                    x_first_t,
                    last_t: t,
                }),
                event: None,
                reason: note(ReasonCode::NoEvent),
            })
        }
        Some(DetState::Cool { until_ms }) => Ok(cool_step(until_ms, t, note)),
        // Unreachable by the sanitising match above; fail closed without a panic.
        Some(_) => Ok(none_if(ReasonCode::NoEvent)),
    }
}

/// `cool` ignores fixes until `t >= until`, then returns to idle; the releasing fix itself is not
/// counted toward a new candidate.
fn cool_step(until_ms: i64, t: i64, note: impl Fn(ReasonCode) -> Option<ReasonCode>) -> Detected {
    if t >= until_ms {
        Detected {
            state: None,
            event: None,
            reason: note(ReasonCode::NoEvent),
        }
    } else {
        Detected {
            state: Some(DetState::Cool { until_ms }),
            event: None,
            reason: note(ReasonCode::NoEvent),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cool_step_releases_at_until_and_not_before() {
        let note = |c: ReasonCode| Some(c);
        assert_eq!(
            cool_step(1000, 999, note).state,
            Some(DetState::Cool { until_ms: 1000 })
        );
        assert_eq!(cool_step(1000, 1000, note).state, None);
        assert_eq!(cool_step(1000, i64::MAX, note).state, None);
        assert_eq!(
            cool_step(i64::MAX, i64::MAX - 1, note).state,
            Some(DetState::Cool { until_ms: i64::MAX })
        );
        assert!(cool_step(1000, 5000, |_| None).reason.is_none());
    }
}
