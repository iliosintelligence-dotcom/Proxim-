//! Per-signpost caps (2 per local day, 5 per local week), the 1800 s app interval, local calendar
//! buckets, and the persisted high-water mark (a clock set backwards cannot reopen caps).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_decide::decide;
use proxim_types::canon::to_canonical_json;
use proxim_types::{Caps, Clock, DetEntry, DetState, Id, Policy, ReasonCode, SignpostCaps, State};

/// Monday 2026-10-12 00:00:00 UTC (the previous day, Sunday 2026-10-11, is week index - 1).
const MON: i64 = 1_791_763_200_000;
const SUN: i64 = MON - DAY;

/// Spacing between visits: exit at +91 s, cool-down 30 min after that, so 1950 s is enough.
const SPACING_MS: i64 = 1_950_000;

/// One visit: see [`enter_cycle`].
fn visit_at(t0: i64, release: bool) -> Vec<proxim_types::Fix> {
    enter_cycle(t0, release)
}

/// Drive visits `SPACING_MS` apart, returning for each visit whether it fired and the drop
/// reasons seen.
fn visits(sim: &mut Sim, start: i64, n: usize) -> Vec<(bool, Vec<ReasonCode>)> {
    let mut res = Vec::new();
    for k in 0..n {
        let t0 = start + SPACING_MS * k as i64;
        let outs = sim.run(&visit_at(t0, k > 0 || !sim.state.mem.det.is_empty()));
        let fired = outs.iter().any(|o| !fires(o).is_empty());
        let reasons = outs
            .iter()
            .flat_map(|o| o.decisions.iter())
            .filter(|d| d.outcome == proxim_types::Outcome::Drop)
            .map(|d| d.reason)
            .collect();
        res.push((fired, reasons));
    }
    res
}

#[test]
fn two_per_day_then_cap_signpost_day_then_the_next_day_reopens() {
    let mut sim = Sim::new(vec![circle("c", 200)]);
    let start = T0; // 08:00 UTC
    let r = visits(&mut sim, start, 3);
    assert!(r[0].0 && r[1].0, "two fires");
    assert!(!r[2].0);
    assert!(r[2].1.contains(&ReasonCode::CapSignpostDay), "{:?}", r[2].1);
    let e = &sim.state.caps.signposts[0];
    assert_eq!(
        (e.day_n, e.week_n),
        (2, 2),
        "the dropped event spends nothing"
    );
    // Next local day (UTC): the day bucket resets, the week bucket carries over (Sunday -> Monday
    // is a new week as well, so use Tuesday vs Monday below). Here: tomorrow 08:00.
    let r = visits(&mut sim, start + DAY, 1);
    assert!(r[0].0);
    let e = &sim.state.caps.signposts[0];
    assert_eq!(e.day_n, 1);
}

#[test]
fn week_cap_five_then_next_monday_reopens() {
    // Tue, Wed, Thu: 2 + 2 + 1 fires, then the sixth fire of the week is capped even on a fresh
    // day; the following Monday starts a new week.
    let tue = MON + DAY + 8 * 3_600_000;
    let mut sim = Sim::new(vec![circle("c", 200)]);
    let mut fired = Vec::new();
    for (d, n) in [(0, 2), (1, 2), (2, 1)] {
        let r = visits(&mut sim, tue + d * DAY, n);
        fired.extend(r.iter().map(|x| x.0));
    }
    assert_eq!(fired, vec![true; 5]);
    let e = &sim.state.caps.signposts[0];
    assert_eq!((e.day_n, e.week_n), (1, 5));
    // Friday: day count is fresh, week count is 5.
    let r = visits(&mut sim, tue + 3 * DAY, 1);
    assert!(!r[0].0);
    assert!(
        r[0].1.contains(&ReasonCode::CapSignpostWeek),
        "{:?}",
        r[0].1
    );
    // The next Monday (6 days after Tuesday): new week.
    let r = visits(&mut sim, tue + 6 * DAY, 1);
    assert!(r[0].0);
    let e = &sim.state.caps.signposts[0];
    assert_eq!((e.day_n, e.week_n), (1, 1));
}

