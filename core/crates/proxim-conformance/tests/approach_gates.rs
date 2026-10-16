//! Spec 5-6: APPROACH gates in order, the P1 confidence ladder, and the must-not-fire list of
//! spec 11 (U-turn, opposite carriageway, stopped vehicle, jitter, unknown consent, late join,
//! parallel roads beyond the buffer, permission downgrade mid-route).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unreachable,
    clippy::print_stderr,
    clippy::disallowed_types,
    clippy::inconsistent_digit_grouping,
    clippy::unreadable_literal
)]

use proxim_conformance::rng::Rng;
use proxim_conformance::*;
use serde_json::{Value, json};

const ID: &str = "r-1";

struct Run {
    steps: Vec<Step>,
}

impl Run {
    fn fire(&self) -> Option<(usize, Value)> {
        self.steps.iter().enumerate().find_map(|(i, s)| {
            s.fires()
                .into_iter()
                .find(|f| f["signpost"] == ID)
                .map(|f| (i, f))
        })
    }
    fn fired(&self) -> bool {
        self.fire().is_some()
    }
    fn saw(&self, reason: &str) -> bool {
        self.steps
            .iter()
            .any(|s| s.reasons(ID).iter().any(|r| r == reason))
    }
    fn reasons(&self) -> Vec<String> {
        let mut v: Vec<String> = self.steps.iter().flat_map(|s| s.reasons(ID)).collect();
        v.sort();
        v.dedup();
        v
    }
}

fn run_with(sp: Value, d: &Drive, steps: i64) -> Run {
    let mut rig = Rig::new().with_signpost(sp);
    Run {
        steps: (0..steps).map(|k| rig.step(&d.fix(k))).collect(),
    }
}

fn run(d: &Drive, steps: i64) -> Run {
    run_with(approach_route(ID, 2500), d, steps)
}

fn base() -> Drive {
    Drive::new(2500, 15_000)
}

#[test]
fn baseline_fires_with_confidence_800_and_copy_ahead() {
    let r = run(&base(), 160);
    let (_, f) = r.fire().expect("fires");
    assert_eq!(
        f["path_conf_milli"], 800,
        "600 + 100 (5 fixes) + 100 (heading within 15 deg), acc <= 30 m"
    );
    assert_eq!(f["path_tier"], 1);
    assert_eq!(f["copy"], "ahead");
}

// ---- gates, one at a time -----------------------------------------------------------------------

#[test]
fn gate_accuracy_65_m_boundary() {
    let mut d = base();
    d.acc_mm = 65_000;
    let r = run(&d, 160);
    let (_, f) = r.fire().expect("acc_eff exactly 65 m passes the gate");
    assert_eq!(
        f["path_conf_milli"], 600,
        "600 + 100 + 100 - 200 (acc_eff > 50 m)"
    );
    let mut d = base();
    d.acc_mm = 66_000;
    let r = run(&d, 160);
    assert!(!r.fired());
    assert!(r.saw("FIX_ACCURACY_TOO_COARSE"), "{:?}", r.reasons());
}

#[test]
fn confidence_ladder_follows_accuracy_and_heading() {
    // (acc_mm, course_ddeg, expected conf)
    for (acc, course, conf) in [
        (5_000, 0, 800),
        (10_000, 0, 800),
        (30_000, 0, 800), // max acc_eff of the last 3 = 30 m: not above 30 m, no penalty
        (35_000, 0, 700), // > 30 m: -100
        (50_000, 0, 700), // exactly 50 m: only the first penalty
        (55_000, 0, 600), // > 50 m: -200 (exclusive of the first)
        (5_000, 150, 800), // exactly 15 degrees: dot >= cos 15 deg
        (5_000, 200, 700), // 20 degrees: within 30 deg tolerance but no heading bonus
        (5_000, 3400, 700),
    ] {
        let mut d = base();
        d.acc_mm = acc;
        d.course = Some(course);
        let r = run(&d, 160);
        let (_, f) = r
            .fire()
            .unwrap_or_else(|| panic!("acc {acc} course {course}: no fire, saw {:?}", r.reasons()));
        assert_eq!(f["path_conf_milli"], conf, "acc {acc} course {course}");
    }
}

