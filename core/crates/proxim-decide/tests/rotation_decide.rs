//! Engine spec 7.3 (engine_spec 2, M3 slice 2): wording rotation inside `decide` (step 7b).
//! The full engine, driven by real visits, must pick the wordings the approved reference
//! (`core/testdata/golden/rotation/golden_v2.json`) picks, and keep its ledger.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::controls::apply;
use proxim_types::{Clock, Control, Id, Outcome, Policy, ReasonCode, Signpost, Wordings};
use serde_json::{Value, json};
use std::path::PathBuf;

/// A Monday 42 days (a multiple of 7 and of the 6-wording cycle) times 493 after rotation day 0,
/// so reference day d replays as rotation day `BASE + d` with the same weekday and slot phase.
const BASE: i64 = 42 * 493;

fn golden() -> Value {
    let p = std::env::var_os("PROXIM_CORE_DIR")
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            PathBuf::from,
        )
        .join("testdata/golden/rotation/golden_v2.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn trace(name: &str) -> Value {
    golden()["select"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap()
        .clone()
}

/// The brew family as a signpost: its id, digests and weekday mask from the reference trace.
fn family_signpost(t: &Value, avail_mask: u8) -> Signpost {
    let id = String::from_utf8(
        (0..t["signpost_id_hex"].as_str().unwrap().len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&t["signpost_id_hex"].as_str().unwrap()[i..i + 2], 16).unwrap()
            })
            .collect(),
    )
    .unwrap();
    let mut sp = circle(&id, 200);
    sp.valid_until_ms = i64::MAX;
    sp.rot_days = Some(u8::try_from(t["dow_mask"].as_u64().unwrap()).unwrap());
    sp.wordings = Some(Wordings {
        digests: t["digests_hex"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d.as_str().unwrap().into())
            .collect(),
        avail_mask,
    });
    sp
}

/// 08:00 UTC on reference day `d`.
fn morning(d: i64) -> i64 {
    (BASE + d + 4) * DAY + 8 * 3_600_000
}

/// One visit on reference day `d`: the fired wording, if any.
fn visit_day(sim: &mut Sim, d: i64) -> Option<u8> {
    let release = !sim.state.mem.det.is_empty();
    let outs = sim.run(&enter_cycle(morning(d), release));
    outs.iter()
        .flat_map(|o| o.decisions.iter())
        .find(|x| x.outcome == Outcome::Fire)
        .map(|x| {
            x.wording
                .expect("a family with wordings fires with a wording")
        })
}

fn ledger_snapshot(sim: &Sim) -> Value {
    let rows: Vec<Value> = sim
        .state
        .rotation
        .rows
        .iter()
        .map(|r| json!([r.vid, r.count, r.last_day - BASE]))
        .collect();
    let f = &sim.state.rotation.families[0];
    json!({"rows": rows, "recent": f.recent, "last_day": f.last_day - BASE})
}

fn replay(name: &str, policy: Policy) {
    let t = trace(name);
    let mut sim = Sim::new(vec![family_signpost(&t, 0b11_1111)]);
    sim.policy = policy;
    for op in t["ops"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["op"] == "select")
    {
        let d = op["day"].as_i64().unwrap();
        let want = u8::try_from(op["expect_idx"].as_u64().unwrap()).unwrap();
        assert_eq!(visit_day(&mut sim, d), Some(want), "{name} day {d}");
    }
    let mut final_rows = t["final"].clone();
    // The reference lists rows by vid; the state lists them by (id, vid): the same order here.
    final_rows["rows"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|r| r[0].as_u64().unwrap());
    assert_eq!(ledger_snapshot(&sim), final_rows, "{name}: ledger");
}

#[test]
fn decide_reproduces_the_reference_rotation_weekdays() {
    replay("n6_weekdays_daily", explain());
}

#[test]
fn decide_reproduces_the_reference_rotation_on_lane_1_of_2() {
    replay(
        "n6_weekdays_daily_lane1of2",
        Policy {
            rot_lane: Some(1),
            rot_lanes: Some(2),
            ..explain()
        },
    );
}

#[test]
fn a_family_without_wordings_fires_without_one_and_writes_no_ledger() {
    let mut sim = Sim::new(vec![circle("plain", 200)]);
    let outs = sim.run(&enter_cycle(T0, false));
    let fire = outs
        .iter()
        .flat_map(|o| o.decisions.iter())
        .find(|d| d.outcome == Outcome::Fire)
        .unwrap();
    assert_eq!(fire.wording, None);
    assert!(sim.state.rotation.is_empty());
}

#[test]
fn only_available_wordings_are_shown() {
    let t = trace("n6_weekdays_daily");
    let mut sim = Sim::new(vec![family_signpost(&t, 0b10_0101)]);
    for d in [0, 1, 2, 3, 4, 7, 8, 9] {
        let w = visit_day(&mut sim, d).unwrap();
        assert!(
            0b10_0101 >> w & 1 == 1,
            "day {d}: wording {w} is not available"
        );
    }
}

#[test]
fn consent_withdrawal_wipes_the_ledger_and_a_standoff_writes_nothing() {
    let t = trace("n6_weekdays_daily");
    let mut sim = Sim::new(vec![family_signpost(&t, 0b11_1111)]);
    visit_day(&mut sim, 0).unwrap();
    assert!(!sim.state.rotation.is_empty());
    let kept = sim.state.rotation.clone();
    // A fix inside a standoff zone: the call ends before anything is learned (I6, 5.2).
    sim.policy.standoff_zones = vec![proxim_types::StandoffZone {
        id: Id::new("z").unwrap(),
        center: proxim_types::Position {
            lat_e7: 450_000_000,
            lon_e7: 70_000_000,
        },
        standoff_m: None,
    }];
    let out = sim.step(&still(morning(1), 0, 0));
    assert_eq!(out.fix.reason, ReasonCode::SensitiveStandoff);
    assert_eq!(sim.state.rotation, kept);
    sim.policy.standoff_zones.clear();
    sim.consent.location_matching = proxim_types::ConsentValue::Denied;
    sim.step(&still(morning(1) + 60_000, 0, 0));
    assert!(
        sim.state.rotation.is_empty(),
        "withdrawal wipes the wording ledger"
    );
}

