//! The decision function (engine-spec 7): ordered hard filters F01-F25, caps, one-slot selection.
//!
//! Pure: [`decide`] is a function of its arguments only. No clock (time is `clock.now_ms`), no
//! randomness (`seed` is carried and UNUSED in M1), no I/O, no hash-order dependence (`BTreeMap`
//! and sorted vectors only). Every input is untrusted: no `unwrap`, no unchecked indexing, time
//! arithmetic saturates and geometry arithmetic is `i128` or checked, so overflow is impossible.
//!
//! M1 scope of the pack checks (spec 8, 10): the engine receives already-decoded signposts, so
//! signature, pack expiry and revocation (reason ids 500 and above: `REVOKED`, `SIGNATURE_INVALID`,
//! `SEQ_ROLLBACK`, `KID_REVOKED`, `PACK_EXPIRED`) are placeholders owned by `proxim-pack` (M2) and
//! are not decided here; what M1 does check is the signpost's own validity window (F13, F14), the
//! reserved-event fail-closed rule (F11), consent (F01, F15) and the policy filters (F16 to F18).
//!
//! Module map: [`json`] is the cross-binding conformance entry (`decide_json`), [`golden`] runs and
//! builds golden vectors, [`sha256`] hashes their bytes.
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod calendar;
mod caps;
mod eff;
pub mod golden;
pub mod json;
pub mod params;
pub mod sha256;
mod standoff;
mod validate;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use caps::CapsBook;
use eff::Eff;
use params::{
    DET_PRUNE_MS, EVENT_AGE_APPROACH_S, EVENT_AGE_ENTER_S, EVENT_DIST_M, FIX_FUTURE_SLACK_MS,
    FIX_MAX_AGE_MS, JUMP_MARGIN_MM, JUMP_MAX_SPEED_MMPS, JUMP_RESET_AT, JUMP_WINDOW_MS,
    RANK_DEFAULT_MILLI,
};
use proxim_detect::{Event, Prepared};
use proxim_geo::{acc_eff_mm, dist_mm, frame_for};
use proxim_path::{copy_for_tier, path_update};
use proxim_types::defaults::{
    KAPPA_PERMILLE, LEAD_S_DEFAULT, LEAD_S_MAX, LEAD_S_MIN, MAX_DETECTORS, MAX_SIGNPOSTS,
};
use proxim_types::{
    Caps, Class, Clock, Consent, ConsentValue, Decision, DetEntry, DetState, EngineError, Enu,
    EventKind, Fix, FixStatus, Mem, Note, Outcome, Output, PathInfo, Policy, Position, ReasonCode,
    Signpost, State, TraceFix,
};

const ORIGIN: Enu = Enu { e_mm: 0, n_mm: 0 };

