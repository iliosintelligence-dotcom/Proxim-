//! APPROACH detector with the time-to-anchor fire rule (engine-spec 6.2, playbook 4).
//!
//! idle -> `armed` when every gate holds -> `latched` after one fire (or after a late join).
//! A latch is released only by distance (`1.2 x d_hi` from the anchor) or a new `rev`.

use crate::params::{
    BAND_HYSTERESIS_MMPS, CONFIRM_MAX_GAP_MS, CONFIRM_SPACING_MS, D_PASS_MIN_MM, D_PASS_S,
    L_MIN_FLOOR_MS, L_MIN_MS_PER_LEAD_S, MAX_ACC_EFF_MM, MAX_FIX_AGE_MS, REARM_DEN, REARM_NUM,
    T_COMP_MS,
};
use crate::prep::{Prepared, RoutePrep};
use crate::{Detected, Event};
use proxim_geo::{acc_eff_mm, dist_mm, rem_along_mm};
use proxim_path::{
    P1Evidence, PathSummary, closing, d_rem_mm, heading_dot, p1_confidence, project_recent,
    t_hat_ms, v_eff_for,
};
use proxim_types::defaults::KAPPA_PERMILLE;
use proxim_types::{DetState, EngineError, Enu, EventKind, Fix, LatchWhy, Mode, ReasonCode};