#[test]
fn dismiss_sends_the_shown_wording_to_the_back() {
    let t = trace("n6_weekdays_daily");
    let mut sim = Sim::new(vec![family_signpost(&t, 0b11_1111)]);
    visit_day(&mut sim, 0).unwrap();
    let id = sim.signposts[0].id.clone();
    let before = sim.state.rotation.rows.clone();
    let clock = Clock {
        now_ms: morning(0) + 600_000,
        tz_offset_min: 0,
    };
    sim.state = apply(&sim.state, &Control::Dismiss { id: id.clone() }, &clock).unwrap();
    let shown = *sim.state.rotation.families[0].recent.last().unwrap();
    let after = sim
        .state
        .rotation
        .rows
        .iter()
        .find(|r| r.vid == shown)
        .unwrap();
    let was = before.iter().find(|r| r.vid == shown).unwrap();
    assert_eq!(
        after.count,
        was.count + 512,
        "dismiss = cn + 512 on the same day"
    );
    assert!(sim.state.controls.dismissed.iter().any(|d| d.id == id));
}

#[test]
fn malformed_wordings_fail_closed_and_bad_lanes_are_a_policy_error() {
    let t = trace("n6_weekdays_daily");
    let base = family_signpost(&t, 0b11_1111);
    let mut bad: Vec<Signpost> = Vec::new();
    for (k, (digests, mask)) in [
        (vec!["00".repeat(32)], 0b10u8), // wording 0 unavailable
        (vec!["0".repeat(64); 2], 0b11), // vid collision
        (vec!["ZZ".repeat(32)], 0b1),    // not hex
        (vec!["11".repeat(32)], 0b11),   // mask beyond the family
        (vec![], 0b1),                   // no wordings
    ]
    .into_iter()
    .enumerate()
    {
        let mut sp = base.clone();
        sp.id = Id::new(&format!("bad-{k}")).unwrap();
        sp.wordings = Some(Wordings {
            digests,
            avail_mask: mask,
        });
        bad.push(sp);
    }
    let mut sim = Sim::new(bad);
    let outs = sim.run(&enter_cycle(morning(0), false));
    let drops: Vec<_> = outs.iter().flat_map(drop_reasons).collect();
    for k in 0..5 {
        assert!(
            drops.contains(&(format!("bad-{k}"), ReasonCode::TriggerInvalid)),
            "bad-{k}: {drops:?}"
        );
    }
    let mut sim = Sim::new(vec![base]);
    sim.policy = Policy {
        rot_lane: Some(2),
        rot_lanes: Some(2),
        ..explain()
    };
    let clock = Clock {
        now_ms: morning(0),
        tz_offset_min: 0,
    };
    let fix = still(morning(0), 0, 0);
    assert!(
        proxim_decide::decide(
            &sim.state,
            &fix,
            &sim.signposts,
            &sim.policy,
            &sim.consent,
            &clock,
            0
        )
        .is_err()
    );
}

#[test]
fn the_ledger_is_bounded_globally_and_evicts_the_oldest_row_first() {
    // 400 families of 6 wordings each, every one shown once on one of 400 days: 2,400 rows would
    // be written, the state keeps 2,000, and the survivors are the newest.
    let t = trace("n6_weekdays_daily");
    let mut st = proxim_types::State::empty();
    let mut fams = Vec::new();
    for k in 0..400u32 {
        let mut sp = family_signpost(&t, 0b11_1111);
        sp.id = Id::new(&format!("f{k:04}")).unwrap();
        fams.push(sp);
    }
    // Build the ledger directly through the public kernel and the state shape decide reads.
    let mut rows = Vec::new();
    let mut families = Vec::new();
    for (k, sp) in fams.iter().enumerate() {
        let fam = proxim_decide::rotation::family_of(sp.id.as_str(), sp.wordings.as_ref().unwrap())
            .unwrap();
        for v in &fam.vids {
            rows.push(proxim_types::RotRow {
                id: sp.id.clone(),
                vid: *v,
                count: 256,
                last_day: BASE + k as i64 / 6,
            });
        }
        families.push(proxim_types::RotFamily {
            id: sp.id.clone(),
            recent: vec![fam.vids[0]],
            last_day: BASE + k as i64 / 6,
        });
    }
    rows.sort_by(|a, b| (&a.id, a.vid).cmp(&(&b.id, b.vid)));
    rows.truncate(2000);
    st.rotation = proxim_types::Rotation { rows, families };
    // A fire of a new family writes one more row; the global limit evicts one old row.
    let mut sim = Sim::new(vec![family_signpost(&t, 0b11_1111)]);
    sim.state = st;
    let before = sim.state.rotation.rows.len();
    assert_eq!(before, 2000);
    visit_day(&mut sim, 70).unwrap();
    assert_eq!(sim.state.rotation.rows.len(), 2000);
    let fresh_id = sim.signposts[0].id.as_str().to_string();
    assert!(
        sim.state
            .rotation
            .rows
            .iter()
            .any(|r| r.id.as_str() == fresh_id),
        "the row just written survives"
    );
}