#[test]
fn the_day_boundary_follows_the_local_timezone() {
    // 23:30 UTC is already the next day at UTC+2. Two fires before 22:00 UTC (= midnight local
    // at +2) and a third after it: a new local day, so it fires; at UTC it would be capped.
    let base = MON + 20 * 3_600_000; // 20:00 UTC Monday
    for (tz, third_fires) in [(0_i16, false), (120, true)] {
        let mut sim = Sim::new(vec![circle("c", 200)]);
        sim.tz = tz;
        let r = visits(&mut sim, base, 3);
        // visits at 20:00:00, 20:30:50, 21:01:40 UTC -> all before 22:00 UTC: third is capped at
        // UTC+0 and at UTC+2 (same local day, which began at 22:00 UTC the day before).
        assert!(r[0].0 && r[1].0 && !r[2].0, "tz {tz}: {:?}", r);
        // A fourth visit at 22:30 UTC: local day 2 for UTC+2, still the same day for UTC.
        let r = visits(&mut sim, base + 2 * 3_600_000 + 1_800_000, 1);
        assert_eq!(r[0].0, third_fires, "tz {tz}");
    }
}

#[test]
fn week_boundary_is_monday_in_local_time() {
    // Sunday 23:00 UTC is Monday 01:00 at UTC+2, i.e. the next week there.
    let sun_late = SUN + 23 * 3_600_000;
    for (tz, expect_week_delta) in [(0_i16, 0_i64), (120, 1)] {
        let mut sim = Sim::new(vec![circle("c", 200)]);
        sim.tz = tz;
        visits(&mut sim, sun_late - 3 * DAY, 1);
        let w0 = sim.state.caps.signposts[0].week_idx;
        visits(&mut sim, sun_late, 1);
        let w1 = sim.state.caps.signposts[0].week_idx;
        assert_eq!(w1 - w0, expect_week_delta, "tz {tz}");
        // A new local week restarts the count; the same week carries it.
        let want_week_n = if expect_week_delta == 1 { 1 } else { 2 };
        assert_eq!(sim.state.caps.signposts[0].week_n, want_week_n, "tz {tz}");
    }
}

#[test]
fn app_interval_boundary_is_exactly_1800_seconds() {
    // One ENTER event at E; the persisted last fire is 1_799_999 ms before (blocked) or exactly
    // 1_800_000 ms before (allowed). The detector is preset to one fix short of the event.
    let e = T0 + 12_000;
    let run = |last_fire: i64, interval: Option<u32>| {
        let mut state = State::empty();
        state.caps.hwm_ms = e - 100;
        state.caps.last_fire_ms = Some(last_fire);
        state.mem.last_t_ms = Some(e - 4000);
        state.mem.det.push(DetEntry {
            id: Id::new("c").unwrap(),
            rev: 1,
            st: DetState::Cand {
                n: 3,
                first_t: e - 12_000,
                last_t: e - 4000,
            },
        });
        let policy = Policy {
            app_min_interval_s: interval,
            ..explain()
        };
        let (o, _) = decide(
            &state,
            &still(e, 0, 0),
            &[circle("c", 200)],
            &policy,
            &granted(),
            &clock(e),
            0,
        )
        .unwrap();
        o
    };
    let blocked = run(e - 1_799_999, None);
    assert!(
        has_drop(&blocked, "c", ReasonCode::CapAppInterval),
        "{:?}",
        blocked.decisions
    );
    assert!(fires(&run(e - 1_800_000, None)).len() == 1);
    // A partner may lengthen the interval, never shorten it.
    assert!(has_drop(
        &run(e - 1_800_000, Some(3600)),
        "c",
        ReasonCode::CapAppInterval
    ));
    assert!(fires(&run(e - 1_800_000, Some(1))).len() == 1);
    assert!(has_drop(
        &run(e - 1_799_999, Some(1)),
        "c",
        ReasonCode::CapAppInterval
    ));
    // A last fire in the future of `now` (corrupt or tampered) blocks: fail closed.
    assert!(has_drop(
        &run(e + 5000, None),
        "c",
        ReasonCode::CapAppInterval
    ));
}

#[test]
fn app_interval_is_shared_across_signposts() {
    let mut sim = Sim::new(vec![circle("a", 200), circle_at("b", 3_000_000, 0, 200)]);
    let outs = sim.run(&visit(T0, 0, 0, 5, 0));
    assert_eq!(fire_indices(&outs), vec![3]);
    // 100 s later b's ENTER event is inside the 30 min window.
    let outs = sim.run(&visit(T0 + 100_000, 3_000_000, 0, 5, 0));
    assert!(fire_indices(&outs).is_empty());
    assert!(
        outs.iter()
            .any(|o| has_drop(o, "b", ReasonCode::CapAppInterval))
    );
}

#[test]
fn caps_policy_values_only_narrow() {
    let narrow = |day: Option<u32>, week: Option<u32>| {
        let mut sim = Sim::new(vec![circle("c", 200)]);
        sim.policy = Policy {
            cap_signpost_day: day,
            cap_signpost_week: week,
            ..explain()
        };
        visits(&mut sim, T0, 4).iter().filter(|r| r.0).count()
    };
    assert_eq!(narrow(None, None), 2);
    assert_eq!(narrow(Some(1), None), 1);
    assert_eq!(narrow(Some(0), None), 0);
    assert_eq!(
        narrow(Some(50), Some(50)),
        2,
        "a partner cannot raise the day cap"
    );
    // Week cap lowered to 1: only one fire even though the day cap allows two.
    assert_eq!(narrow(None, Some(1)), 1);
}