/// `decide(state, fix, signposts, policy, consent, clock, seed) -> (Output, State)`.
///
/// `seed` is carried but UNUSED in M1 (a test asserts it). A shell MUST treat any `Err` as
/// "no decision, state unchanged".
///
/// # Errors
/// [`EngineError::Parse`] for a structurally invalid `state`, `clock` or zone;
/// [`EngineError::Limit`] for oversize lists; [`EngineError::UnsupportedLatitude`] for a standoff
/// zone above 80 degrees; arithmetic errors are unreachable for validated input.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn decide(
    state: &State,
    fix: &Fix,
    signposts: &[Signpost],
    policy: &Policy,
    consent: &Consent,
    clock: &Clock,
    seed: u64,
) -> Result<(Output, State), EngineError> {
    let _ = seed; // M1: carried, never read (seed_has_no_effect_in_m1).
    validate::state(state)?;
    validate::clock(clock)?;
    if signposts.len() > MAX_SIGNPOSTS {
        return Err(EngineError::Limit);
    }
    let eff = Eff::new(policy)?;
    let zones = standoff::build(policy)?;
    let now = clock.now_ms.max(state.caps.hwm_ms);

    // ---- global filters F01-F10 (first failure ends the call) ----
    if consent.location_matching != ConsentValue::Granted {
        return Ok(stop(state, now, ReasonCode::ConsentDenied, true));
    }
    if eff.child_directed {
        return Ok(stop(state, now, ReasonCode::EngineDisabled, true));
    }
    if validate::fix_is_invalid(fix) {
        return Ok(stop(state, now, ReasonCode::FixInvalid, false));
    }
    if fix.is_mock && !eff.simulator_mode {
        return Ok(stop(state, now, ReasonCode::FixMock, false));
    }
    if fix.acc_mm > eff.max_acc_mm {
        return Ok(stop(state, now, ReasonCode::FixAccuracyTooCoarse, false));
    }
    if fix.t_ms > now.saturating_add(FIX_FUTURE_SLACK_MS) {
        return Ok(stop(state, now, ReasonCode::FixFromFuture, false));
    }
    if now.saturating_sub(fix.t_ms) > FIX_MAX_AGE_MS {
        return Ok(stop(state, now, ReasonCode::FixStale, false));
    }
    if state.mem.last_t_ms.is_some_and(|l| fix.t_ms <= l) {
        return Ok(stop(state, now, ReasonCode::FixOutOfOrder, false));
    }
    // F09: reject an impossible jump; the third consecutive reject resets and accepts.
    let mut base_mem = state.mem.clone();
    if state.mem.trace.last().is_some_and(|p| is_jump(p, fix)) {
        let rejects = state.mem.jump_rejects.saturating_add(1);
        if rejects < JUMP_RESET_AT {
            let mut next = state.clone();
            next.mem.jump_rejects = rejects;
            let (out, _) = stop(state, now, ReasonCode::FixJump, false);
            return Ok((out, next));
        }
        base_mem = Mem::default();
    }
    base_mem.jump_rejects = 0;
    // F10: minimal standoff. Generic SUPPRESSED, memory wiped, caps untouched, nothing learned.
    let acc_eff = acc_eff_mm(fix.acc_mm, KAPPA_PERMILLE)?;
    if standoff::fix_in_standoff(&zones, fix, acc_eff)? {
        let reason = if eff.explain {
            ReasonCode::SensitiveStandoff
        } else {
            ReasonCode::Suppressed
        };
        return Ok(stop(state, now, reason, true));
    }

    // ---- accepted: filters, trace, caps buckets ----
    let (mut mem, summary) = path_update(&base_mem, fix)?;
    let mut book = CapsBook::load(&state.caps, now, clock.tz_offset_min);

    let (candidates, mut drops) = select_revisions(signposts, fix.t_ms);
    let present: BTreeSet<&str> = signposts.iter().map(|s| s.id.as_str()).collect();
    let mut det: BTreeMap<String, DetEntry> = BTreeMap::new();
    for e in &mem.det {
        if present.contains(e.id.as_str()) || !is_stale(&e.st, now) {
            det.insert(String::from(e.id.as_str()), e.clone());
        }
    }

    let ctx = Ctx {
        eff: &eff,
        consent,
        zones: &zones,
        now,
    };
    let mut notes: Vec<Note> = Vec::new();
    let mut survivors: Vec<Cand<'_>> = Vec::new();

    for sp in candidates {
        let key = sp.id.as_str();
        let prepared = match prepare(&ctx, sp)? {
            Ok(p) => p,
            Err(reason) => {
                det.remove(key);
                drops.push(drop_decision(sp, reason, fix.t_ms));
                continue;
            }
        };
        let pre = det.get(key).filter(|e| e.rev == sp.rev).map(|e| e.st);
        if pre.is_none() {
            // Idle (or a replaced revision): beyond reach_m nothing is recorded.
            det.remove(key);
            if prepared.beyond_reach(fix.position()) {
                continue;
            }
        }
        let d = prepared.step(pre, &summary, fix, now, eff.explain)?;
        match d.state {
            Some(st) => {
                det.insert(
                    String::from(key),
                    DetEntry {
                        id: sp.id.clone(),
                        rev: sp.rev,
                        st,
                    },
                );
            }
            None => {
                det.remove(key);
            }
        }
        match d.reason {
            Some(
                r @ (ReasonCode::TooLateForLead
                | ReasonCode::UnsupportedEvent
                | ReasonCode::TriggerInvalid),
            ) => drops.push(drop_decision(sp, r, fix.t_ms)),
            Some(r) if eff.explain => notes.push(Note {
                signpost: sp.id.clone(),
                reason: r,
            }),
            _ => {}
        }
        let Some(ev) = d.event else { continue };
        match event_filter(&ctx, &book, state.caps.last_fire_ms, sp, &ev) {
            Some(reason) => drops.push(event_drop(sp, &ev, reason)),
            None => survivors.push(Cand { sp, ev, pre }),
        }
    }

    // ---- selection: score desc, t_hat asc (absent last), id asc ----
    survivors.sort_by(|a, b| {
        b.score()
            .cmp(&a.score())
            .then_with(|| cmp_t_hat(a.ev.t_hat_ms, b.ev.t_hat_ms))
            .then_with(|| a.sp.id.cmp(&b.sp.id))
    });
    let mut fires: Vec<Decision> = Vec::new();
    let mut last_fire_ms = state.caps.last_fire_ms;
    let mut path_info = summary.info;
    for (rank, c) in survivors.iter().enumerate() {
        if rank < eff.slots {
            book.record_fire(&c.sp.id);
            last_fire_ms = Some(now);
            path_info = PathInfo {
                tier: c.ev.path_tier,
                conf_milli: c.ev.path_conf_milli,
                ..summary.info
            };
            fires.push(fire_decision(c.sp, &c.ev));
        } else {
            // The loser reverts to its pre-event detector state: it may fire at a later fix.
            match c.pre {
                Some(st) => {
                    det.insert(
                        String::from(c.sp.id.as_str()),
                        DetEntry {
                            id: c.sp.id.clone(),
                            rev: c.sp.rev,
                            st,
                        },
                    );
                }
                None => {
                    det.remove(c.sp.id.as_str());
                }
            }
            drops.push(event_drop(c.sp, &c.ev, ReasonCode::LostSlot));
        }
    }

    // ---- commit and output ----
    if book.len() > MAX_SIGNPOSTS || det.len() > MAX_DETECTORS {
        return Err(EngineError::Limit);
    }
    mem.det = det.into_values().collect();
    let next = State {
        v: 1,
        caps: Caps {
            hwm_ms: now,
            last_fire_ms,
            signposts: book.into_vec(),
        },
        mem,
    };
    sort_decisions(&mut fires);
    sort_decisions(&mut drops);
    let mut decisions = fires;
    let notes = if eff.explain && !notes.is_empty() {
        Some(notes)
    } else {
        None
    };
    if eff.explain {
        decisions.append(&mut drops);
    }
    let out = Output {
        v: 1,
        t_ms: now,
        fix: FixStatus {
            ok: true,
            reason: ReasonCode::Ok,
        },
        path: path_info,
        decisions,
        notes,
    };
    Ok((out, next))
}