#[allow(clippy::too_many_lines)]
pub(crate) fn step_approach(
    p: &Prepared,
    r: &RoutePrep,
    st: Option<DetState>,
    path: &PathSummary,
    fix: &Fix,
    now_ms: i64,
    explain: bool,
) -> Result<Detected, EngineError> {
    let note = |c: ReasonCode| explain.then_some(c);
    let t = fix.t_ms;
    let origin = Enu { e_mm: 0, n_mm: 0 };

    // Time must move forward within a detector; otherwise nothing changes.
    let prev_last = match st {
        Some(DetState::Armed { last_t, .. } | DetState::Latched { last_t, .. }) => Some(last_t),
        _ => None,
    };
    if prev_last.is_some_and(|l| t <= l) {
        return Ok(Detected {
            state: st,
            event: None,
            reason: note(ReasonCode::FixOutOfOrder),
        });
    }
    let q = p.frame.project(fix.position()).ok();

    // A latch holds until the fix is farther than 1.2 x d_hi (Euclidean) from the anchor point.
    if let Some(DetState::Latched { why, .. }) = st {
        let far = match q {
            None => true,
            Some(q) => {
                let d = dist_mm(r.anchor_pt, q)?;
                i128::from(d) * REARM_DEN > i128::from(r.d_hi_mm) * REARM_NUM
            }
        };
        return Ok(if far {
            Detected {
                state: None,
                event: None,
                reason: note(ReasonCode::NoEvent),
            }
        } else {
            Detected {
                state: Some(DetState::Latched { why, last_t: t }),
                event: None,
                reason: note(ReasonCode::NoEvent),
            }
        });
    }

    let armed_prev = match st {
        Some(DetState::Armed { confirm_t, last_t }) => Some((confirm_t, last_t)),
        _ => None,
    };
    let armed = armed_prev.is_some();
    let idle = |reason: ReasonCode| Detected {
        state: None,
        event: None,
        reason: note(reason),
    };

    // Candidate pre-filter (spec 1.2): beyond reach_m the trigger is out of play; armed -> idle.
    let Some(q) = q else {
        return Ok(idle(ReasonCode::OutOfRange));
    };
    if dist_mm(origin, q)? > p.reach_mm {
        return Ok(idle(ReasonCode::OutOfRange));
    }

    // Gates, in the spec's order; the first failure returns to idle.
    let age_ms = now_ms.saturating_sub(t).max(0);
    if age_ms > MAX_FIX_AGE_MS {
        return Ok(idle(ReasonCode::FixStale));
    }
    let acc_eff = acc_eff_mm(fix.acc_mm, KAPPA_PERMILLE)?;
    if acc_eff > MAX_ACC_EFF_MM {
        return Ok(idle(ReasonCode::FixAccuracyTooCoarse));
    }
    let v_eff = v_eff_for(r.mode, path.vf_mmps);
    let proj = match project_recent(path, &p.frame, &r.route, armed, v_eff) {
        Ok(x) => x,
        Err(EngineError::OutOfFrame) => return Ok(idle(ReasonCode::OutOfRange)),
        Err(e) => return Err(e),
    };
    if proj
        .items
        .iter()
        .any(|i| i.xt_mm.saturating_abs() > r.buffer_mm)
    {
        return Ok(idle(ReasonCode::OffCorridor));
    }
    let (band_lo, band_hi) = if armed {
        (
            r.band_min_mmps.saturating_sub(BAND_HYSTERESIS_MMPS),
            r.band_max_mmps.saturating_add(BAND_HYSTERESIS_MMPS),
        )
    } else {
        (r.band_min_mmps, r.band_max_mmps)
    };
    if !path.vf_mmps.is_some_and(|v| v >= band_lo && v <= band_hi) {
        return Ok(idle(ReasonCode::SpeedOutOfBand));
    }
    let dot = heading_dot(path.heading_q30, proj.cur.tangent_q30);
    if !dot.is_some_and(|d| d >= r.cos_tol_q30) {
        return Ok(idle(ReasonCode::BearingMismatch));
    }
    if !closing(&proj.items) {
        return Ok(idle(ReasonCode::NotClosing));
    }
    let rem = rem_along_mm(r.anchor_mm, proj.cur.along_mm)?;
    let d_rem = d_rem_mm(rem, v_eff, age_ms)?;
    let d_pass = D_PASS_MIN_MM.max(i64::from(v_eff).saturating_mul(D_PASS_S));
    if d_rem > r.d_hi_mm || d_rem < d_pass {
        return Ok(idle(ReasonCode::BeyondWindow));
    }
    let conf = p1_confidence(&P1Evidence {
        recent: &proj.items,
        fixes_60s: path.fixes_60s,
        vf_mmps: path.vf_mmps,
        mode: r.mode,
        heading_dot_q30: dot,
        buffer_mm: r.buffer_mm,
    })?;
    // Approach implies at least tier 1; P2 and P3 are reserved, so a higher floor never passes.
    let Some(conf) = conf.filter(|_| p.min_tier <= 1) else {
        return Ok(idle(ReasonCode::PathTierLow));
    };
    if conf < p.min_conf_milli {
        return Ok(idle(ReasonCode::PathConfidenceLow));
    }

    let t_hat = t_hat_ms(d_rem, v_eff)?;
    let d_lo = match r.mode {
        Mode::Vehicle => r.d_lo_base_mm,
        _ => r.d_lo_base_mm.max(acc_eff.saturating_mul(3)),
    }
    .min(r.d_hi_mm);
    let lead_ms = i64::from(r.lead_s) * 1000;

    let Some((confirm_t, last_t)) = armed_prev else {
        // First fix where every gate holds. Late join: too close to give the promised lead.
        let l_min = L_MIN_FLOOR_MS.max(lead_ms / 1000 * L_MIN_MS_PER_LEAD_S);
        if t_hat < l_min && d_rem > d_lo {
            return Ok(Detected {
                state: Some(DetState::Latched {
                    why: LatchWhy::Late,
                    last_t: t,
                }),
                event: None,
                reason: Some(ReasonCode::TooLateForLead), // terminal: reported even without explain
            });
        }
        return Ok(Detected {
            state: Some(DetState::Armed {
                confirm_t: None,
                last_t: t,
            }),
            event: None,
            reason: note(ReasonCode::NoEvent),
        });
    };

    let holds = t_hat <= lead_ms.saturating_add(T_COMP_MS) || d_rem <= d_lo;
    let confirm_t = if t.saturating_sub(last_t) > CONFIRM_MAX_GAP_MS {
        None
    } else {
        confirm_t
    };
    let next_confirm = if !holds {
        None
    } else {
        match confirm_t {
            None => Some(t),
            Some(c) if t.saturating_sub(c) >= CONFIRM_SPACING_MS => {
                return Ok(Detected {
                    state: Some(DetState::Latched {
                        why: LatchWhy::Fired,
                        last_t: t,
                    }),
                    event: Some(Event {
                        kind: EventKind::Approach,
                        t_ms: t,
                        d_rem_mm: Some(d_rem),
                        t_hat_ms: Some(t_hat),
                        dist_to_shape_mm: proj.cur.dist_mm,
                        path_tier: 1,
                        path_conf_milli: conf,
                    }),
                    reason: None,
                });
            }
            Some(c) => Some(c),
        }
    };
    Ok(Detected {
        state: Some(DetState::Armed {
            confirm_t: next_confirm,
            last_t: t,
        }),
        event: None,
        reason: note(ReasonCode::NoEvent),
    })
}
