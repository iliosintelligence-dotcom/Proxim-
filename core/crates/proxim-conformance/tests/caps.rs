//! Spec 7 caps (F22 2/day, F23 5/week, F24 1800 s), local calendar buckets, the clock high-water
//! mark and the clock-backwards case. Natural multi-fire sequences plus seeded-state edge cases.
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

const ID: &str = "c-1";

/// Walk east from `from_m` to `to_m` at 50 m/s from time `t`, returns the next free time.
fn walk(rig: &mut Rig, from_m: i64, to_m: i64, mut t: i64) -> i64 {
    let mut e = from_m;
    while e != to_m {
        e = if to_m > e {
            (e + 50).min(to_m)
        } else {
            (e - 50).max(to_m)
        };
        let s = rig.step(&fix_at(t, offset(ORIGIN, e * 1000, 0)));
        assert!(s.fix_ok(), "{}", s.fix_reason());
        t += 1000;
    }
    t
}

/// One visit: dwell inside the circle until the event resolves (fire or a drop), then leave and
/// complete the EXIT (so the detector is in `cool`). Returns the resolving step and the next time at
/// which the cool-down has ended.
fn visit(rig: &mut Rig, t: i64) -> (Step, i64) {
    let mut resolved = None;
    let mut tt = t;
    for k in 0..10 {
        tt = t + k * 1000;
        let s = rig.step(&fix_at(tt, ORIGIN));
        assert!(s.fix_ok(), "{}", s.fix_reason());
        if s.fired(ID) || !s.drops(ID).is_empty() {
            resolved = Some(s);
            break;
        }
    }
    let resolved = resolved.expect("an event must resolve within 10 fixes");
    let mut t2 = walk(rig, 0, 170, tt + 1000);
    for _ in 0..5 {
        rig.step(&fix_at(t2, offset(ORIGIN, 170_000, 0)));
        t2 += 1000;
    }
    let st = rig.history.last().unwrap().det(ID).unwrap()["st"].clone();
    assert_eq!(st["s"], "cool", "visit must end in cool-down: {st}");
    (resolved, st["until_ms"].as_i64().unwrap())
}

fn rig() -> Rig {
    Rig::new().with_signpost(enter_circle(ID, 150))
}

#[test]
fn third_fire_in_a_local_day_is_blocked_by_cap_signpost_day() {
    let mut rig = rig();
    let t = T0 + 3_600_000;
    let (s1, t_next) = visit(&mut rig, t);
    assert!(s1.fired(ID));
    let (s2, t_next) = visit(&mut rig, t_next);
    assert!(
        s2.fired(ID),
        "second fire of the day is allowed: {}",
        s2.text
    );
    assert_eq!(s2.cap_entry(ID).unwrap()["day_n"], 2);
    let (s3, _) = visit(&mut rig, t_next);
    assert!(!s3.fired(ID));
    assert_eq!(
        s3.drops(ID),
        vec!["CAP_SIGNPOST_DAY".to_string()],
        "{}",
        s3.text
    );
    // the dropped event spent nothing
    assert_eq!(s3.cap_entry(ID).unwrap()["day_n"], 2);
    assert_eq!(s3.cap_entry(ID).unwrap()["week_n"], 2);
}

