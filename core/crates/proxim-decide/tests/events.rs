//! E5 (reserved events fail closed) and the two built events end to end through `decide`.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_types::{DetEntry, DetState, EventKind, ReasonCode, Shape};

const RESERVED: [EventKind; 5] = [
    EventKind::Exit,
    EventKind::Cross,
    EventKind::Dwell,
    EventKind::Pass,
    EventKind::Sequence,
];

#[test]
fn unsupported_event_is_ignored_fail_closed() {
    for ev in RESERVED {
        // Both a circle-shaped and a route-shaped trigger, the first even geometrically valid.
        for base in [circle("u-1", 200), route("u-1", 2500, 30)] {
            let mut sp = base;
            sp.event = ev;
            let mut sim = Sim::new(vec![sp, circle("ok-1", 200)]);
            // Stand in the circle long enough for ENTER, and drive the route.
            let mut fixes = visit(T0, 0, 0, 6, 0);
            fixes.extend(drive_north(T0 + 600_000, 1_400_000, 15_000, 50, 0));
            let outs = sim.run(&fixes);
            for (i, o) in outs.iter().enumerate() {
                assert!(
                    !o.decisions.iter().any(|d| d.signpost.as_str() == "u-1"
                        && d.outcome == proxim_types::Outcome::Fire),
                    "{ev:?} step {i}"
                );
                assert!(
                    has_drop(o, "u-1", ReasonCode::UnsupportedEvent),
                    "{ev:?} step {i}: {:?}",
                    o.decisions
                );
                // The reserved signpost never gets a detector entry, so it can neither fire nor
                // hold memory.
                assert!(
                    !sim.state.mem.det.iter().any(|e| e.id.as_str() == "u-1"),
                    "{ev:?}"
                );
            }
            // A well-formed neighbour is unaffected.
            assert!(
                outs.iter()
                    .any(|o| fires(o).iter().any(|d| d.signpost.as_str() == "ok-1")),
                "{ev:?}: the other signpost still fires"
            );
        }
    }
}

#[test]
fn unsupported_event_removes_a_stale_detector_entry_and_is_silent_without_explain() {
    // A state that carries a latched entry for the id (e.g. written by an older build).
    let mut sim = Sim::new(vec![{
        let mut sp = circle("u-1", 200);
        sp.event = EventKind::Dwell;
        sp
    }]);
    sim.state.mem.det.push(DetEntry {
        id: proxim_types::Id::new("u-1").unwrap(),
        rev: 1,
        st: DetState::Cool { until_ms: i64::MAX },
    });
    sim.policy.explain = Some(false);
    let o = sim.step(&still(T0, 0, 0));
    assert!(
        o.decisions.is_empty() && o.notes.is_none(),
        "silent without explain"
    );
    assert!(sim.state.mem.det.is_empty());
}

#[test]
fn unsupported_beats_a_shape_mismatch_and_other_defects() {
    // F11 is checked before F12: a reserved event with a mismatched shape and a bad window
    // reports UNSUPPORTED_EVENT, not TRIGGER_INVALID.
    let mut sp = circle("u-1", 10); // radius below the floor too
    sp.event = EventKind::Cross;
    sp.valid_until_ms = sp.valid_from_ms;
    let mut sim = Sim::new(vec![sp]);
    let o = sim.step(&still(T0, 0, 0));
    assert!(has_drop(&o, "u-1", ReasonCode::UnsupportedEvent));
}

#[test]
fn enter_fires_once_per_visit_through_decide() {
    let mut sim = Sim::new(vec![circle("c-1", 200)]);
    let outs = sim.run(&visit(T0, 0, 0, 8, 8));
    // Still fixes 4 s apart: n >= 3 and 10 s span (slow) -> the 4th fix (12 s).
    assert_eq!(fire_indices(&outs), vec![3]);
    let d = fires(&outs[3])[0].clone();
    assert_eq!(
        (d.event, d.path_tier, d.path_conf_milli),
        (EventKind::Enter, 0, 500)
    );
    assert_eq!(d.copy, proxim_types::Copy::NearYou);
    assert_eq!((d.d_rem_mm, d.t_hat_ms), (None, None));
    assert_eq!(d.t_ms, T0 + 12_000);
    assert_eq!(outs[3].path.tier, 0);
    assert_eq!(
        outs[3].path.conf_milli, 500,
        "the path block reports the winning event"
    );
}

#[test]
fn approach_fires_once_per_pass_through_decide() {
    let mut sim = Sim::new(vec![route("r-1", 2500, 30)]);
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 120, 0));
    assert_eq!(fire_indices(&outs), vec![32], "one fire per pass");
    let d = fires(&outs[32])[0].clone();
    assert_eq!(
        (d.event, d.path_tier, d.copy),
        (EventKind::Approach, 1, proxim_types::Copy::Ahead)
    );
    // D_fire = 15 m/s x 43 s = 645 m (spec 11); the fix at 32 s is the first one inside it.
    let d_rem = d.d_rem_mm.unwrap();
    assert!((600_000..=645_000).contains(&d_rem), "d_rem {d_rem}");
    let t_hat = d.t_hat_ms.unwrap();
    assert!((40_000..=43_000).contains(&t_hat), "t_hat {t_hat}");
    assert_eq!(d.path_conf_milli, 800);
}

#[test]
fn reserved_trigger_shapes_do_not_unlock_events() {
    // A "polygon" ENTER with a route shape is a trigger defect, not a fire.
    let mut sp = circle("bad-1", 200);
    sp.trigger.shape = route("x", 100, 30).trigger.shape;
    let mut sim = Sim::new(vec![sp]);
    let o = sim.step(&still(T0, 0, 0));
    assert!(has_drop(&o, "bad-1", ReasonCode::TriggerInvalid));
    assert!(matches!(
        circle("c", 200).trigger.shape,
        Shape::Circle { .. }
    ));
}
