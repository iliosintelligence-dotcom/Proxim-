//! Engine spec 7.4 (engine_spec 2, M3 slice 3): health classes. `health_adjacent` and
//! `health_facility` share the health gates; `health_facility` (and the legacy `health`) leave
//! nothing in the persisted state (I7, owner decision Option A); `prohibited_anchor` never fires.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::controls::apply;
use proxim_types::{Class, Clock, ConsentValue, Control, Id, Outcome, ReasonCode, Signpost, State};

const HOUR: i64 = 3_600_000;

fn health_route(id: &str, class: Class) -> Signpost {
    let mut sp = route(id, 2500, 30);
    sp.class = class;
    sp.category = "health.clinic".into();
    sp
}

/// One drive north along the route starting at `t0`, then a few fixes 15 km south so the
/// approach detector releases: the fired ids and every drop.
fn drive(sim: &mut Sim, t0: i64) -> (Vec<String>, Vec<(String, ReasonCode)>) {
    let mut fixes = drive_north(t0, 1_400_000, 15_000, 40, 0);
    fixes.extend((0..5).map(|k| still(t0 + 120_000 + k * 60_000, 0, -15_000_000)));
    let outs = sim.run(&fixes);
    let fired = outs
        .iter()
        .flat_map(|o| o.decisions.iter())
        .filter(|d| d.outcome == Outcome::Fire)
        .map(|d| d.signpost.as_str().to_string())
        .collect();
    (fired, outs.iter().flat_map(drop_reasons).collect())
}

fn health_sim(sps: Vec<Signpost>) -> Sim {
    let mut sim = Sim::new(sps);
    sim.consent = granted_health();
    sim
}

/// The parts of the state that may be persisted, with the time high-water mark set aside (it
/// moves on every call, whatever fires).
fn persisted(st: &State) -> String {
    let mut caps = st.caps.clone();
    caps.hwm_ms = 0;
    proxim_types::canon::to_canonical_json(&(caps, &st.controls, &st.rotation)).unwrap()
}

#[test]
fn a_facility_fire_changes_nothing_that_is_persisted() {
    for class in [Class::HealthFacility, Class::Health] {
        let mut sim = health_sim(vec![health_route("clinic", class)]);
        let before = persisted(&sim.state);
        let (fired, _) = drive(&mut sim, T0);
        assert_eq!(fired, ["clinic"], "{class:?}");
        assert_eq!(
            persisted(&sim.state),
            before,
            "{class:?}: nothing about the clinic advert on disk"
        );
        let f = &sim.state.mem.facility;
        assert_eq!((f.signposts.len(), f.senders.len()), (1, 1));
        assert!(f.last_fire_ms.is_some());
    }
}

#[test]
fn facility_caps_hold_within_a_session_and_reset_on_restart() {
    let mut sim = health_sim(vec![health_route("clinic", Class::HealthFacility)]);
    let mut fires = 0;
    for k in 0..3 {
        fires += drive(&mut sim, T0 + k * HOUR).0.len();
    }
    assert_eq!(fires, 2, "2 a day, counted in memory");
    let (_, drops) = drive(&mut sim, T0 + 3 * HOUR);
    assert!(
        drops.contains(&("clinic".into(), ReasonCode::CapSignpostDay)),
        "{drops:?}"
    );
    // A new process starts from the persisted state only: the memory-only count is gone (Option A).
    sim.state.mem = proxim_types::Mem::default();
    assert_eq!(drive(&mut sim, T0 + 4 * HOUR).0, ["clinic"]);
}

#[test]
fn facility_fires_still_count_for_the_app_and_the_sender() {
    let mut general = route("cafe", 2500, 30);
    general.sender = "clinic-org".into();
    let mut clinic = health_route("clinic", Class::HealthFacility);
    clinic.sender = "clinic-org".into();
    clinic.rank_milli = Some(3000);
    let mut sim = health_sim(vec![clinic, general]);
    assert_eq!(drive(&mut sim, T0).0, ["clinic"]);
    // 10 minutes later the cafe would fire, but the app interval counts the clinic fire.
    let (fired, drops) = drive(&mut sim, T0 + 600_000);
    assert!(fired.is_empty());
    assert!(
        drops.iter().any(|(_, r)| *r == ReasonCode::CapAppInterval),
        "{drops:?}"
    );
}

