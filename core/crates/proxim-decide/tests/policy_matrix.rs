//! Policy narrowing, the I9 kernel rule (a child-directed app disables the engine) and, from
//! engine spec 7.7 (engine_spec 2, M3 slice 6), the policy matrix as data, tested exhaustively:
//! 1. every cell of the table matches an independent statement of the plan's rules;
//! 2. every row decides at least one cell, and the last row is the fail-closed catch-all;
//! 3. `decide` agrees with the table on every cell it can see today;
//! 4. the published copy `core/policy/matrix.json` is byte-identical to the table.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::matrix::{self, Cell, RULES, Verdict};
use proxim_types::canon::to_canonical_json;
use proxim_types::{Class, ContentMode, Mem, Outcome, Policy, Profile, ReasonCode, Surface, Wake};
use std::path::PathBuf;

#[test]
fn child_directed_app_disables_engine() {
    let mut sim = Sim::new(vec![circle("c", 200), route("r", 2500, 30)]);
    // Build some caps and mem first (a fire), so that "wipes mem, keeps caps" is observable.
    let outs = sim.run(&visit(T0, 0, 0, 5, 0));
    assert!(!fire_indices(&outs).is_empty());
    assert!(!sim.state.mem.trace.is_empty());
    let caps_before = to_canonical_json(&sim.state.caps).unwrap();
    sim.policy.child_directed = Some(true);
    for k in 0..6 {
        let o = sim.step(&still(T0 + 600_000 + 4000 * k, 0, 0));
        assert!(!o.fix.ok);
        assert_eq!(o.fix.reason, ReasonCode::EngineDisabled);
        assert!(o.decisions.is_empty() && o.notes.is_none());
        assert_eq!(sim.state.mem, Mem::default(), "nothing is remembered");
        assert_eq!(to_canonical_json(&sim.state.caps).unwrap(), caps_before);
    }
    // The 30 minutes of cool-down have passed too; with the flag off the engine works again from
    // scratch: no candidate count survived the disabled period.
    sim.policy.child_directed = Some(false);
    let t = T0 + 4_000_000;
    let outs = sim.run(&visit(t, 0, 0, 4, 0));
    assert_eq!(fire_indices(&outs), vec![3]);
    // `None` means not child-directed.
    sim.policy.child_directed = None;
    assert!(sim.step(&still(t + 20_000, 0, 0)).fix.ok);
}

#[test]
fn child_directed_wins_over_every_later_filter() {
    let mut sim = Sim::new(vec![circle("c", 200)]);
    sim.policy.child_directed = Some(true);
    let mut bad = still(T0, 0, 0);
    bad.is_mock = true;
    bad.lat_e7 = 1_000_000_000;
    assert_eq!(sim.step(&bad).fix.reason, ReasonCode::EngineDisabled);
}

#[test]
fn simulator_mode_only_relaxes_the_mock_filter() {
    let mut sim = Sim::new(vec![circle("c", 200)]);
    sim.policy.simulator_mode = Some(true);
    let mut fixes = visit(T0, 0, 0, 5, 0);
    for f in &mut fixes {
        f.is_mock = true;
    }
    let outs = sim.run(&fixes);
    assert_eq!(fire_indices(&outs), vec![3]);
    // It does not relax accuracy, order or consent.
    let mut worse = still(T0 + 100_000, 0, 0);
    worse.is_mock = true;
    worse.acc_mm = 200_000;
    assert_eq!(
        sim.step(&worse).fix.reason,
        ReasonCode::FixAccuracyTooCoarse
    );
}

#[test]
fn lead_is_clamped_by_the_policy_not_by_the_signpost() {
    // lead_s 40 fires at about D = v x (lead + 3 s) = 15 x 43 = 645 m (fix 32). A policy minimum
    // of 60 s moves it to 15 x 63 = 945 m; a maximum of 20 s moves it to max(15 x 23, 150) = 345 m.
    let fire_at = |lo: Option<u32>, hi: Option<u32>| {
        let mut sim = Sim::new(vec![route("r", 2500, 30)]);
        sim.policy = Policy {
            lead_s_min: lo,
            lead_s_max: hi,
            ..explain()
        };
        let outs = sim.run(&drive_north(T0, 1_000_000, 15_000, 120, 0));
        let idx = fire_indices(&outs);
        assert_eq!(idx.len(), 1, "{lo:?} {hi:?}");
        let d = fires(&outs[idx[0]])[0].clone();
        d.d_rem_mm.unwrap()
    };
    let base = fire_at(None, None);
    let longer = fire_at(Some(60), None);
    let shorter = fire_at(None, Some(20));
    assert!((600_000..=645_000).contains(&base), "{base}");
    assert!((900_000..=945_000).contains(&longer), "{longer}");
    assert!((300_000..=345_000).contains(&shorter), "{shorter}");
    // Out-of-range policy values clamp to the platform 15..120 s range, never beyond.
    assert_eq!(fire_at(Some(1), Some(100_000)), base);
}

