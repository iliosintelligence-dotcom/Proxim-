//! Engine spec 7.2 (engine_spec 2): per-sender and per-app day caps, and the person's own
//! controls (dismiss rests a family 7 days, hide is permanent, report is permanent per sender).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::controls::apply;
use proxim_types::{Clock, Control, Id, Outcome, Policy, ReasonCode, Signpost};

const SPACING_MS: i64 = 1_950_000;

fn sp(id: &str, sender: &str) -> Signpost {
    let mut s = circle(id, 200);
    s.sender = sender.into();
    s
}

/// One visit: the fired id (if any) and every drop.
type Visit = (Option<String>, Vec<(String, ReasonCode)>);

/// `n` visits `SPACING_MS` apart from `start`.
fn visits(sim: &mut Sim, start: i64, n: usize) -> Vec<Visit> {
    (0..n)
        .map(|k| {
            let t0 = start + SPACING_MS * k as i64;
            let outs = sim.run(&enter_cycle(t0, k > 0 || !sim.state.mem.det.is_empty()));
            let fired = outs
                .iter()
                .flat_map(|o| o.decisions.iter())
                .find(|d| d.outcome == Outcome::Fire)
                .map(|d| d.signpost.as_str().to_string());
            (fired, outs.iter().flat_map(drop_reasons).collect())
        })
        .collect()
}

fn clock(now_ms: i64) -> Clock {
    Clock {
        now_ms,
        tz_offset_min: 0,
    }
}

#[test]
fn a_sender_gets_three_a_day_across_its_signposts() {
    let mut sim = Sim::new(vec![
        sp("a", "s1"),
        sp("b", "s1"),
        sp("c", "s1"),
        sp("d", "s1"),
    ]);
    let r = visits(&mut sim, T0, 4);
    let fired: Vec<_> = r.iter().filter_map(|x| x.0.clone()).collect();
    assert_eq!(fired.len(), 3, "{r:?}");
    assert!(r[3].0.is_none());
    assert!(
        r[3].1.iter().any(|(_, c)| *c == ReasonCode::CapSenderDay),
        "{:?}",
        r[3].1
    );
    assert_eq!(sim.state.caps.senders.len(), 1);
    assert_eq!(sim.state.caps.senders[0].day_n, 3);
    // Next day the sender's day resets.
    let r = visits(&mut sim, T0 + DAY, 1);
    assert!(r[0].0.is_some());
    assert_eq!(sim.state.caps.senders[0].day_n, 1);
}

#[test]
fn partners_may_only_lower_the_sender_cap() {
    for (asked, allowed) in [(Some(1), 1), (Some(9), 3), (None, 3)] {
        let mut sim = Sim::new(vec![
            sp("a", "s1"),
            sp("b", "s1"),
            sp("c", "s1"),
            sp("d", "s1"),
        ]);
        sim.policy = Policy {
            cap_sender_day: asked,
            ..explain()
        };
        let n = visits(&mut sim, T0, 5)
            .iter()
            .filter(|x| x.0.is_some())
            .count();
        assert_eq!(n, allowed, "asked {asked:?}");
    }
}

#[test]
fn an_app_gets_four_a_day_unless_the_partner_lowers_or_raises_it_to_eight() {
    let senders: Vec<Signpost> = (0..5)
        .flat_map(|k| {
            [
                sp(&format!("a{k}"), &format!("s{k}")),
                sp(&format!("b{k}"), &format!("s{k}")),
            ]
        })
        .collect();
    for (asked, allowed) in [(None, 4), (Some(2), 2), (Some(8), 8), (Some(50), 8)] {
        let mut sim = Sim::new(senders.clone());
        sim.policy = Policy {
            cap_app_day: asked,
            ..explain()
        };
        let r = visits(&mut sim, T0, 10);
        let n = r.iter().filter(|x| x.0.is_some()).count();
        assert_eq!(n, allowed, "asked {asked:?}");
        let last = &r[allowed].1;
        assert!(
            last.iter().any(|(_, c)| *c == ReasonCode::CapAppDay),
            "asked {asked:?}: {last:?}"
        );
    }
}

#[test]
fn dismiss_rests_the_family_for_seven_days() {
    let mut sim = Sim::new(vec![sp("a", "s1")]);
    assert_eq!(visits(&mut sim, T0, 1)[0].0.as_deref(), Some("a"));
    let at = T0 + SPACING_MS;
    sim.state = apply(
        &sim.state,
        &Control::Dismiss {
            id: Id::new("a").unwrap(),
            memory_only: false,
        },
        &clock(at),
    )
    .unwrap();
    let r = visits(&mut sim, at, 1);
    assert!(r[0].0.is_none());
    assert!(
        r[0].1
            .contains(&("a".into(), ReasonCode::CooldownDismissed)),
        "{:?}",
        r[0].1
    );
    // Six days later: still resting. Seven days and a bit: back, and the dismissal is pruned.
    assert!(visits(&mut sim, at + 6 * DAY, 1)[0].0.is_none());
    assert_eq!(
        visits(&mut sim, at + 7 * DAY + 60_000, 1)[0].0.as_deref(),
        Some("a")
    );
    assert!(sim.state.controls.dismissed.is_empty());
}