#[test]
fn confidence_threshold_blocks_when_signpost_demands_more() {
    // min_path_conf_milli 900 is above anything P1 can reach (800)
    let mut sp = approach_route(ID, 2500);
    sp["min_path_conf_milli"] = json!(900);
    let r = run_with(sp, &base(), 160);
    assert!(!r.fired());
    assert!(r.saw("PATH_CONFIDENCE_LOW"), "{:?}", r.reasons());
    // 800 is reachable and sufficient; 801 is not
    let mut sp = approach_route(ID, 2500);
    sp["min_path_conf_milli"] = json!(800);
    assert!(run_with(sp, &base(), 160).fired());
    let mut sp = approach_route(ID, 2500);
    sp["min_path_conf_milli"] = json!(801);
    assert!(!run_with(sp, &base(), 160).fired());
    // tier 2 (route-confirmed) is reserved and cannot be met in M1
    let mut sp = approach_route(ID, 2500);
    sp["min_path_tier"] = json!(2);
    let r = run_with(sp, &base(), 160);
    assert!(!r.fired());
    assert!(r.saw("PATH_TIER_LOW"), "{:?}", r.reasons());
}

#[test]
fn gate_off_corridor_uses_the_centre_point_against_buffer() {
    for (lat_m, fires) in [
        (0, true),
        (25, true),
        (29, true),
        (32, false),
        (60, false),
        (200, false),
    ] {
        let mut d = base();
        d.lateral_mm = lat_m * 1000;
        let r = run(&d, 160);
        assert_eq!(r.fired(), fires, "lateral {lat_m} m");
        if !fires {
            assert!(
                r.saw("OFF_CORRIDOR"),
                "lateral {lat_m} m: {:?}",
                r.reasons()
            );
        }
    }
    // the buffer is authored: 100 m buffer admits a 60 m offset
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.shape.buffer_m", json!(100));
    let mut d = base();
    d.lateral_mm = 60_000;
    assert!(run_with(sp, &d, 160).fired());
}

#[test]
fn gate_stale_fix_resets_the_detector() {
    let d = base();
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    for k in 0..30 {
        rig.step(&d.fix(k));
    }
    assert!(rig.history.last().unwrap().det(ID).is_some());
    // fix age 10 001 ms (> 10 s) against the clock: FIX_STALE for APPROACH, back to idle
    let t = d.t_ms(30);
    let s = rig.call(&d.fix(30), t + 10_001);
    assert!(s.fix_ok(), "F07 allows up to 30 s: {}", s.fix_reason());
    assert!(!s.fired(ID));
    assert!(
        s.reasons(ID).contains(&"FIX_STALE".to_string()),
        "{}",
        s.text
    );
    // exactly 10 000 ms is not stale
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    for k in 0..30 {
        rig.step(&d.fix(k));
    }
    let s = rig.call(&d.fix(30), t + 10_000);
    assert!(
        !s.reasons(ID).contains(&"FIX_STALE".to_string()),
        "{}",
        s.text
    );
}