#[test]
fn sixth_fire_in_a_week_is_blocked_and_the_next_week_resets() {
    let mut rig = rig();
    let mut fired = 0;
    let mut blocked_week = None;
    // Monday x2, Tuesday x2, Wednesday x1, then a second Wednesday visit
    let starts = [
        T0 + 3_600_000,
        0, // chained
        T0 + DAY_MS + 3_600_000,
        0,
        T0 + 2 * DAY_MS + 3_600_000,
        0,
    ];
    let mut t_next = 0;
    for (i, st) in starts.iter().enumerate() {
        let t = if *st == 0 { t_next } else { (*st).max(t_next) };
        let (s, n) = visit(&mut rig, t);
        t_next = n;
        if s.fired(ID) {
            fired += 1;
        } else {
            blocked_week = Some((i, s.drops(ID)));
        }
    }
    assert_eq!(fired, 5, "5 fires fit in the week (2 + 2 + 1)");
    let (i, drops) = blocked_week.expect("the 6th visit must be blocked");
    assert_eq!(i, 5);
    assert_eq!(
        drops,
        vec!["CAP_SIGNPOST_WEEK".to_string()],
        "day_n is 1 so the week cap is the reason"
    );
    // the next Monday: a new week, the count restarts at 1 and the old bucket is replaced
    let next_monday = T0 + 7 * DAY_MS + 3_600_000;
    let (s, _) = visit(&mut rig, next_monday);
    assert!(s.fired(ID), "{}", s.text);
    let e = s.cap_entry(ID).unwrap();
    assert_eq!(
        (e["day_n"].as_i64(), e["week_n"].as_i64()),
        (Some(1), Some(1))
    );
    assert_eq!(e["week_idx"], week_idx(next_monday, 0));
}

#[test]
fn cap_app_interval_blocks_a_second_signpost_within_1800_s() {
    // two different signposts, 600 m apart: one card per 30 minutes per app
    let mut rig = Rig::new()
        .with_signpost(enter_circle_at("a-1", ORIGIN, 150))
        .with_signpost(enter_circle_at("b-1", offset(ORIGIN, 600_000, 0), 150));
    let mut t = BASE_T;
    let mut fired_a = false;
    for _ in 0..6 {
        let s = rig.step(&fix_at(t, ORIGIN));
        fired_a |= s.fired("a-1");
        t += 1000;
    }
    assert!(fired_a);
    t = walk(&mut rig, 0, 600, t);
    let mut b_reason = None;
    for _ in 0..8 {
        let s = rig.step(&fix_at(t, offset(ORIGIN, 600_000, 0)));
        assert!(!s.fired("b-1"), "b-1 must wait 1800 s: {}", s.text);
        if !s.drops("b-1").is_empty() {
            b_reason = Some(s.drops("b-1"));
        }
        t += 1000;
    }
    assert_eq!(b_reason, Some(vec!["CAP_APP_INTERVAL".to_string()]));
}

// ---- seeded boundaries -------------------------------------------------------------------------

/// A rig after 3 inside fixes (the 4th fires) with `caps` seeded for `ID`.
fn primed(entry: Option<Value>, last_fire: Option<i64>, tz: i16) -> (Rig, i64) {
    let mut rig = Rig::new().with_signpost(enter_circle(ID, 200));
    rig.tz = tz;
    for k in 0..3 {
        rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
    }
    let mut st = rig.state_json();
    if let Some(e) = entry {
        st["caps"]["signposts"] = json!([e]);
    }
    if let Some(l) = last_fire {
        st["caps"]["last_fire_ms"] = json!(l);
    }
    rig.state = render(&st);
    (rig, BASE_T + 3000)
}

fn entry(day_idx: i64, day_n: i64, week_idx: i64, week_n: i64) -> Value {
    json!({"id": ID, "day_idx": day_idx, "day_n": day_n, "week_idx": week_idx, "week_n": week_n})
}

fn cur_entry(t: i64, tz: i16, day_n: i64, week_n: i64) -> Value {
    entry(day_idx(t, tz), day_n, week_idx(t, tz), week_n)
}

