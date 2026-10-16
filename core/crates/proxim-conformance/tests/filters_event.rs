//! Spec 7, event filters F20-F24 (a dropped Event is consumed, not retried), driven through a real
//! APPROACH event so the cross-track distance (F21) is non-zero.
//!
//! Method: replay a fixed 1 Hz drive up to the step BEFORE the fire step `K`, then make the K-th call
//! with the variant (lagging clock for F20, tight `max_event_dist_m` for F21, seeded `caps` for
//! F22-F24). Because caps and the clock never influence detection, the event exists at step `K`
//! in every variant, so the reported reason is purely the filter chain.
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
use serde_json::{Value, json};

const ID: &str = "r-1";

fn drive() -> Drive {
    let mut d = Drive::new(2500, 15_000);
    d.lateral_mm = 12_000; // 12 m east of the route axis: inside the 30 m buffer, cross-track 12 m
    d
}

fn signpost(max_dist_m: i64, max_age_s: i64) -> Value {
    let mut sp = approach_route(ID, 2500);
    set(&mut sp, "trigger.max_event_dist_m", json!(max_dist_m));
    set(&mut sp, "trigger.max_event_age_s", json!(max_age_s));
    sp
}

/// The fire step index of the baseline drive.
fn fire_step() -> i64 {
    static K: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    *K.get_or_init(compute_fire_step)
}

fn compute_fire_step() -> i64 {
    let mut rig = Rig::new().with_signpost(signpost(100, 100));
    let (k, d) = drive_until_fire(&mut rig, &drive(), ID, 300).expect("baseline fires");
    assert_eq!(d["event"], "approach");
    k
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum F {
    TooOld = 20,
    TooFar = 21,
    Day = 22,
    Week = 23,
    App = 24,
}

const ALL: [F; 5] = [F::TooOld, F::TooFar, F::Day, F::Week, F::App];

fn code(f: F) -> &'static str {
    match f {
        F::TooOld => "EVENT_TOO_OLD",
        F::TooFar => "EVENT_TOO_FAR",
        F::Day => "CAP_SIGNPOST_DAY",
        F::Week => "CAP_SIGNPOST_WEEK",
        F::App => "CAP_APP_INTERVAL",
    }
}

/// Replay to step K-1, then make call K with the filters in `on` violated.
fn run_variant(on: &[F]) -> (Rig, Step, i64) {
    let k_fire = fire_step();
    let has = |f: F| on.contains(&f);
    // F21: cross-track is 12 m; max_event_dist_m 5 m rejects, 100 m accepts
    // F20: the max age is 1 s and the clock lags the fix by 5 s at the K-th call
    let sp = signpost(
        if has(F::TooFar) { 5 } else { 100 },
        if has(F::TooOld) { 1 } else { 100 },
    );
    let mut rig = Rig::new().with_signpost(sp);
    let d = drive();
    for k in 0..k_fire {
        let s = rig.step(&d.fix(k));
        assert!(
            s.fires().is_empty() && s.drops(ID).is_empty(),
            "step {k}: {}",
            s.text
        );
    }
    // seed caps (they do not influence detection) for F22-F24
    let t = d.t_ms(k_fire);
    let mut st = rig.state_json();
    let (di, wi) = (day_idx(t, 0), week_idx(t, 0));
    let day_n = if has(F::Day) { 2 } else { 1 };
    let week_n = if has(F::Week) {
        5
    } else if has(F::Day) {
        2
    } else {
        1
    };
    if has(F::Day) || has(F::Week) || has(F::App) {
        st["caps"]["signposts"] =
            json!([{"id": ID, "day_idx": di, "day_n": day_n, "week_idx": wi, "week_n": week_n}]);
    }
    if has(F::App) {
        st["caps"]["last_fire_ms"] = json!(t - 60_000);
    }
    rig.state = render(&st);
    let now = if has(F::TooOld) { t + 5000 } else { t };
    let step = rig.call(&d.fix(k_fire), now);
    (rig, step, k_fire)
}

#[test]
fn control_fires_with_no_violation() {
    let (_, s, _) = run_variant(&[]);
    assert!(s.fired(ID), "{}", s.text);
    assert_eq!(s.fires()[0]["path_tier"], 1);
}