// ---- the high-water mark ----

/// A state where the signpost already fired twice on local day `day` (UTC) and the clock has
/// run until `hwm`.
fn capped_state(hwm: i64, day_idx: i64, week_idx: i64) -> State {
    State {
        v: 1,
        caps: Caps {
            hwm_ms: hwm,
            last_fire_ms: Some(hwm - 3_600_000),
            signposts: vec![SignpostCaps {
                id: Id::new("c").unwrap(),
                day_idx,
                day_n: 2,
                week_idx,
                week_n: 2,
                tz_offset_min: None,
            }],
            senders: vec![],
        },
        mem: Default::default(),
        controls: Default::default(),
    }
}

/// The detector one fix short of an ENTER event at `e`.
fn primed(mut st: State, e: i64) -> State {
    st.mem.last_t_ms = Some(e - 4000);
    st.mem.det.push(DetEntry {
        id: Id::new("c").unwrap(),
        rev: 1,
        st: DetState::Cand {
            n: 3,
            first_t: e - 12_000,
            last_t: e - 4000,
        },
    });
    st
}

#[test]
fn a_clock_set_backwards_cannot_reopen_the_day_cap() {
    // The previous call ran at 00:00:02 UTC on day D+1 and left two fires on D+1. The system
    // clock is now set 4 s back, to 23:59:58 on day D, while the GPS fix is stamped 00:00:03.
    // Without the high-water mark `now` would be day D, where the entry (day D+1) does not count:
    // a third fire. With it, `now` stays on D+1 and the cap holds.
    let d1 = MON; // 00:00:00 UTC Monday = start of local day D+1
    let hwm = d1 + 2000;
    let (day_idx, week_idx) = (d1 / DAY, (d1 / DAY + 3) / 7);
    let e = d1 + 3000;
    let st = primed(capped_state(hwm, day_idx, week_idx), e);
    let back = Clock {
        now_ms: d1 - 2000,
        tz_offset_min: 0,
    };
    let (o, next) = decide(
        &st,
        &still(e, 0, 0),
        &[circle("c", 200)],
        &explain(),
        &granted(),
        &back,
        0,
    )
    .unwrap();
    assert!(fires(&o).is_empty());
    assert!(
        has_drop(&o, "c", ReasonCode::CapSignpostDay),
        "{:?}",
        o.decisions
    );
    assert_eq!(o.t_ms, hwm, "now = max(clock, hwm)");
    assert_eq!(next.caps.hwm_ms, hwm);
    // Control: the same state with the clock honest fires nothing either (cap reached) ...
    let honest = Clock {
        now_ms: hwm,
        tz_offset_min: 0,
    };
    let (o2, _) = decide(
        &st,
        &still(e, 0, 0),
        &[circle("c", 200)],
        &explain(),
        &granted(),
        &honest,
        0,
    )
    .unwrap();
    assert!(has_drop(&o2, "c", ReasonCode::CapSignpostDay));
    // ... and the same clock-back on a state whose entry is from the PREVIOUS day (cap not yet
    // reached today) does fire: the hwm is not a blanket block.
    let st2 = primed(capped_state(hwm, day_idx - 1, week_idx - 1), e);
    let (o3, _) = decide(
        &st2,
        &still(e, 0, 0),
        &[circle("c", 200)],
        &explain(),
        &granted(),
        &back,
        0,
    )
    .unwrap();
    assert_eq!(fires(&o3).len(), 1);
}

#[test]
fn a_clock_set_backwards_cannot_reopen_the_week_cap() {
    // Week cap reached on Monday's week; the clock is set back into the previous week (Sunday).
    let hwm = MON + 3_600_000;
    let (day_idx, week_idx) = (hwm / DAY, (hwm / DAY + 3) / 7);
    let mut st = capped_state(hwm, day_idx - 1, week_idx); // fired on earlier days of this week
    st.caps.signposts[0].week_n = 5;
    st.caps.signposts[0].day_n = 0;
    let e = hwm + 1000;
    let st = primed(st, e);
    // Clock 2 h back = Sunday 23:00; the fix is stamped honestly.
    let back = Clock {
        now_ms: hwm - 2 * 3_600_000,
        tz_offset_min: 0,
    };
    // F06: the fix is more than 5 s ahead of the (unclamped) clock, but `now` is clamped to hwm.
    let (o, _) = decide(
        &st,
        &still(e, 0, 0),
        &[circle("c", 200)],
        &explain(),
        &granted(),
        &back,
        0,
    )
    .unwrap();
    assert!(
        has_drop(&o, "c", ReasonCode::CapSignpostWeek),
        "{:?}",
        o.decisions
    );
}

