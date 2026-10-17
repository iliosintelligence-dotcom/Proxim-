//! One-slot selection (spec 7): `(score desc, t_hat asc with absent last, id asc)`, the loser gets
//! LOST_SLOT and reverts to its pre-event detector state, the winner is committed.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_types::{DetState, Policy, ReasonCode, Signpost};

fn rank(mut sp: Signpost, r: u32) -> Signpost {
    sp.rank_milli = Some(r);
    sp
}

/// The `state` of a detector entry by id.
fn st_of(sim: &Sim, id: &str) -> Option<DetState> {
    sim.state
        .mem
        .det
        .iter()
        .find(|e| e.id.as_str() == id)
        .map(|e| e.st)
}

#[test]
fn higher_score_wins_and_the_loser_reverts_to_its_pre_event_state() {
    let mut sim = Sim::new(vec![
        rank(circle("a", 200), 1000),
        rank(circle("b", 200), 2000),
    ]);
    let fixes = visit(T0, 0, 0, 6, 0);
    let outs = sim.run(&fixes[..3]);
    assert!(fire_indices(&outs).is_empty());
    assert!(matches!(
        st_of(&sim, "a"),
        Some(DetState::Cand { n: 3, .. })
    ));
    // The 4th fix completes ENTER for both: b (2000 x 500 / 1000 = 1000) beats a (500).
    let o = sim.step(&fixes[3]);
    let f = fires(&o);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].signpost.as_str(), "b");
    assert!(has_drop(&o, "a", ReasonCode::LostSlot));
    // Winner: detector advanced (in). Loser: reverted to the state before the event (cand n = 3).
    assert!(matches!(st_of(&sim, "b"), Some(DetState::In { .. })));
    assert!(
        matches!(st_of(&sim, "a"), Some(DetState::Cand { n: 3, first_t, last_t })
        if first_t == T0 && last_t == T0 + 8000)
    );
    // Only the winner is committed.
    let ids: Vec<_> = sim
        .state
        .caps
        .signposts
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(ids, ["b"]);
    assert_eq!(sim.state.caps.last_fire_ms, Some(T0 + 12_000));
    // The next fix: the loser produces its event again, but the app interval (F24) now blocks it
    // and consumes it. It cannot sneak in a second card within 30 minutes.
    let o = sim.step(&fixes[4]);
    assert!(fires(&o).is_empty());
    assert!(has_drop(&o, "a", ReasonCode::CapAppInterval));
    assert!(matches!(st_of(&sim, "a"), Some(DetState::In { .. })));
}

#[test]
fn equal_scores_fall_back_to_the_id() {
    for ids in [["a", "b"], ["b", "a"]] {
        let mut sim = Sim::new(vec![circle(ids[0], 200), circle(ids[1], 200)]);
        let outs = sim.run(&visit(T0, 0, 0, 4, 0));
        let o = &outs[3];
        assert_eq!(
            fires(o)[0].signpost.as_str(),
            "a",
            "lowest id wins a full tie"
        );
        assert!(has_drop(o, "b", ReasonCode::LostSlot));
    }
}

#[test]
fn a_smaller_t_hat_beats_an_absent_one_at_equal_score() {
    // APPROACH (t_hat present, conf 800) and ENTER (t_hat absent, conf 500) fire on the same fix.
    // The route's t_hat is 41.332 s against its 40 s lead, so its timing fit is 984 (7.5) and its
    // score 1000 x 800 x 984 / 1e6 = 787; the circle's is 1574 x 500 / 1000 = 787: a tie. The
    // circle's id sorts first, so only the t_hat rule can make the route win.
    let mut circle_sp = rank(circle_at("a-circle", 0, 2_020_000, 200), 1574);
    circle_sp.rank_milli = Some(1574);
    let route_sp = rank(route("z-route", 2500, 30), 1000);
    let mut sim = Sim::new(vec![circle_sp, route_sp]);
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 40, 0));
    let idx = fire_indices(&outs);
    assert_eq!(idx, vec![32], "{idx:?}");
    let o = &outs[32];
    assert_eq!(fires(o)[0].signpost.as_str(), "z-route");
    assert!(has_drop(o, "a-circle", ReasonCode::LostSlot));
    let lost = o
        .decisions
        .iter()
        .find(|d| d.signpost.as_str() == "a-circle")
        .unwrap();
    assert_eq!((lost.path_tier, lost.path_conf_milli), (0, 500));
    // Swap the ranks so the circle scores higher: now it wins on score, whatever the t_hat.
    let mut sim = Sim::new(vec![
        rank(circle_at("a-circle", 0, 2_020_000, 200), 2000),
        rank(route("z-route", 2500, 30), 1000),
    ]);
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 40, 0));
    assert_eq!(fires(&outs[32])[0].signpost.as_str(), "a-circle");
}

