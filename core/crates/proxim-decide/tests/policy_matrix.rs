//! Policy narrowing and the I9 kernel rule: a child-directed app disables the engine.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_types::canon::to_canonical_json;
use proxim_types::{Mem, Policy, ReasonCode};

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
    // Spec 10: reserved fields parse and have no effect (and are not re-emitted).
    let policy: Policy = proxim_types::strict::from_json(
        br#"{"explain":true,"rot_lane":{"anything":[1,2,3]},"rot_lanes":7}"#,
    )
    .unwrap();
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
            "rot_days",
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