#[test]
fn health_adjacent_is_gated_like_health_but_counted_normally() {
    let mut sim = health_sim(vec![health_route("pharmacy", Class::HealthAdjacent)]);
    assert_eq!(drive(&mut sim, T0).0, ["pharmacy"]);
    assert_eq!(
        sim.state.caps.signposts.len(),
        1,
        "persisted like any advert"
    );
    assert!(sim.state.mem.facility.is_empty());
    // Without the health consent it never fires.
    let mut sim = Sim::new(vec![health_route("pharmacy", Class::HealthAdjacent)]);
    let (fired, drops) = drive(&mut sim, T0);
    assert!(fired.is_empty());
    assert!(drops.contains(&("pharmacy".into(), ReasonCode::HealthConsentMissing)));
}

#[test]
fn a_prohibited_anchor_never_fires() {
    let mut sim = health_sim(vec![health_route("shelter", Class::ProhibitedAnchor)]);
    let (fired, drops) = drive(&mut sim, T0);
    assert!(fired.is_empty());
    assert!(
        drops.contains(&("shelter".into(), ReasonCode::ProhibitedAnchor)),
        "{drops:?}"
    );
    assert!(sim.state.caps.signposts.is_empty() && sim.state.mem.facility.is_empty());
}

#[test]
fn facility_wordings_rotate_in_memory_only() {
    let mut sp = health_route("clinic", Class::HealthFacility);
    sp.wordings = Some(proxim_types::Wordings {
        digests: vec!["11".repeat(32), "22".repeat(32), "33".repeat(32)],
        avail_mask: 0b111,
    });
    let mut sim = health_sim(vec![sp]);
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 40, 0));
    let fire = outs
        .iter()
        .flat_map(|o| o.decisions.iter())
        .find(|d| d.outcome == Outcome::Fire)
        .unwrap();
    assert!(fire.wording.is_some());
    assert!(sim.state.rotation.is_empty(), "no wording history on disk");
    assert_eq!(sim.state.mem.facility.rotation.rows.len(), 1);
}

#[test]
fn memory_only_controls_block_and_are_never_persisted() {
    let mut sim = health_sim(vec![health_route("clinic", Class::HealthFacility)]);
    let clock = Clock {
        now_ms: T0,
        tz_offset_min: 0,
    };
    sim.state = apply(
        &sim.state,
        &Control::Hide {
            id: Id::new("clinic").unwrap(),
            memory_only: true,
        },
        &clock,
    )
    .unwrap();
    assert!(sim.state.controls.is_empty(), "nothing persisted");
    assert_eq!(sim.state.mem.facility.controls.hidden.len(), 1);
    let (fired, drops) = drive(&mut sim, T0);
    assert!(fired.is_empty());
    assert!(drops.contains(&("clinic".into(), ReasonCode::HiddenByUser)));
}

#[test]
fn a_standoff_and_a_withdrawal_both_wipe_the_facility_memory() {
    let mut sim = health_sim(vec![health_route("clinic", Class::HealthFacility)]);
    drive(&mut sim, T0);
    assert!(!sim.state.mem.facility.is_empty());
    // I6: inside a standoff nothing at all is kept in memory, the facility record included.
    sim.policy.standoff_zones = vec![proxim_types::StandoffZone {
        id: Id::new("z").unwrap(),
        center: pos(0, 2_000_000),
        standoff_m: None,
    }];
    let out = sim.step(&still(T0 + HOUR, 0, 2_000_000));
    assert_eq!(out.fix.reason, ReasonCode::SensitiveStandoff);
    assert_eq!(sim.state.mem, proxim_types::Mem::default());
    sim.policy.standoff_zones.clear();
    drive(&mut sim, T0 + 2 * HOUR);
    assert!(!sim.state.mem.facility.is_empty());
    sim.consent.location_matching = ConsentValue::Denied;
    sim.step(&still(T0 + 3 * HOUR, 0, 0));
    assert!(sim.state.mem.facility.is_empty());
}