#[test]
fn each_event_filter_alone_reports_its_own_code() {
    for f in ALL {
        let (_, s, _) = run_variant(&[f]);
        assert!(s.is_ok() && s.fix_ok(), "{f:?}: {}", s.text);
        assert!(!s.fired(ID), "{f:?} must stop the fire");
        assert_eq!(s.drops(ID), vec![code(f).to_string()], "{f:?}: {}", s.text);
    }
}

#[test]
fn ordering_every_subset_of_event_filters_reports_the_first() {
    let mut checked = 0;
    for mask in 1_u32..32 {
        let on: Vec<F> = ALL
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, f)| f)
            .collect();
        let (_, s, _) = run_variant(&on);
        assert!(!s.fired(ID), "{on:?} fired: {}", s.text);
        assert_eq!(
            s.drops(ID),
            vec![code(on[0]).to_string()],
            "{on:?}: first in spec order must win"
        );
        checked += 1;
    }
    assert_eq!(checked, 31);
}

#[test]
fn a_dropped_event_is_consumed_and_spends_no_cap() {
    for f in [F::TooOld, F::TooFar] {
        let (mut rig, s, k) = run_variant(&[f]);
        assert!(!s.fired(ID));
        // no cap was written by the dropped event
        assert!(
            s.cap_entry(ID).is_none(),
            "{f:?} must not spend a cap: {:?}",
            s.cap_entry(ID)
        );
        assert!(s.caps().get("last_fire_ms").is_none());
        // the detector does not retry: no fire and no repeated event drop on the next 20 fixes
        let d = drive();
        for j in 1..=20 {
            let s = rig.step(&d.fix(k + j));
            assert!(s.fires().is_empty(), "{f:?}: retried at +{j}");
            assert!(
                s.drops(ID).is_empty(),
                "{f:?}: event dropped again at +{j}: {:?}",
                s.drops(ID)
            );
        }
    }
}

#[test]
fn fire_commits_exactly_one_day_one_week_and_last_fire() {
    let (_, s, k) = run_variant(&[]);
    let d = drive();
    let now = d.t_ms(k);
    let e = s.cap_entry(ID).expect("cap entry written by the winner");
    assert_eq!(
        (e["day_n"].as_i64(), e["week_n"].as_i64()),
        (Some(1), Some(1)),
        "a first fire counts 1 day, 1 week"
    );
    assert_eq!(e["day_idx"], day_idx(now, 0));
    assert_eq!(e["week_idx"], week_idx(now, 0));
    assert_eq!(s.caps()["last_fire_ms"], now);
    assert_eq!(s.caps()["hwm_ms"], now);
}

#[test]
fn event_age_and_distance_defaults_do_not_bind_a_fresh_foreground_fix() {
    // Spec 7: F20/F21 are vacuous for a fresh fix with default settings.
    let mut rig = Rig::new().with_signpost(approach_route(ID, 2500));
    let mut d = drive();
    d.lateral_mm = 25_000; // 25 m off axis, still in the 30 m corridor; far below the 5000 m default
    assert!(drive_until_fire(&mut rig, &d, ID, 300).is_some());
}

#[test]
fn enter_event_too_old_with_a_lagging_clock() {
    // ENTER: cross-track is always 0, so only F20 can bind. Fix 3 s old relative to the clock,
    // max_event_age_s = 1.
    let mut sp = enter_circle("c-1", 200);
    set(&mut sp, "trigger.max_event_age_s", json!(1));
    let mut rig = Rig::new().with_signpost(sp);
    for k in 0..3 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        assert!(s.fires().is_empty());
    }
    let s = rig.call(&fix_at(BASE_T + 3000, ORIGIN), BASE_T + 8000);
    assert_eq!(
        s.drops("c-1"),
        vec!["EVENT_TOO_OLD".to_string()],
        "{}",
        s.text
    );
    // the 1 s limit is exclusive of equality: age exactly 1000 ms is fine
    let mut sp = enter_circle("c-1", 200);
    set(&mut sp, "trigger.max_event_age_s", json!(1));
    let mut rig = Rig::new().with_signpost(sp);
    for k in 0..3 {
        rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
    }
    let s = rig.call(&fix_at(BASE_T + 3000, ORIGIN), BASE_T + 4000);
    assert!(
        s.fired("c-1"),
        "age == max_event_age_s x 1000 is not too old: {}",
        s.text
    );
}