/// Decisions in `(id, rev, reason id)` order. Fires are listed before drops by the caller; with
/// the M1 ceiling of one slot there is at most one fire, so the order of fires matters only once
/// slots above one are allowed (unit-tested on synthetic decisions).
fn sort_decisions(v: &mut [Decision]) {
    v.sort_by(|a, b| {
        a.signpost
            .cmp(&b.signpost)
            .then(a.rev.cmp(&b.rev))
            .then(a.reason.id().cmp(&b.reason.id()))
    });
}

/// Per-call constants of the signpost loop.
struct Ctx<'a> {
    eff: &'a Eff<'a>,
    consent: &'a Consent,
    zones: &'a [standoff::Zone],
    now: i64,
}

/// A detector event that passed the hard filters and competes for a slot.
struct Cand<'a> {
    sp: &'a Signpost,
    ev: Event,
    pre: Option<DetState>,
}

impl Cand<'_> {
    /// `rank_milli x path_conf_milli / 1000` (spec 7).
    fn score(&self) -> u64 {
        let rank = self.sp.rank_milli.unwrap_or(RANK_DEFAULT_MILLI);
        u64::from(rank) * u64::from(self.ev.path_conf_milli) / 1000
    }
}

/// `t_hat` ascending, an absent value last.
fn cmp_t_hat(a: Option<i64>, b: Option<i64>) -> core::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => core::cmp::Ordering::Less,
        (None, Some(_)) => core::cmp::Ordering::Greater,
        (None, None) => core::cmp::Ordering::Equal,
    }
}