#[test]
fn the_authored_lead_must_be_in_range_even_if_the_policy_would_clamp_it() {
    let mut r = route("r", 2500, 30);
    r.trigger.lead_s = Some(500);
    let mut sim = Sim::new(vec![r]);
    sim.policy = Policy {
        lead_s_max: Some(120),
        ..explain()
    };
    let o = sim.step(&still(T0, 0, 0));
    assert!(has_drop(&o, "r", ReasonCode::TriggerInvalid));
}

#[test]
fn reserved_policy_and_signpost_fields_are_accepted_and_ignored() {
    // Spec 10: reserved fields parse and have no effect (and are not re-emitted). A lane has no
    // effect on a family without wordings (7.3).
    let policy: Policy =
        proxim_types::strict::from_json(br#"{"explain":true,"rot_lane":3,"rot_lanes":7}"#).unwrap();
    let sp: proxim_types::Signpost = {
        let mut v = serde_json::to_value(circle("c", 200)).unwrap();
        let o = v.as_object_mut().unwrap();
        for k in [
            "intent",
            "content_mode",
            "precision_class",
            "min_mode",
            "max_acc_m",
            "arm_scope",
            "relation",
            "selector",
            "live_at_ms",
            "fresh_required_s",
            "cooldown_s",
            "creatives",
            "pack_kind",
        ] {
            o.insert(k.into(), serde_json::json!({"x": [1, 2]}));
        }
        proxim_types::strict::from_json(v.to_string().as_bytes()).unwrap()
    };
    let mut sim = Sim::new(vec![sp]);
    sim.policy = policy;
    let outs = sim.run(&visit(T0, 0, 0, 4, 0));
    assert_eq!(fire_indices(&outs), vec![3]);
    let text = to_canonical_json(&sim.state).unwrap();
    assert!(!text.contains("rot_lane") && !text.contains("creatives"));
}

/// The plan's rules, written out on their own (PLAN 6.5, 6.6; Addendum 01 1, 5.1, 5.2; R4, R6).
/// Not derived from `RULES`: if the two disagree on any cell, one of them is wrong.
fn oracle(c: &Cell) -> bool {
    let health = matches!(c.class, Class::HealthAdjacent | Class::HealthFacility);
    !c.child_directed
        && c.class != Class::ProhibitedAnchor
        && c.mode == ContentMode::Hosted
        && c.wake == Wake::Foreground
        && c.surface == Surface::InApp
        && (!health || (c.profile == Profile::Sealed && c.health_tier))
}

#[test]
fn every_cell_matches_the_plan() {
    let cells = matrix::cells();
    assert_eq!(cells.len(), 2 * 2 * 2 * 4 * 2 * 2 * 2, "every combination");
    let mut allowed = 0;
    for c in &cells {
        let (rule, v) = matrix::lookup(c);
        assert_eq!(v == Verdict::Allow, oracle(c), "{c:?} decided by {rule}");
        allowed += usize::from(v == Verdict::Allow);
    }
    // General: 4 (profile x tier) in-app foreground hosted cells; health: 2 classes x 1.
    assert_eq!(allowed, 6);
}

#[test]
fn the_hard_rules_hold_in_every_cell() {
    for c in matrix::cells() {
        let v = matrix::lookup(&c).1;
        let health = matches!(c.class, Class::HealthAdjacent | Class::HealthFacility);
        if c.class == Class::ProhibitedAnchor || c.child_directed {
            assert_ne!(v, Verdict::Allow, "{c:?}");
        }
        if c.surface == Surface::OsNotification && (c.mode == ContentMode::Hosted || health) {
            assert_ne!(v, Verdict::Allow, "no hosted or health notification: {c:?}");
        }
        if c.mode == ContentMode::Hosted && c.wake == Wake::Background {
            assert_ne!(v, Verdict::Allow, "hosted background is off: {c:?}");
        }
        if c.profile == Profile::Sealed
            && (c.surface == Surface::OsNotification || c.wake == Wake::Background)
        {
            assert_ne!(
                v,
                Verdict::Allow,
                "a Sealed app: no notification, no background: {c:?}"
            );
        }
        if health && (c.profile != Profile::Sealed || !c.health_tier) {
            assert_ne!(
                v,
                Verdict::Allow,
                "health only in a Sealed health-tier app: {c:?}"
            );
        }
    }
}

#[test]
fn every_row_decides_a_cell_and_the_last_row_fails_closed() {
    let cells = matrix::cells();
    let last = RULES.last().unwrap();
    assert_eq!(last.id, "fail_closed");
    assert!(
        cells.iter().all(|c| last.matches(c)),
        "the catch-all matches every cell"
    );
    assert!(matches!(last.verdict, Verdict::Deny(_)));
    for r in &RULES[..RULES.len() - 1] {
        assert!(
            cells.iter().any(|c| matrix::lookup(c).0 == r.id),
            "row {} decides nothing (shadowed)",
            r.id
        );
    }
    let mut ids: Vec<_> = RULES.iter().map(|r| r.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), RULES.len(), "row ids are unique");
}