#[test]
fn day_cap_counts_boundaries() {
    for (day_n, week_n, blocked) in [
        (0, 0, false),
        (1, 1, false),
        (2, 2, true),
        (3, 3, true),
        (1, 4, false),
        (1, 5, true),
    ] {
        let (mut rig, t) = primed(Some(cur_entry(BASE_T + 3000, 0, day_n, week_n)), None, 0);
        let s = rig.step(&fix_at(t, ORIGIN));
        if blocked {
            let want = if day_n >= 2 {
                "CAP_SIGNPOST_DAY"
            } else {
                "CAP_SIGNPOST_WEEK"
            };
            assert_eq!(
                s.drops(ID),
                vec![want.to_string()],
                "day {day_n} week {week_n}"
            );
        } else {
            assert!(s.fired(ID), "day {day_n} week {week_n}: {}", s.text);
            let e = s.cap_entry(ID).unwrap();
            assert_eq!(
                (e["day_n"].as_i64(), e["week_n"].as_i64()),
                (Some(day_n + 1), Some(week_n + 1))
            );
        }
    }
}

#[test]
fn partner_caps_only_narrow() {
    // larger partner caps clamp to 2 / 5; smaller ones are honoured
    for (key, val, d, w, blocked) in [
        ("cap_signpost_day", 10, 2, 2, true),
        ("cap_signpost_day", u32::MAX as i64, 2, 2, true),
        ("cap_signpost_day", 1, 1, 1, true),
        ("cap_signpost_day", 2, 1, 1, false),
        ("cap_signpost_week", 100, 1, 5, true),
        ("cap_signpost_week", 2, 1, 2, true),
        ("cap_signpost_week", 5, 1, 4, false),
    ] {
        let (mut rig, t) = primed(Some(cur_entry(BASE_T + 3000, 0, d, w)), None, 0);
        rig.policy[key] = json!(val);
        let s = rig.step(&fix_at(t, ORIGIN));
        assert_eq!(
            !s.fired(ID),
            blocked,
            "{key}={val} seeded {d}/{w}: {}",
            s.text
        );
    }
}

#[test]
fn caps_are_keyed_by_id_not_revision() {
    let (mut rig, t) = primed(Some(cur_entry(BASE_T + 3000, 0, 2, 2)), None, 0);
    // same id, bumped revision: the detector restarts, so run enough fixes to reach the event again
    rig.signposts[0]["rev"] = json!(2);
    let mut reasons = vec![];
    for k in 0..6 {
        let s = rig.step(&fix_at(t + k * 1000, ORIGIN));
        assert!(!s.fired(ID));
        reasons.extend(s.drops(ID));
    }
    assert!(
        reasons.contains(&"CAP_SIGNPOST_DAY".to_string()),
        "{reasons:?}"
    );
}

#[test]
fn app_interval_is_exact_and_partners_may_only_lengthen_it() {
    let now = BASE_T + 3000;
    for (ago_ms, interval, blocked) in [
        (1_799_999_i64, None, true),
        (1_800_000, None, false),
        (60_000, None, true),
        (60_000, Some(10), true), // shorter than 1800 s is clamped up
        (60_000, Some(0), true),
        (2_000_000, Some(3600), true), // partner lengthens to an hour
        (3_600_000, Some(3600), false),
        (3_599_999, Some(3600), true),
    ] {
        let (mut rig, t) = primed(None, Some(now - ago_ms), 0);
        if let Some(i) = interval {
            rig.policy["app_min_interval_s"] = json!(i);
        }
        let s = rig.step(&fix_at(t, ORIGIN));
        if blocked {
            assert_eq!(
                s.drops(ID),
                vec!["CAP_APP_INTERVAL".to_string()],
                "ago {ago_ms} interval {interval:?}"
            );
        } else {
            assert!(
                s.fired(ID),
                "ago {ago_ms} interval {interval:?}: {}",
                s.text
            );
        }
    }
}