/// Call-ending output for F01-F10. `wipe` clears `mem`; `caps` is never touched.
fn stop(state: &State, now: i64, reason: ReasonCode, wipe: bool) -> (Output, State) {
    let out = Output {
        v: 1,
        t_ms: now,
        fix: FixStatus { ok: false, reason },
        path: PathInfo {
            tier: 0,
            conf_milli: 0,
            speed_mmps: None,
            heading_valid: false,
        },
        decisions: Vec::new(),
        notes: None,
    };
    let mut next = state.clone();
    if wipe {
        next.mem = Mem::default();
    }
    (out, next)
}

/// F09: implied speed above 83 m/s AND distance beyond both accuracies plus 100 m, against the
/// newest trace fix if it is at most 60 s old. A fix that cannot be placed in the previous fix's
/// frame is treated as a jump (fail closed).
fn is_jump(prev: &TraceFix, fix: &Fix) -> bool {
    let dt = fix.t_ms.saturating_sub(prev.t_ms);
    if dt <= 0 || dt > JUMP_WINDOW_MS {
        return false;
    }
    let d = frame_for(Position {
        lat_e7: prev.lat_e7,
        lon_e7: prev.lon_e7,
    })
    .and_then(|f| f.project(fix.position()))
    .and_then(|q| dist_mm(ORIGIN, q));
    let Ok(d) = d else { return true };
    let too_fast = i128::from(d) * 1000 > JUMP_MAX_SPEED_MMPS * i128::from(dt);
    let margin = i64::from(prev.acc_mm) + i64::from(fix.acc_mm) + JUMP_MARGIN_MM;
    too_fast && d > margin
}

/// An absent detector entry is stale (pruned) when unseen for 900 s; a cool-down is kept until it
/// has expired (errata: dropping it earlier would shorten the cool-down).
fn is_stale(st: &DetState, now: i64) -> bool {
    match st {
        DetState::Cool { until_ms } => now >= *until_ms,
        DetState::Cand { last_t, .. }
        | DetState::In { last_t, .. }
        | DetState::Armed { last_t, .. }
        | DetState::Latched { last_t, .. } => now.saturating_sub(*last_t) > DET_PRUNE_MS,
    }
}

/// One revision per id is processed (the highest). Lower revisions are superseded and an id with
/// two copies of its highest revision is ambiguous; both are `TRIGGER_INVALID` drops. Input order
/// never matters: the list is sorted by `(id, rev)` first.
fn select_revisions(signposts: &[Signpost], t_ms: i64) -> (Vec<&Signpost>, Vec<Decision>) {
    let mut order: Vec<&Signpost> = signposts.iter().collect();
    order.sort_by(|a, b| a.id.cmp(&b.id).then(a.rev.cmp(&b.rev)));
    let mut keep: Vec<&Signpost> = Vec::new();
    let mut drops: Vec<Decision> = Vec::new();
    for group in order.chunk_by(|a, b| a.id == b.id) {
        let mut rev_iter = group.iter().rev();
        let Some(top) = rev_iter.next() else { continue };
        let ambiguous = rev_iter.next().is_some_and(|second| second.rev == top.rev);
        let n_drop = if ambiguous {
            group.len()
        } else {
            group.len().saturating_sub(1)
        };
        for s in group.iter().take(n_drop) {
            drops.push(drop_decision(s, ReasonCode::TriggerInvalid, t_ms));
        }
        if !ambiguous {
            keep.push(top);
        }
    }
    (keep, drops)
}