#[test]
fn hide_is_permanent_for_the_signpost_only() {
    let mut sim = Sim::new(vec![sp("a", "s1"), sp("b", "s1")]);
    sim.state = apply(
        &sim.state,
        &Control::Hide {
            id: Id::new("a").unwrap(),
            memory_only: false,
        },
        &clock(T0),
    )
    .unwrap();
    let r = visits(&mut sim, T0, 2);
    assert_eq!(r[0].0.as_deref(), Some("b"));
    assert!(r.iter().all(|x| x.0.as_deref() != Some("a")));
    assert!(r[0].1.contains(&("a".into(), ReasonCode::HiddenByUser)));
    assert!(visits(&mut sim, T0 + 400 * DAY, 1)[0].0.as_deref() != Some("a"));
}

#[test]
fn report_is_permanent_for_every_signpost_of_that_sender() {
    let mut sim = Sim::new(vec![sp("a", "s1"), sp("b", "s1"), sp("c", "s2")]);
    sim.state = apply(
        &sim.state,
        &Control::Report {
            sender: "s1".into(),
            memory_only: false,
        },
        &clock(T0),
    )
    .unwrap();
    let r = visits(&mut sim, T0, 3);
    let fired: Vec<_> = r.iter().filter_map(|x| x.0.clone()).collect();
    assert_eq!(fired, ["c", "c"], "only s2 ever fires: {r:?}");
    assert!(r[0].1.contains(&("a".into(), ReasonCode::SenderReported)));
    assert!(r[0].1.contains(&("b".into(), ReasonCode::SenderReported)));
}

#[test]
fn controls_are_sorted_unique_bounded_and_survive_a_restart() {
    let s = proxim_types::State::empty();
    let c = clock(T0);
    let mut st = s.clone();
    for id in ["m", "b", "m", "z"] {
        st = apply(
            &st,
            &Control::Hide {
                id: Id::new(id).unwrap(),
                memory_only: false,
            },
            &c,
        )
        .unwrap();
    }
    let hidden: Vec<_> = st.controls.hidden.iter().map(Id::as_str).collect();
    assert_eq!(hidden, ["b", "m", "z"]);
    assert!(
        apply(
            &st,
            &Control::Report {
                sender: String::new(),
                memory_only: false
            },
            &c
        )
        .is_err()
    );
    // Re-dismissing extends the rest rather than adding a second entry.
    st = apply(
        &st,
        &Control::Dismiss {
            id: Id::new("a").unwrap(),
            memory_only: false,
        },
        &c,
    )
    .unwrap();
    st = apply(
        &st,
        &Control::Dismiss {
            id: Id::new("a").unwrap(),
            memory_only: false,
        },
        &clock(T0 + 1000),
    )
    .unwrap();
    assert_eq!(st.controls.dismissed.len(), 1);
    assert_eq!(st.controls.dismissed[0].until_ms, T0 + 1000 + 7 * DAY);
    // A full dismissal list drops the entry that ends soonest.
    let mut full = s;
    for k in 0..proxim_types::defaults::MAX_DISMISSED {
        full = apply(
            &full,
            &Control::Dismiss {
                id: Id::new(&format!("d{k:04}")).unwrap(),
                memory_only: false,
            },
            &clock(T0 + k as i64),
        )
        .unwrap();
    }
    full = apply(
        &full,
        &Control::Dismiss {
            id: Id::new("new").unwrap(),
            memory_only: false,
        },
        &clock(T0 + 10_000),
    )
    .unwrap();
    assert_eq!(
        full.controls.dismissed.len(),
        proxim_types::defaults::MAX_DISMISSED
    );
    assert!(
        full.controls
            .dismissed
            .iter()
            .all(|d| d.id.as_str() != "d0000")
    );
    // A hand-made unsorted list is a structural error for decide.
    let mut bad = st.clone();
    bad.controls.hidden.reverse();
    let fix = still(T0, 0, 0);
    assert!(proxim_decide::decide(&bad, &fix, &[], &explain(), &granted(), &c, 0).is_err());
    // The state JSON keeps controls, so a process restart (which keeps the persisted state) keeps them.
    let text = proxim_types::canon::to_canonical_json(&st).unwrap();
    let back: proxim_types::State =
        proxim_types::strict::from_canonical_json(text.as_bytes()).unwrap();
    assert_eq!(back.controls, st.controls);
}

#[test]
fn an_engine_spec_1_state_reads_unchanged() {
    // No `senders`, no `controls`: both are absent, not empty arrays, in the canonical form.
    let text = proxim_types::canon::to_canonical_json(&proxim_types::State::empty()).unwrap();
    assert!(
        !text.contains("senders") && !text.contains("controls"),
        "{text}"
    );
}

#[test]
fn apply_control_json_round_trips_and_fails_closed() {
    use proxim_decide::json::apply_control_json;
    let st = proxim_types::canon::to_canonical_json(&proxim_types::State::empty()).unwrap();
    let clock = r#"{"now_ms":1791705600000,"tz_offset_min":0}"#;
    let out = apply_control_json(&st, r#"{"action":"hide","id":"sp-1"}"#, clock);
    assert!(
        out.starts_with(r#"{"state":"#) && out.contains(r#""hidden":["sp-1"]"#),
        "{out}"
    );
    for bad in [
        r#"{"action":"hide"}"#,
        r#"{"action":"mute","id":"x"}"#,
        r#"{"action":"report","sender":""}"#,
        "[]",
    ] {
        let out = apply_control_json(&st, bad, clock);
        assert!(out.starts_with(r#"{"error":"#), "{bad}: {out}");
    }
}
