//! Spec 7 selection and commit: survivors ordered by (score desc, t_hat asc with absent last, id asc);
//! the first `slots` fire, the rest get F25 LOST_SLOT and revert to their pre-event detector state.
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

use proxim_conformance::*;
use serde_json::json;

fn two(a: (&str, i64), b: (&str, i64)) -> Rig {
    let mut sa = enter_circle(a.0, 200);
    sa["rank_milli"] = json!(a.1);
    let mut sb = enter_circle(b.0, 200);
    sb["rank_milli"] = json!(b.1);
    let mut rig = Rig::new();
    rig.signposts = vec![sa, sb];
    rig
}

/// Run until some signpost fires or loses; returns that step.
fn first_event(rig: &mut Rig) -> Step {
    for k in 0..8 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        if !s.fires().is_empty() {
            return s;
        }
    }
    panic!("nothing fired");
}

#[test]
fn equal_scores_break_ties_by_ascending_id() {
    let mut rig = two(("a-1", 1000), ("b-1", 1000));
    let s = first_event(&mut rig);
    assert_eq!(s.fires().len(), 1, "one slot");
    assert!(s.fired("a-1"));
    assert_eq!(s.drops("b-1"), vec!["LOST_SLOT".to_string()], "{}", s.text);
    // input order must not matter
    let mut rig = two(("b-1", 1000), ("a-1", 1000));
    rig.signposts.reverse();
    let s = first_event(&mut rig);
    assert!(s.fired("a-1"));
}

#[test]
fn higher_rank_beats_lower_id() {
    let mut rig = two(("a-1", 500), ("b-1", 1000));
    let s = first_event(&mut rig);
    assert!(s.fired("b-1"), "score = rank x conf / 1000: {}", s.text);
    assert_eq!(s.drops("a-1"), vec!["LOST_SLOT".to_string()]);
    // score is the integer rank x conf / 1000 (u64): at conf 500, rank 1001 floors to 500 = a tie
    // (lower id wins) while rank 1002 gives 501 and wins.
    let mut rig = two(("a-1", 1000), ("b-1", 1001));
    assert!(first_event(&mut rig).fired("a-1"), "500 vs 500 is a tie");
    let mut rig = two(("a-1", 1000), ("b-1", 1002));
    assert!(first_event(&mut rig).fired("b-1"), "501 beats 500");
}

#[test]
fn decisions_are_sorted_fire_first_then_by_id() {
    let mut rig = Rig::new();
    let mut sps = vec![];
    for (id, rank) in [("d-1", 1000), ("a-1", 900), ("c-1", 800), ("b-1", 700)] {
        let mut sp = enter_circle(id, 200);
        sp["rank_milli"] = json!(rank);
        sps.push(sp);
    }
    rig.signposts = sps;
    let s = first_event(&mut rig);
    let order: Vec<String> = s
        .decisions()
        .iter()
        .map(|d| d["signpost"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        order,
        ["d-1", "a-1", "b-1", "c-1"],
        "fire first, then drops by id: {}",
        s.text
    );
    assert_eq!(s.decisions()[0]["outcome"], "fire");
}

#[test]
fn slots_cannot_be_raised_by_a_partner() {
    for slots in [2, 3, 100] {
        let mut rig = two(("a-1", 1000), ("b-1", 1000));
        rig.policy["slots"] = json!(slots);
        let s = first_event(&mut rig);
        assert_eq!(s.fires().len(), 1, "slots={slots} clamps to 1");
    }
}

#[test]
fn the_loser_reverts_to_its_pre_event_state_and_is_then_held_by_the_app_interval() {
    let mut rig = two(("a-1", 1000), ("b-1", 1000));
    let s = first_event(&mut rig);
    assert!(s.fired("a-1"));
    // loser: pre-event state is still a candidate (3 counted fixes), the winner is `in`
    let det_b = s.det("b-1").unwrap();
    assert_eq!(
        det_b["st"]["s"], "cand",
        "loser must revert to its pre-event state: {det_b}"
    );
    assert_eq!(s.det("a-1").unwrap()["st"]["s"], "in");
    // the winner's caps are written, the loser's are not
    assert!(s.cap_entry("a-1").is_some());
    assert!(s.cap_entry("b-1").is_none());
    // next fix: b-1's event exists again but the app interval (just fired) holds it back
    let s2 = rig.step(&fix_at(BASE_T + 4000, ORIGIN));
    assert!(s2.fires().is_empty());
    assert_eq!(
        s2.drops("b-1"),
        vec!["CAP_APP_INTERVAL".to_string()],
        "{}",
        s2.text
    );
}

#[test]
fn smaller_time_to_anchor_wins_before_id_order() {
    // Two approach signposts on the same route; z-1's anchor is 5 m nearer the vehicle, so its t_hat is
    // smaller. Ids would pick a-1; (score, t_hat) must pick z-1 when both events occur at one fix.
    let found = (0..15).find_map(|off_m| {
        let mut a = approach_route("a-1", 2500);
        let mut z = approach_route("z-1", 2500);
        set(&mut z, "trigger.shape.anchor_along_m", json!(2495));
        let _ = &mut a;
        let mut rig = Rig::new();
        rig.signposts = vec![a, z];
        let mut d = Drive::new(2500, 15_000);
        d.start_along_mm = off_m * 1000;
        for k in 0..200 {
            let s = rig.step(&d.fix(k));
            let lost = s.drops("a-1").contains(&"LOST_SLOT".to_string())
                || s.drops("z-1").contains(&"LOST_SLOT".to_string());
            if !s.fires().is_empty() || lost {
                return if lost { Some(s) } else { None };
            }
        }
        None
    });
    let s = found.expect("some start offset makes both events land on the same fix");
    let (a_t, z_t) = (
        s.decisions()
            .iter()
            .find(|d| d["signpost"] == "z-1")
            .cloned(),
        s.decisions()
            .iter()
            .find(|d| d["signpost"] == "a-1")
            .cloned(),
    );
    assert!(a_t.is_some() && z_t.is_some());
    assert!(
        s.fired("z-1"),
        "smaller t_hat must win over lower id: {}",
        s.text
    );
    assert_eq!(s.drops("a-1"), vec!["LOST_SLOT".to_string()]);
}