/// Static filters F11-F19, then the prepared trigger. `Ok(Err(reason))` is a per-signpost drop.
fn prepare(ctx: &Ctx<'_>, sp: &Signpost) -> Result<Result<Prepared, ReasonCode>, EngineError> {
    let eff = ctx.eff;
    // F11 (E5): everything but enter and approach is reserved and fails closed.
    if !matches!(sp.event, EventKind::Enter | EventKind::Approach) {
        return Ok(Err(ReasonCode::UnsupportedEvent));
    }
    // F12: validity window and lead range are checked on the authored values, before narrowing.
    if sp.valid_until_ms <= sp.valid_from_ms {
        return Ok(Err(ReasonCode::TriggerInvalid));
    }
    let mut narrowed: Option<Signpost> = None;
    if sp.event == EventKind::Approach {
        let lead = sp.trigger.lead_s.unwrap_or(LEAD_S_DEFAULT);
        if !(LEAD_S_MIN..=LEAD_S_MAX).contains(&lead) {
            return Ok(Err(ReasonCode::TriggerInvalid));
        }
        let clamped = lead.clamp(eff.lead_lo_s, eff.lead_hi_s);
        if clamped != lead {
            let mut c = sp.clone();
            c.trigger.lead_s = Some(clamped);
            narrowed = Some(c);
        }
    }
    let prepared = match Prepared::new(narrowed.as_ref().unwrap_or(sp)) {
        Ok(p) => p,
        Err(reason) => return Ok(Err(reason)),
    };
    // F13 / F14: live window, [valid_from, valid_until).
    if ctx.now < sp.valid_from_ms {
        return Ok(Err(ReasonCode::NotYetLive));
    }
    if ctx.now >= sp.valid_until_ms {
        return Ok(Err(ReasonCode::Expired));
    }
    // F15 (I12): health needs the consent flag and a receipt (M1: non-empty; the SDK issues it).
    if sp.class == Class::Health {
        let receipt = ctx
            .consent
            .health_receipt
            .as_deref()
            .is_some_and(|r| !r.is_empty());
        if ctx.consent.nearby_health_events != ConsentValue::Granted || !receipt {
            return Ok(Err(ReasonCode::HealthConsentMissing));
        }
    }
    if eff.categories_blocked.contains(sp.category.as_str()) {
        return Ok(Err(ReasonCode::PartnerCategoryBlocked));
    }
    if eff.senders_blocked.contains(sp.sender.as_str()) {
        return Ok(Err(ReasonCode::SenderBlocked));
    }
    if sp.sensitivity > eff.sensitivity_ceiling {
        return Ok(Err(ReasonCode::SensitivityAboveCeiling));
    }
    if sp.class == Class::Health && standoff::trigger_in_standoff(ctx.zones, sp)? {
        return Ok(Err(ReasonCode::TriggerInStandoff));
    }
    Ok(Ok(prepared))
}

/// Event filters F20-F24. A dropped event is consumed, never retried.
fn event_filter(
    ctx: &Ctx<'_>,
    book: &CapsBook,
    last_fire_ms: Option<i64>,
    sp: &Signpost,
    ev: &Event,
) -> Option<ReasonCode> {
    let default_age = if sp.event == EventKind::Approach {
        EVENT_AGE_APPROACH_S
    } else {
        EVENT_AGE_ENTER_S
    };
    let max_age_ms = i64::from(sp.trigger.max_event_age_s.unwrap_or(default_age)) * 1000;
    if ctx.now.saturating_sub(ev.t_ms) > max_age_ms {
        return Some(ReasonCode::EventTooOld);
    }
    let max_dist_mm = i64::from(sp.trigger.max_event_dist_m.unwrap_or(EVENT_DIST_M)) * 1000;
    if ev.dist_to_shape_mm > max_dist_mm {
        return Some(ReasonCode::EventTooFar);
    }
    if book.day_count(sp.id.as_str()) >= ctx.eff.cap_day {
        return Some(ReasonCode::CapSignpostDay);
    }
    if book.week_count(sp.id.as_str()) >= ctx.eff.cap_week {
        return Some(ReasonCode::CapSignpostWeek);
    }
    // A last fire in the future of `now` (tampered state) also blocks: fail closed.
    if last_fire_ms.is_some_and(|l| ctx.now.saturating_sub(l) < ctx.eff.app_interval_ms) {
        return Some(ReasonCode::CapAppInterval);
    }
    None
}

fn drop_decision(sp: &Signpost, reason: ReasonCode, t_ms: i64) -> Decision {
    Decision {
        signpost: sp.id.clone(),
        rev: sp.rev,
        outcome: Outcome::Drop,
        reason,
        event: sp.event,
        t_ms,
        path_tier: 0,
        path_conf_milli: 0,
        copy: copy_for_tier(0),
        d_rem_mm: None,
        t_hat_ms: None,
    }
}

fn event_drop(sp: &Signpost, ev: &Event, reason: ReasonCode) -> Decision {
    Decision {
        signpost: sp.id.clone(),
        rev: sp.rev,
        outcome: Outcome::Drop,
        reason,
        event: ev.kind,
        t_ms: ev.t_ms,
        path_tier: ev.path_tier,
        path_conf_milli: ev.path_conf_milli,
        copy: copy_for_tier(ev.path_tier),
        d_rem_mm: ev.d_rem_mm,
        t_hat_ms: ev.t_hat_ms,
    }
}