#[test]
fn daily_buckets_follow_the_local_time_zone() {
    // 23:30 UTC. With tz +60 it is already the next local day, so yesterday's count does not apply.
    let t = T0 + 23 * 3_600_000 + 30 * 60_000; // Monday 23:30 UTC
    for (tz, expect_blocked) in [(0_i16, true), (60, false), (-60, true)] {
        let mut rig = Rig::new().with_signpost(enter_circle(ID, 200));
        rig.tz = tz;
        for k in 0..3 {
            rig.step(&fix_at(t + k * 1000, ORIGIN));
        }
        let mut st = rig.state_json();
        // two fires "today in UTC" (day D)
        st["caps"]["signposts"] = json!([entry(day_idx(t, 0), 2, week_idx(t, 0), 2)]);
        rig.state = render(&st);
        let s = rig.step(&fix_at(t + 3000, ORIGIN));
        assert_eq!(!s.fired(ID), expect_blocked, "tz {tz}: {}", s.text);
        if !expect_blocked {
            let e = s.cap_entry(ID).unwrap();
            assert_eq!(e["day_idx"], day_idx(t, tz), "tz {tz}");
            assert_eq!(e["day_n"], 1, "a new local day restarts the day count");
        }
    }
}

#[test]
fn day_and_week_indices_match_the_spec_formula() {
    // day_idx = floor_div(now + tz x 60000, 86_400_000); week_idx = floor_div(day_idx + 3, 7) (Monday start)
    let times = [
        T0,
        T0 - 1,                       // Sunday 23:59:59.999 UTC
        T0 + 6 * DAY_MS + 86_399_999, // Sunday end of week
        T0 + 7 * DAY_MS,
        T0 + 12 * 3_600_000,
        1_000_000_000_000,
    ];
    for t in times {
        for tz in [-720_i16, -330, -60, 0, 59, 60, 330, 840] {
            let mut rig = Rig::new().with_signpost(enter_circle(ID, 200));
            rig.tz = tz;
            let mut fired = None;
            for k in 0..5 {
                let s = rig.call(&fix_at(t + k * 1000, ORIGIN), t + k * 1000);
                if s.fired(ID) {
                    fired = Some(s);
                    break;
                }
            }
            let s = fired.unwrap_or_else(|| panic!("no fire at t={t} tz={tz}"));
            let e = s.cap_entry(ID).unwrap();
            let now = t + 3000;
            assert_eq!(e["day_idx"], day_idx(now, tz), "day_idx t={t} tz={tz}");
            assert_eq!(e["week_idx"], week_idx(now, tz), "week_idx t={t} tz={tz}");
        }
    }
    // Monday 2026-10-12 starts a new week; the millisecond before belongs to the previous one
    assert_eq!(week_idx(T0, 0), week_idx(T0 - 1, 0) + 1);
    assert_eq!(week_idx(T0 + 6 * DAY_MS + 86_399_999, 0), week_idx(T0, 0));
}

#[test]
fn entries_from_earlier_weeks_are_pruned_each_call() {
    let (mut rig, t) = primed(None, None, 0);
    let now = t;
    let w = week_idx(now, 0);
    let mut st = rig.state_json();
    st["caps"]["signposts"] = json!([
        {"id": "a-old", "day_idx": day_idx(now, 0) - 9, "day_n": 2, "week_idx": w - 1, "week_n": 5},
        {"id": ID, "day_idx": day_idx(now, 0), "day_n": 0, "week_idx": w, "week_n": 0},
        {"id": "z-cur", "day_idx": day_idx(now, 0), "day_n": 1, "week_idx": w, "week_n": 1},
    ]);
    rig.state = render(&st);
    let s = rig.step(&fix_at(t, ORIGIN));
    let ids: Vec<String> = s.caps()["signposts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ids,
        vec![ID.to_string(), "z-cur".to_string()],
        "earlier-week entry pruned, ascending by id"
    );
}

// ---- clock behaviour ---------------------------------------------------------------------------