#[test]
fn the_high_water_mark_ratchets_and_is_stored_on_every_accepted_call() {
    let mut sim = Sim::new(vec![]);
    sim.step_at(&still(T0, 0, 0), T0);
    assert_eq!(sim.state.caps.hwm_ms, T0);
    let o = sim.step_at(&still(T0 + 1000, 0, 0), T0 - 3_600_000);
    assert_eq!(o.t_ms, T0, "output time is the clamped now");
    assert_eq!(sim.state.caps.hwm_ms, T0);
    sim.step_at(&still(T0 + 2000, 0, 0), T0 + 2000);
    assert_eq!(sim.state.caps.hwm_ms, T0 + 2000);
    // Rejected fixes do not move it.
    let before = sim.state.caps.hwm_ms;
    sim.step_at(&still(T0 + 2000, 0, 0), T0 + 99_000); // out of order
    assert_eq!(sim.state.caps.hwm_ms, before);
}

#[test]
fn caps_older_than_this_week_are_pruned_each_call() {
    let hwm = MON + 3_600_000;
    let week_idx = (hwm / DAY + 3) / 7;
    let mut st = State::empty();
    st.caps.hwm_ms = hwm;
    st.caps.signposts = vec![
        SignpostCaps {
            id: Id::new("a").unwrap(),
            day_idx: hwm / DAY - 10,
            day_n: 2,
            week_idx: week_idx - 2,
            week_n: 5,
            tz_offset_min: None,
        },
        SignpostCaps {
            id: Id::new("b").unwrap(),
            day_idx: hwm / DAY - 1,
            day_n: 1,
            week_idx: week_idx - 1,
            week_n: 3,
            tz_offset_min: None,
        },
        SignpostCaps {
            id: Id::new("c").unwrap(),
            day_idx: hwm / DAY,
            day_n: 1,
            week_idx,
            week_n: 1,
            tz_offset_min: None,
        },
    ];
    let (_, next) = decide(
        &st,
        &still(hwm, 0, 0),
        &[],
        &Policy::default(),
        &granted(),
        &clock(hwm),
        0,
    )
    .unwrap();
    let ids: Vec<_> = next.caps.signposts.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["c"]);
}

#[test]
fn caps_survive_a_restart_with_empty_mem() {
    // A new process starts from caps only (mem is never persisted): the day cap still holds.
    let mut sim = Sim::new(vec![circle("c", 200)]);
    let r = visits(&mut sim, T0, 2);
    assert!(r[0].0 && r[1].0);
    let persisted = to_canonical_json(&sim.state.caps).unwrap();
    let mut fresh = Sim::new(vec![circle("c", 200)]);
    fresh.state.caps = proxim_types::strict::from_canonical_json(persisted.as_bytes()).unwrap();
    let r = visits(&mut fresh, T0 + 2 * SPACING_MS, 1);
    assert!(!r[0].0);
    assert!(r[0].1.contains(&ReasonCode::CapSignpostDay));
}

#[test]
fn invalid_caps_state_is_a_whole_call_error() {
    let f = still(T0, 0, 0);
    let call = |st: &State| decide(st, &f, &[], &Policy::default(), &granted(), &clock(T0), 0);
    let mut dup = State::empty();
    let e = SignpostCaps {
        id: Id::new("a").unwrap(),
        day_idx: 1,
        day_n: 1,
        week_idx: 1,
        week_n: 1,
        tz_offset_min: None,
    };
    dup.caps.signposts = vec![e.clone(), e.clone()];
    assert_eq!(call(&dup), Err(proxim_types::EngineError::Parse));
    let mut unsorted = State::empty();
    let mut b = e.clone();
    b.id = Id::new("b").unwrap();
    unsorted.caps.signposts = vec![b, e];
    assert_eq!(call(&unsorted), Err(proxim_types::EngineError::Parse));
    let mut neg = State::empty();
    neg.caps.hwm_ms = -1;
    assert_eq!(call(&neg), Err(proxim_types::EngineError::Parse));
    let mut wrong_v = State::empty();
    wrong_v.v = 2;
    assert_eq!(call(&wrong_v), Err(proxim_types::EngineError::Parse));
    // An Err is "no decision": the caller keeps its state, we cannot have changed it.
    assert!(call(&State::empty()).is_ok());
}