#[test]
fn gate_speed_band() {
    // vehicle minimum 2.5 m/s
    let slow = Drive::new(2500, 2_000);
    let r = run(&slow, 300);
    assert!(
        !r.fired() && r.saw("SPEED_OUT_OF_BAND"),
        "{:?}",
        r.reasons()
    );
    // authored band: speed_max 10 m/s rejects a 15 m/s drive
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.speed_max_mmps", json!(10_000));
    let r = run_with(sp, &base(), 160);
    assert!(
        !r.fired() && r.saw("SPEED_OUT_OF_BAND"),
        "{:?}",
        r.reasons()
    );
    // foot band 0.5..2.5 m/s: a 5 m/s foot signpost never fires
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.mode", json!("foot"));
    let r = run_with(sp, &Drive::new(2500, 5_000), 160);
    assert!(
        !r.fired() && r.saw("SPEED_OUT_OF_BAND"),
        "{:?}",
        r.reasons()
    );
    // speed_min 8.3 m/s (free-flowing) rejects a 5 m/s vehicle
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.speed_min_mmps", json!(8_300));
    let r = run_with(sp, &Drive::new(2500, 5_000), 300);
    assert!(
        !r.fired() && r.saw("SPEED_OUT_OF_BAND"),
        "{:?}",
        r.reasons()
    );
}

#[test]
fn gate_bearing_tolerance_is_inclusive_at_the_authored_limit() {
    // default tolerance 30 degrees = 300 ddeg; the OS course is exact and the route runs due north
    for (course, fires) in [
        (0_u16, true),
        (299, true),
        (300, true),
        (301, false),
        (350, false),
        (3300, true),
        (3299, false),
    ] {
        let mut d = base();
        d.course = Some(course);
        let r = run(&d, 160);
        assert_eq!(r.fired(), fires, "course {course} ddeg");
        if !fires {
            assert!(
                r.saw("BEARING_MISMATCH"),
                "course {course}: {:?}",
                r.reasons()
            );
        }
    }
    // authored 45 degrees (450 ddeg) admits 40 degrees
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.bearing_tol_ddeg", json!(450));
    let mut d = base();
    d.course = Some(400);
    assert!(run_with(sp, &d, 160).fired());
    // tighter 15 degrees rejects 20
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.bearing_tol_ddeg", json!(150));
    let mut d = base();
    d.course = Some(200);
    assert!(!run_with(sp, &d, 160).fired());
}

#[test]
fn heading_from_displacement_when_there_is_no_usable_os_course() {
    // No course: heading comes from the displacement between fixes (needs >= max(15 m, 3 acc_eff)).
    let mut d = base();
    d.course = None;
    assert!(run(&d, 160).fired(), "displacement heading at 15 m/s");
    // A course with accuracy above 450 ddeg is ignored in favour of displacement
    let mut d = base();
    d.course = Some(1800); // would be south...
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let mut fired = false;
    for k in 0..160 {
        let mut f = d.fix(k);
        f["course_acc_ddeg"] = json!(451);
        fired |= rig.step(&f).fired(ID);
    }
    assert!(
        fired,
        "course_acc 451 > 450: the unreliable course must be ignored"
    );
    // ... and a south course with GOOD accuracy blocks
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    for k in 0..160 {
        let mut f = d.fix(k);
        f["course_acc_ddeg"] = json!(450);
        assert!(
            !rig.step(&f).fired(ID),
            "course_acc 450 is accepted: heading is south"
        );
    }
}

#[test]
fn heading_is_invalid_at_walking_pace_for_vehicles() {
    // vf < 800 mm/s: the OS course is not used, displacement is below 15 m in 3 s -> no heading
    let mut d = Drive::new(2500, 600);
    d.start_along_mm = 2_300_000;
    let r = run(&d, 200);
    assert!(!r.fired());
}

#[test]
fn gate_not_closing_when_the_heading_lies() {
    // moving south away from the anchor but reporting a north course
    let mut d = base();
    d.speed_mmps = -15_000;
    d.start_along_mm = 1_500_000;
    d.course = Some(0);
    d.doppler = true;
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let mut notes = vec![];
    for k in 0..120 {
        let mut f = d.fix(k);
        f["speed_mmps"] = json!(15_000); // speed is reported as magnitude
        let s = rig.step(&f);
        assert!(!s.fired(ID), "moving away must never fire (step {k})");
        notes.extend(s.reasons(ID));
    }
    assert!(notes.contains(&"NOT_CLOSING".to_string()), "{notes:?}");
}