#[test]
fn the_legacy_health_class_reads_as_health_facility() {
    for c in matrix::cells()
        .into_iter()
        .filter(|c| c.class == Class::HealthFacility)
    {
        let legacy = Cell {
            class: Class::Health,
            ..c
        };
        assert_eq!(matrix::lookup(&legacy), matrix::lookup(&c));
    }
}

/// One approach drive past a signpost of `class` in a partner app with this policy: whether it
/// fired, the drop reasons, and the call-level reason of the first fix.
fn drive(class: Class, policy: Policy) -> (bool, Vec<ReasonCode>, ReasonCode) {
    let mut sp = route("s", 2500, 30);
    sp.class = class;
    sp.category = "health.clinic".into();
    let mut sim = Sim::new(vec![sp]);
    sim.consent = granted_health();
    sim.policy = policy;
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 40, 0));
    let fired = outs
        .iter()
        .any(|o| o.decisions.iter().any(|d| d.outcome == Outcome::Fire));
    let drops = outs.iter().flat_map(drop_reasons).map(|(_, r)| r).collect();
    (fired, drops, outs[0].fix.reason)
}

#[test]
fn decide_agrees_with_the_table_on_every_cell_it_can_see() {
    // v1: every signpost is hosted and every call is a foreground call.
    let mut seen = 0;
    for c in matrix::cells()
        .into_iter()
        .filter(|c| c.mode == ContentMode::Hosted && c.wake == Wake::Foreground)
    {
        let policy = Policy {
            surface: Some(c.surface),
            profile: Some(c.profile),
            health_tier: Some(c.health_tier),
            child_directed: Some(c.child_directed),
            ..explain()
        };
        let (fired, drops, call) = drive(c.class, policy);
        match matrix::lookup(&c).1 {
            Verdict::Allow => assert!(fired, "{c:?}: {drops:?}"),
            Verdict::Deny(ReasonCode::EngineDisabled) => {
                assert!(!fired);
                assert_eq!(call, ReasonCode::EngineDisabled, "{c:?}");
            }
            Verdict::Deny(reason) => {
                assert!(!fired, "{c:?}");
                assert!(
                    !drops.is_empty() && drops.iter().all(|r| *r == reason),
                    "{c:?}: {drops:?}"
                );
            }
        }
        seen += 1;
    }
    assert_eq!(seen, 2 * 4 * 2 * 2 * 2);
}

#[test]
fn defaults_are_an_in_app_standard_partner_without_the_health_tier() {
    let (fired, _, _) = drive(Class::General, explain());
    assert!(fired);
    let (fired, drops, _) = drive(Class::HealthAdjacent, explain());
    assert!(!fired && drops.contains(&ReasonCode::HealthPartnerNotEligible));
    assert!(drive(Class::HealthAdjacent, health_partner()).0);
    let notify = Policy {
        surface: Some(Surface::OsNotification),
        ..explain()
    };
    let (fired, drops, _) = drive(Class::General, notify);
    assert!(!fired && drops.contains(&ReasonCode::SurfaceNotAllowed));
}

#[test]
fn the_published_copy_is_byte_identical() {
    let path = std::env::var_os("PROXIM_CORE_DIR")
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            PathBuf::from,
        )
        .join("policy/matrix.json");
    let want = matrix::published_json().unwrap() + "\n";
    if std::env::var_os("PROXIM_BLESS").is_some() {
        std::fs::write(&path, &want).unwrap();
    }
    let got = std::fs::read_to_string(&path).unwrap();
    assert!(
        got == want,
        "core/policy/matrix.json is stale: regenerate with PROXIM_BLESS=1 and review the diff"
    );
}