#[test]
fn high_water_mark_ratchets_and_a_clock_set_backwards_cannot_reopen_a_cap() {
    // (a) clock 20 s behind the high-water mark: now is clamped to hwm
    let (mut rig, t) = primed(Some(cur_entry(BASE_T + 3000, 0, 2, 2)), None, 0);
    let hwm_before = rig.state_json()["caps"]["hwm_ms"].as_i64().unwrap();
    let s = rig.call(&fix_at(t, ORIGIN), t - 20_000);
    assert_eq!(
        s.output["t_ms"],
        hwm_before.max(t - 20_000),
        "Output.t_ms is the clamped clock"
    );
    assert_eq!(s.drops(ID), vec!["CAP_SIGNPOST_DAY".to_string()]);

    // (b) clock one whole day behind but fix times correct: day index comes from the clamped now
    let (mut rig, t) = primed(Some(cur_entry(BASE_T + 3000, 0, 2, 2)), None, 0);
    let s = rig.call(&fix_at(t, ORIGIN), t - DAY_MS);
    assert!(s.is_ok() && s.fix_ok(), "{}", s.text);
    assert!(
        !s.fired(ID),
        "a backwards clock must not reopen the day cap: {}",
        s.text
    );
    assert_eq!(s.drops(ID), vec!["CAP_SIGNPOST_DAY".to_string()]);
    assert!(
        s.caps()["hwm_ms"].as_i64().unwrap() >= t - 1000,
        "hwm never decreases"
    );

    // (c) a fix stamped by the wrong (backwards) clock is stale against the high-water mark
    let (mut rig, t) = primed(Some(cur_entry(BASE_T + 3000, 0, 2, 2)), None, 0);
    let s = rig.call(&fix_at(t - DAY_MS, ORIGIN), t - DAY_MS);
    assert!(!s.fired(ID));
    assert!(
        !s.fix_ok(),
        "fix from the past against the hwm is rejected: {}",
        s.text
    );
    assert!(
        matches!(s.fix_reason().as_str(), "FIX_STALE" | "FIX_OUT_OF_ORDER"),
        "{}",
        s.fix_reason()
    );

    // (d) the state's hwm_ms never goes down across any accepted call
    let (mut rig, t) = primed(None, None, 0);
    let mut last = rig.state_json()["caps"]["hwm_ms"].as_i64().unwrap();
    for (k, lag) in [0_i64, -5000, -1000, 3000, -29_000, 0].iter().enumerate() {
        let tt = t + k as i64 * 1000;
        let s = rig.call(&fix_at(tt, ORIGIN), tt + lag);
        if s.is_ok() {
            let h = s.caps()["hwm_ms"].as_i64().unwrap();
            assert!(h >= last, "hwm went backwards: {last} -> {h}");
            last = h;
        }
    }
}

#[test]
fn a_far_future_clock_ratchets_hwm_and_blocks_later_correct_fixes_failing_closed() {
    // Spec 7: now = max(clock, hwm), stored as caps.hwm_ms (persisted). One accepted call stamped a
    // year ahead lifts the mark; afterwards correctly stamped fixes are stale or out of order and
    // nothing can fire. Fail closed, but with no recovery path (reported as spec issue S-HWM).
    let year = 365 * DAY_MS;
    let mut rig = Rig::new().with_signpost(enter_circle(ID, 200));
    let s = rig.call(&fix_at(BASE_T + year, ORIGIN), BASE_T + year);
    assert!(s.fix_ok(), "{}", s.text);
    assert_eq!(s.caps()["hwm_ms"], BASE_T + year);
    for k in 0..8 {
        let s = rig.call(
            &fix_at(BASE_T + 1000 + k * 1000, ORIGIN),
            BASE_T + 1000 + k * 1000,
        );
        assert!(s.is_ok());
        assert!(!s.fix_ok(), "correct time after a ratchet is rejected");
        assert!(
            matches!(s.fix_reason().as_str(), "FIX_STALE" | "FIX_OUT_OF_ORDER"),
            "{}",
            s.fix_reason()
        );
        assert!(s.fires().is_empty());
    }
}