#[test]
fn zero_slots_means_nothing_ever_fires_and_nothing_is_committed() {
    let mut sim = Sim::new(vec![circle("a", 200)]);
    sim.policy = Policy {
        slots: Some(0),
        ..explain()
    };
    let outs = sim.run(&visit(T0, 0, 0, 6, 0));
    assert!(fire_indices(&outs).is_empty());
    assert!(has_drop(&outs[3], "a", ReasonCode::LostSlot));
    assert!(sim.state.caps.signposts.is_empty());
    assert_eq!(sim.state.caps.last_fire_ms, None);
}

#[test]
fn slots_are_capped_at_three() {
    // engine_spec 2 (7.5): a partner may ask for up to 3 slots; the default stays 1.
    let four = || {
        vec![
            circle("a", 200),
            circle("b", 200),
            circle("c", 200),
            circle("d", 200),
        ]
    };
    for (asked, fired) in [(None, 1), (Some(2), 2), (Some(3), 3), (Some(9), 3)] {
        let mut sim = Sim::new(four());
        sim.policy = Policy {
            slots: asked,
            cap_sender_day: None,
            ..explain()
        };
        // Four senders, so the sender cap does not bind.
        for (k, sp) in sim.signposts.iter_mut().enumerate() {
            sp.sender = format!("s{k}");
        }
        let outs = sim.run(&visit(T0, 0, 0, 4, 0));
        assert_eq!(fires(&outs[3]).len(), fired, "asked {asked:?}");
        let lost = outs[3]
            .decisions
            .iter()
            .filter(|d| d.reason == ReasonCode::LostSlot)
            .count();
        assert_eq!(lost, 4 - fired, "asked {asked:?}");
    }
}

#[test]
fn decisions_are_sorted_fire_first_then_by_id() {
    let mut a = circle("a", 200);
    a.valid_until_ms = 1; // expired at T0 -> drop
    let mut sim = Sim::new(vec![
        a,
        circle("m", 200),
        circle("z", 200),
        rank(circle("b", 200), 3000),
    ]);
    let outs = sim.run(&visit(T0, 0, 0, 4, 0));
    let ids: Vec<_> = outs[3]
        .decisions
        .iter()
        .map(|d| (d.signpost.as_str(), d.reason))
        .collect();
    assert_eq!(
        ids,
        vec![
            ("b", ReasonCode::Ok),
            ("a", ReasonCode::Expired),
            ("m", ReasonCode::LostSlot),
            ("z", ReasonCode::LostSlot)
        ]
    );
}

#[test]
fn the_score_uses_integer_division_and_a_zero_rank_still_ties_by_id() {
    // rank 0 scores 0 but is still an eligible candidate when it is the only one.
    let mut sim = Sim::new(vec![rank(circle("a", 200), 0)]);
    let outs = sim.run(&visit(T0, 0, 0, 4, 0));
    assert_eq!(fire_indices(&outs), vec![3]);
    // 1999 x 500 / 1000 = 999 < 1000: one rank point short of the 2000 ranked sibling's half.
    let mut sim = Sim::new(vec![
        rank(circle("a", 200), 1999),
        rank(circle("b", 200), 2000),
    ]);
    let outs = sim.run(&visit(T0, 0, 0, 4, 0));
    assert_eq!(fires(&outs[3])[0].signpost.as_str(), "b");
    // u32::MAX rank does not overflow.
    let mut sim = Sim::new(vec![
        rank(circle("a", 200), u32::MAX),
        rank(circle("b", 200), 1),
    ]);
    let outs = sim.run(&visit(T0, 0, 0, 4, 0));
    assert_eq!(fires(&outs[3])[0].signpost.as_str(), "a");
}