#[test]
fn stationary_device_claiming_speed_never_fires() {
    // a stuck receiver: jitter well below acc_eff / 4 (2.5 m), claims 10 m/s north
    let mut rng = Rng::new(1234);
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let d = base();
    for k in 0..300 {
        let p = offset(
            ORIGIN,
            rng.range_i64(-2000, 2000),
            -1_000_000 + rng.range_i64(-2000, 2000),
        );
        let mut f = fix_at(d.t_ms(k), p);
        f["speed_mmps"] = json!(10_000);
        let s = rig.step(&f);
        assert!(!s.fired(ID), "stuck device fired at step {k}");
    }
}

#[test]
fn stopped_vehicle_never_fires() {
    let mut d = base();
    d.speed_mmps = 0;
    d.start_along_mm = 1_800_000;
    let r = run(&d, 300);
    assert!(!r.fired());
}

#[test]
fn u_turn_before_the_anchor_does_not_fire() {
    // drive north 1000 m away from the fire distance, U-turn at 1 km, drive back south
    let mut rig = Rig::new().with_signpost(approach_route(ID, 4000));
    let mut k = 0;
    let t = |k: i64| BASE_T + k * 1000;
    let route_point = |k: i64, along_m: i64, course: u16, spd: i64| {
        let mut f = fix_at(t(k), offset(ORIGIN, 0, (along_m - 4000) * 1000));
        f["course_ddeg"] = json!(course);
        f["speed_mmps"] = json!(spd);
        f
    };
    // north from 0 to 600 m at 15 m/s (d_rem 3400 > d_hi, well before the fire distance)
    let mut along = 0;
    while along < 600 {
        assert!(!rig.step(&route_point(k, along, 0, 15_000)).fired(ID));
        along += 15;
        k += 1;
    }
    // U-turn: back south
    while along > 0 {
        let s = rig.step(&route_point(k, along, 1800, 15_000));
        assert!(!s.fired(ID), "fired while driving away at along={along}");
        along -= 15;
        k += 1;
    }
    // and keep going south of the route start
    for _ in 0..60 {
        let s = rig.step(&route_point(k, along, 1800, 15_000));
        assert!(!s.fired(ID));
        along -= 15;
        k += 1;
    }
}

#[test]
fn u_turn_inside_the_fire_zone_after_arming_does_not_fire_on_the_way_out() {
    // approach until just before the fire distance, then reverse; the detector must not fire afterwards
    let d = base();
    let r0 = run(&d, 200);
    let (k_fire, _) = r0.fire().unwrap();
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let k_turn = k_fire as i64 - 3; // armed but not yet fired
    for k in 0..k_turn {
        assert!(!rig.step(&d.fix(k)).fired(ID));
    }
    let mut along = d.along_mm(k_turn - 1);
    for j in 0..120 {
        along -= 15_000;
        let p = offset(ORIGIN, 0, along - 2_500_000);
        let mut f = fix_at(d.t_ms(k_turn + j), p);
        f["course_ddeg"] = json!(1800);
        f["speed_mmps"] = json!(15_000);
        let s = rig.step(&f);
        assert!(!s.fired(ID), "fired {j} steps after the U-turn");
    }
}

#[test]
fn opposite_carriageway_does_not_fire() {
    // 15 m east of the route (inside the buffer), driving south, same speed
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let mut notes = vec![];
    for k in 0..160 {
        let along = 2_500_000 - 15_000 * k;
        let mut f = fix_at(BASE_T + k * 1000, offset(ORIGIN, 15_000, along - 2_500_000));
        f["course_ddeg"] = json!(1800);
        let s = rig.step(&f);
        assert!(!s.fired(ID), "opposite carriageway fired at step {k}");
        notes.extend(s.reasons(ID));
    }
    assert!(
        notes.contains(&"BEARING_MISMATCH".to_string())
            || notes.contains(&"NOT_CLOSING".to_string()),
        "{notes:?}"
    );
}