fn fire_decision(sp: &Signpost, ev: &Event) -> Decision {
    Decision {
        outcome: Outcome::Fire,
        reason: ReasonCode::Ok,
        ..event_drop(sp, ev, ReasonCode::Ok)
    }
}

/// Small constructors for unit tests inside this crate.
#[cfg(test)]
pub(crate) mod tests_support {
    use proxim_types::{Class, EventKind, Id, Position, Reserved, Shape, Signpost, Trigger};

    pub fn general_signpost() -> Signpost {
        let origin = Position {
            lat_e7: 450_000_000,
            lon_e7: 70_000_000,
        };
        Signpost {
            id: Id::new("sp-1").unwrap(),
            rev: 1,
            class: Class::General,
            category: "cafe".into(),
            sender: "s".into(),
            sensitivity: 0,
            event: EventKind::Enter,
            valid_from_ms: 0,
            valid_until_ms: i64::MAX,
            min_path_tier: None,
            min_path_conf_milli: None,
            rank_milli: None,
            trigger: Trigger {
                origin,
                reach_m: None,
                shape: Shape::Circle {
                    center: origin,
                    radius_m: 200,
                },
                max_event_age_s: None,
                max_event_dist_m: None,
                lead_s: None,
                d_lo_m: None,
                d_hi_m: None,
                mode: None,
                speed_min_mmps: None,
                speed_max_mmps: None,
                bearing_tol_ddeg: None,
            },
            intent: Reserved::default(),
            content_mode: Reserved::default(),
            precision_class: Reserved::default(),
            min_mode: Reserved::default(),
            max_acc_m: Reserved::default(),
            arm_scope: Reserved::default(),
            relation: Reserved::default(),
            selector: Reserved::default(),
            live_at_ms: Reserved::default(),
            fresh_required_s: Reserved::default(),
            cooldown_s: Reserved::default(),
            creatives: Reserved::default(),
            rot_days: Reserved::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::general_signpost;
    use core::cmp::Ordering;
    use proxim_types::{Id, LatchWhy};

    fn trace_fix(t_ms: i64, lat_e7: i32, lon_e7: i32, acc_mm: u32) -> TraceFix {
        TraceFix {
            t_ms,
            lat_e7,
            lon_e7,
            acc_mm,
            v_mmps: None,
        }
    }

    fn fix(t_ms: i64, lat_e7: i32, lon_e7: i32, acc_mm: u32) -> Fix {
        Fix {
            t_ms,
            lat_e7,
            lon_e7,
            acc_mm,
            speed_mmps: None,
            speed_acc_mmps: None,
            course_ddeg: None,
            course_acc_ddeg: None,
            is_mock: false,
        }
    }

    #[test]
    fn is_jump_needs_speed_and_distance_and_a_recent_previous_fix() {
        // 1 e7 unit of latitude is 11.1 mm; 10_000 units = 111 m. Both accuracies 5 m: margin 110 m.
        let prev = trace_fix(0, 450_000_000, 70_000_000, 5000);
        // 111 m in 1 s: 111 m/s > 83 m/s and 111 m > 110 m -> jump.
        assert!(is_jump(&prev, &fix(1000, 450_010_000, 70_000_000, 5000)));
        // 100 m in 1 s: fast but inside the margin.
        assert!(!is_jump(&prev, &fix(1000, 450_009_000, 70_000_000, 5000)));
        // 111 m in 2 s: 55 m/s, not fast enough.
        assert!(!is_jump(&prev, &fix(2000, 450_010_000, 70_000_000, 5000)));
        // Same displacement but 60 s later is compared (dt == window), 60.001 s later is not.
        let far = 450_900_000; // 10 km: 166 m/s over 60 s
        assert!(is_jump(&prev, &fix(60_000, far, 70_000_000, 5000)));
        assert!(!is_jump(&prev, &fix(60_001, far, 70_000_000, 5000)));
        // dt <= 0 is not a jump (F08 handles ordering).
        assert!(!is_jump(&prev, &fix(0, far, 70_000_000, 5000)));
        assert!(!is_jump(&prev, &fix(-5, far, 70_000_000, 5000)));
        // A previous fix at an unsupported latitude cannot be framed: fail closed.
        let polar = trace_fix(0, 850_000_000, 0, 5000);
        assert!(is_jump(&polar, &fix(1000, 850_000_000, 0, 5000)));
        // Accuracy widens the margin by the sum of both.
        let prev60 = trace_fix(0, 450_000_000, 70_000_000, 60_000);
        assert!(!is_jump(
            &prev60,
            &fix(1000, 450_010_000, 70_000_000, 60_000)
        ));
    }

    #[test]
    fn is_stale_prunes_absent_entries_by_age_but_keeps_a_live_cool_down() {
        let now = 10_000_000;
        let cand = |last_t| DetState::Cand {
            n: 1,
            first_t: last_t,
            last_t,
        };
        assert!(
            !is_stale(&cand(now - DET_PRUNE_MS), now),
            "exactly 900 s is kept"
        );
        assert!(is_stale(&cand(now - DET_PRUNE_MS - 1), now));
        for st in [
            DetState::In {
                xn: 0,
                x_first_t: None,
                last_t: now - DET_PRUNE_MS - 1,
            },
            DetState::Armed {
                confirm_t: None,
                a_prev_mm: None,
                last_t: now - DET_PRUNE_MS - 1,
            },
            DetState::Latched {
                why: LatchWhy::Fired,
                last_t: now - DET_PRUNE_MS - 1,
            },
        ] {
            assert!(is_stale(&st, now));
        }
        // A cool-down is kept until it ends, however long the signpost has been absent.
        assert!(!is_stale(&DetState::Cool { until_ms: now + 1 }, now));
        assert!(is_stale(&DetState::Cool { until_ms: now }, now));
        // Saturation: a last_t far in the future is not stale, and no overflow at the extremes.
        assert!(!is_stale(&cand(i64::MAX), i64::MIN));
        assert!(is_stale(&cand(i64::MIN), i64::MAX));
    }

    #[test]
    fn t_hat_ordering_puts_absent_last() {
        assert_eq!(cmp_t_hat(Some(1), Some(2)), Ordering::Less);
        assert_eq!(cmp_t_hat(Some(2), Some(1)), Ordering::Greater);
        assert_eq!(cmp_t_hat(Some(2), Some(2)), Ordering::Equal);
        assert_eq!(cmp_t_hat(Some(i64::MAX), None), Ordering::Less);
        assert_eq!(cmp_t_hat(None, Some(i64::MIN)), Ordering::Greater);
        assert_eq!(cmp_t_hat(None, None), Ordering::Equal);
    }

    #[test]
    fn score_is_rank_times_confidence_over_1000_in_u64() {
        let ev = |conf| Event {
            kind: EventKind::Enter,
            t_ms: 0,
            d_rem_mm: None,
            t_hat_ms: None,
            dist_to_shape_mm: 0,
            path_tier: 0,
            path_conf_milli: conf,
        };
        let mut sp = general_signpost();
        let score = |sp: &Signpost, conf| {
            Cand {
                sp,
                ev: ev(conf),
                pre: None,
            }
            .score()
        };
        assert_eq!(score(&sp, 500), 500, "default rank 1000");
        sp.rank_milli = Some(1999);
        assert_eq!(score(&sp, 500), 999, "floor division");
        sp.rank_milli = Some(0);
        assert_eq!(score(&sp, 800), 0);
        sp.rank_milli = Some(u32::MAX);
        assert_eq!(score(&sp, u16::MAX), 281_470_681_677);
    }

    #[test]
    fn select_revisions_keeps_the_top_revision_and_drops_the_rest() {
        let mk = |id: &str, rev: u32| {
            let mut s = general_signpost();
            s.id = Id::new(id).unwrap();
            s.rev = rev;
            s
        };
        let input = [
            mk("b", 2),
            mk("a", 3),
            mk("a", 1),
            mk("b", 2),
            mk("c", 7),
            mk("a", 2),
        ];
        let (keep, drops) = select_revisions(&input, 42);
        let kept: Vec<_> = keep.iter().map(|s| (s.id.as_str(), s.rev)).collect();
        // `a` keeps rev 3; `b` has two copies of rev 2 (ambiguous: none kept); `c` keeps rev 7.
        assert_eq!(kept, [("a", 3), ("c", 7)]);
        let mut dropped: Vec<_> = drops.iter().map(|d| (d.signpost.as_str(), d.rev)).collect();
        dropped.sort();
        assert_eq!(dropped, [("a", 1), ("a", 2), ("b", 2), ("b", 2)]);
        assert!(
            drops
                .iter()
                .all(|d| d.reason == ReasonCode::TriggerInvalid && d.t_ms == 42)
        );
        // Order of the input does not matter.
        let mut rev = input.to_vec();
        rev.reverse();
        let (keep2, _) = select_revisions(&rev, 42);
        assert_eq!(
            keep2
                .iter()
                .map(|s| (s.id.as_str(), s.rev))
                .collect::<Vec<_>>(),
            kept
        );
        assert!(select_revisions(&[], 0).0.is_empty());
    }

    #[test]
    fn decisions_sort_by_id_then_rev_then_reason() {
        let sp = |id: &str, rev: u32| {
            let mut s = general_signpost();
            s.id = Id::new(id).unwrap();
            s.rev = rev;
            s
        };
        let mut v = alloc::vec![
            drop_decision(&sp("b", 1), ReasonCode::Expired, 0),
            drop_decision(&sp("a", 2), ReasonCode::Expired, 0),
            drop_decision(&sp("a", 1), ReasonCode::TriggerInvalid, 0),
            drop_decision(&sp("a", 1), ReasonCode::Expired, 0),
        ];
        sort_decisions(&mut v);
        let got: Vec<_> = v
            .iter()
            .map(|d| (d.signpost.as_str(), d.rev, d.reason))
            .collect();
        assert_eq!(
            got,
            [
                ("a", 1, ReasonCode::TriggerInvalid), // 201 < 203
                ("a", 1, ReasonCode::Expired),
                ("a", 2, ReasonCode::Expired),
                ("b", 1, ReasonCode::Expired)
            ]
        );
    }

    #[test]
    fn stop_wipes_mem_only_when_asked_and_never_touches_caps() {
        let mut st = State::empty();
        st.caps.hwm_ms = 77;
        st.caps.last_fire_ms = Some(5);
        st.mem.jump_rejects = 2;
        st.mem.trace.push(trace_fix(1, 0, 0, 0));
        let (o, kept) = stop(&st, 99, ReasonCode::FixStale, false);
        assert_eq!(kept, st);
        assert!(!o.fix.ok && o.fix.reason == ReasonCode::FixStale && o.t_ms == 99);
        let (_, wiped) = stop(&st, 99, ReasonCode::ConsentDenied, true);
        assert_eq!(wiped.mem, Mem::default());
        assert_eq!(wiped.caps, st.caps);
    }

    #[test]
    fn drop_and_fire_decisions_carry_the_event_facts() {
        let sp = general_signpost();
        let ev = Event {
            kind: EventKind::Approach,
            t_ms: 1234,
            d_rem_mm: Some(500),
            t_hat_ms: Some(40_000),
            dist_to_shape_mm: 3,
            path_tier: 1,
            path_conf_milli: 800,
        };
        let f = fire_decision(&sp, &ev);
        assert_eq!(f.outcome, Outcome::Fire);
        assert_eq!(f.reason, ReasonCode::Ok);
        assert_eq!(
            (
                f.event,
                f.t_ms,
                f.path_tier,
                f.path_conf_milli,
                f.d_rem_mm,
                f.t_hat_ms
            ),
            (EventKind::Approach, 1234, 1, 800, Some(500), Some(40_000))
        );
        assert_eq!(f.copy, proxim_types::Copy::Ahead);
        let d = event_drop(&sp, &ev, ReasonCode::LostSlot);
        assert_eq!((d.outcome, d.reason), (Outcome::Drop, ReasonCode::LostSlot));
        assert_eq!(d.d_rem_mm, Some(500));
        let s = drop_decision(&sp, ReasonCode::Expired, 9);
        assert_eq!(
            (
                s.outcome,
                s.t_ms,
                s.path_tier,
                s.path_conf_milli,
                s.d_rem_mm
            ),
            (Outcome::Drop, 9, 0, 0, None)
        );
        assert_eq!(s.copy, proxim_types::Copy::NearYou);
    }
}