#[test]
fn consent_withdrawal_and_standoff_keep_the_caps_that_were_earned() {
    let (mut rig, t) = fired_state();
    let before = rig.state_json()["caps"].clone();
    rig.consent["location_matching"] = json!("denied");
    let s = rig.call(&fix_at(t + 1000, ORIGIN), t + 1000);
    assert_eq!(s.state["caps"], before);
    rig.consent["location_matching"] = json!("granted");
    // consent returns: caps still count (the ENTER must be earned again, but the app interval holds)
    let mut reasons = vec![];
    for k in 2..12 {
        let s = rig.call(&fix_at(t + k * 1000, ORIGIN), t + k * 1000);
        assert!(
            !s.fired(ID),
            "app interval 1800 s still applies after a consent round trip"
        );
        reasons.extend(s.drops(ID));
    }
    assert!(
        reasons.contains(&"CAP_APP_INTERVAL".to_string()),
        "{reasons:?}"
    );
}

#[test]
fn negative_local_days_and_week_boundaries_use_floor_division() {
    // 1970-01-05 (day_idx 4) is a Monday: week_idx = floor_div(4 + 3, 7) = 1. One hour west of UTC
    // at 00:00:03 UTC it is still Sunday 1970-01-04 locally: day_idx 3, week_idx 0.
    // At 00:00:03 UTC on day 0 with tz -720 the local day is -1 (floor, not truncation).
    for (t, tz, want_day, want_week) in [
        (4 * DAY_MS + 3000, -60_i16, 3_i64, 0_i64),
        (4 * DAY_MS + 3000, 0, 4, 1),
        (4 * DAY_MS + 3000, 840, 4, 1),
        (3000, -720, -1, 0),
        (3000, -840, -1, 0),
        (3000, 0, 0, 0),
        (3000 + 2 * DAY_MS, -840, 1, 0),
    ] {
        let mut rig = Rig::new().with_signpost(enter_circle(ID, 200));
        rig.tz = tz;
        let mut entry = None;
        for k in 0..6 {
            let s = rig.call(&fix_at(t + k * 1000, ORIGIN), t + k * 1000);
            assert!(s.fix_ok(), "t={t} tz={tz}: {}", s.fix_reason());
            if s.fired(ID) {
                entry = s.cap_entry(ID);
                break;
            }
        }
        let e = entry.unwrap_or_else(|| panic!("no fire at t={t} tz={tz}"));
        assert_eq!(
            (e["day_idx"].as_i64(), e["week_idx"].as_i64()),
            (Some(want_day), Some(want_week)),
            "t={t} tz={tz}"
        );
    }
}

#[test]
#[ignore = "spec-violation: S-TZ-CAP-BYPASS"]
fn changing_tz_offset_min_must_not_reopen_an_exhausted_day_cap() {
    // Spec 7 says a new index resets the count, so a caller that varies `tz_offset_min` between calls
    // moves "today" and gets a fresh 2/day. The platform cap (spec 3.3: partners only narrow, 2/day)
    // is then bypassable by whoever supplies the offset. Safe property: within one UTC day (24 h)
    // an exhausted cap stays exhausted whatever offset the next call carries.
    let t = BASE_T;
    // two fires already counted for local day D under tz +840
    let (mut rig, _) = primed(Some(cur_entry(t + 3000, 840, 2, 2)), None, 840);
    let s = rig.step(&fix_at(BASE_T + 3000, ORIGIN));
    assert_eq!(
        s.drops(ID),
        vec!["CAP_SIGNPOST_DAY".to_string()],
        "same offset: blocked"
    );
    // same instant, offset moved west by 26 h: the engine opens a new bucket
    let (mut rig, _) = primed(Some(cur_entry(t + 3000, 840, 2, 2)), None, -720);
    let s = rig.step(&fix_at(BASE_T + 3000, ORIGIN));
    assert!(
        !s.fired(ID),
        "a different tz_offset_min reopened the 2/day cap: {}",
        s.text
    );
}