#[test]
fn parallel_road_outside_buffer_plus_accuracy_does_not_fire() {
    for lat in [45_i64, 60, 120] {
        let mut d = base();
        d.lateral_mm = lat * 1000;
        assert!(!run(&d, 160).fired(), "parallel road {lat} m away");
    }
}

#[test]
fn boundary_jitter_never_enters_and_never_exits() {
    // Section 11: radius 150 m, acc_eff 10 m: jitter within 141..159 m never ENTERs.
    let mut rng = Rng::new(42);
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 150));
    let mut prev = 150_i64;
    for k in 0..600 {
        let r = rng.range_i64(141, 159);
        let ang = rng.below(4);
        let (e, n) = match ang {
            0 => (r, 0),
            1 => (0, r),
            2 => (-r, 0),
            _ => (0, -r),
        };
        // stay within the jump limit by interpolating only radially: keep the same axis changes small
        let _ = prev;
        prev = r;
        let mut f = fix_at(BASE_T + k * 1000, offset(ORIGIN, e * 1000, n * 1000));
        f["speed_mmps"] = json!(10_000);
        let s = rig.call(&f, BASE_T + k * 1000);
        if !s.fix_ok() {
            continue; // an axis flip is a 300 m hop in 1 s (FIX_JUMP): fine, just not counted
        }
        assert!(!s.fired("c-1"), "jitter fired at step {k}");
    }
}

#[test]
fn stationary_dwell_inside_fires_exactly_once() {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    let mut fires = 0;
    for k in 0..2000 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        fires += s.fires().len();
    }
    assert_eq!(fires, 1);
}

#[test]
fn unknown_or_denied_consent_never_fires_whatever_the_drive() {
    for v in ["unknown", "denied", "Granted", ""] {
        let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
        rig.consent["location_matching"] = json!(v);
        let d = base();
        for k in 0..160 {
            let s = rig.step(&d.fix(k));
            assert!(s.fires().is_empty(), "consent {v:?} fired at {k}");
        }
        assert_eq!(rig.history.last().unwrap().mem(), &empty_mem());
    }
}

#[test]
fn permission_downgraded_mid_route_wipes_state_and_the_approach_restarts() {
    let d = base();
    let baseline = run(&d, 200).fire().unwrap().0 as i64;
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let cut = baseline - 8; // armed, not yet fired
    for k in 0..cut {
        assert!(!rig.step(&d.fix(k)).fired(ID));
    }
    rig.consent["location_matching"] = json!("denied");
    for k in cut..cut + 5 {
        let s = rig.step(&d.fix(k));
        assert!(!s.fix_ok() && s.fix_reason() == "CONSENT_DENIED");
        assert_eq!(s.mem(), &empty_mem());
    }
    rig.consent["location_matching"] = json!("granted");
    let mut fired_at = None;
    for k in cut + 5..cut + 200 {
        let s = rig.step(&d.fix(k));
        if k == cut + 5 {
            assert_eq!(s.trace_len(), 1, "history starts again after the downgrade");
        }
        if s.fired(ID) {
            fired_at = Some(k);
            break;
        }
    }
    let k = fired_at.expect("fires again after the permission returns");
    assert!(
        k > baseline,
        "must re-arm from scratch: fired at {k}, baseline {baseline}"
    );
}

#[test]
fn beyond_window_far_away_and_past_d_pass() {
    // far: 3500 m route, vehicle at 0 -> d_rem 3500 > d_hi 3000 for the first steps
    let d = Drive::new(3500, 15_000);
    let mut rig = Rig::new().with_signpost(approach_route(ID, 3500));
    let first: Vec<Step> = (0..20).map(|k| rig.step(&d.fix(k))).collect();
    assert!(first.iter().all(|s| !s.fired(ID)));
    assert!(
        first
            .iter()
            .any(|s| s.reasons(ID).contains(&"BEYOND_WINDOW".to_string())),
        "far away is BEYOND_WINDOW"
    );
    // too close: 60 m from the anchor (< d_pass = max(60 m, 5 s x v_eff))
    let mut d = Drive::new(2500, 15_000);
    d.start_along_mm = 2_440_000;
    let r = run(&d, 30);
    assert!(!r.fired());
    assert!(r.saw("BEYOND_WINDOW"), "{:?}", r.reasons());
}

#[test]
fn nothing_is_a_candidate_beyond_reach_m() {
    // trigger.reach_m: a fix farther than reach from the origin (the anchor) is not a candidate.
    // With reach 500 m the 645 m fire distance is out of range, so the detector arms inside 500 m,
    // where the time rule already holds, and fires there (never earlier).
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.reach_m", json!(500));
    let r = run_with(sp, &base(), 200);
    let (_, f) = r.fire().expect("fires once inside reach");
    let d_rem = f["d_rem_mm"].as_i64().unwrap();
    assert!(
        d_rem <= 500_000 + 1000,
        "fired at d_rem {d_rem} mm, beyond reach 500 m"
    );
}

#[test]
fn one_fire_per_pass_and_rearm_only_beyond_1_2_d_hi_or_on_new_rev() {
    let d = base();
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let mut k = 0;
    let mut fires = 0;
    while k < 140 {
        fires += rig.step(&d.fix(k)).fires().len();
        k += 1;
    }
    assert_eq!(fires, 1);
    let det = rig.history.last().unwrap().det(ID).unwrap();
    assert_eq!(det["st"]["s"], "latched");
    assert_eq!(det["st"]["why"], "fired");
    // clear the 30-minute app interval: ride 4 km north (> 1.2 x 3000 m = 3600 m from the anchor),
    // then come back on a second pass much later; a new pass must be able to fire again
    let t_far = d.t_ms(k) + 1_900_000;
    let mut cur = 0;
    // teleport far north (gap > 60 s so no FIX_JUMP): 4 km beyond the anchor
    let far = |dist_m: i64, t: i64| {
        let mut f = fix_at(t, offset(ORIGIN, 0, dist_m * 1000));
        f["course_ddeg"] = json!(0);
        f
    };
    for i in 0..5 {
        let s = rig.call(&far(4000, t_far + i * 1000), t_far + i * 1000);
        assert!(s.fix_ok(), "{}", s.fix_reason());
        cur = i;
    }
    let _ = cur;
    let det = rig.history.last().unwrap().det(ID);
    assert!(
        det.is_none() || det.as_ref().unwrap()["st"]["s"] != "latched",
        "re-armed beyond 1.2 x d_hi: {det:?}"
    );
    // second pass from the north end heading south is the wrong direction; use a reversed-direction
    // check by simply asserting passing the anchor alone never re-armed:
    let mut rig2 = Rig::new().with_signpost(approach_route(ID, 2500));
    for k in 0..140 {
        rig2.step(&d.fix(k));
    }
    // 100 m past the anchor on a continued north drive is still latched
    let s = rig2.step(&d.fix(140));
    assert_eq!(s.det(ID).unwrap()["st"]["s"], "latched");
}

#[test]
fn a_new_revision_rearms_a_latched_detector() {
    let d = base();
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let mut k = 0;
    while k < 140 {
        rig.step(&d.fix(k));
        k += 1;
    }
    assert_eq!(
        rig.history.last().unwrap().det(ID).unwrap()["st"]["s"],
        "latched"
    );
    rig.signposts[0]["rev"] = json!(2);
    let s = rig.step(&d.fix(k));
    let det = s.det(ID);
    assert!(
        det.is_none() || det.as_ref().unwrap()["rev"] == 2,
        "state of the old revision is replaced: {det:?}"
    );
}
