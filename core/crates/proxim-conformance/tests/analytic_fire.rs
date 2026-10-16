//! Spec 11 / 6.2 analytic values: time-to-anchor fire distances and the ENTER/EXIT radii.
//!
//! Tolerance rationale (spec 6.2): `D_fire = clamp(v_eff x (lead_s + 3 s), d_lo, d_hi)` is the
//! distance at which the rule STARTS to hold. Firing needs the rule to hold on two accepted fixes at
//! least 1000 ms apart, so at 1 Hz the realised fire is 1 or 2 steps (1 s each) inside `D_fire`
//! (errata E-P7 allows up to 3). The frame scale error is 0.1 % at lat 45. The window used
//! everywhere below is therefore `[D_fire - 3 steps - 0.2 %, D_fire + 0.2 %]`.
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

/// Run a steady 1 Hz drive and return (fix index, d_rem_mm, t_hat_ms, decision).
fn fire_of(sp: serde_json::Value, d: &Drive, steps: i64) -> (i64, i64, i64, serde_json::Value) {
    let mut rig = Rig::new().with_signpost(sp);
    let (k, dec) = drive_until_fire(&mut rig, d, "r-1", steps).expect("signpost must fire");
    (
        k,
        dec["d_rem_mm"].as_i64().expect("fire carries d_rem_mm"),
        dec["t_hat_ms"].as_i64().expect("fire carries t_hat_ms"),
        dec,
    )
}

fn assert_window(name: &str, d_rem_mm: i64, d_fire_m: i64, step_mm: i64) {
    let d_fire = d_fire_m * 1000;
    let tol = d_fire / 500; // 0.2 %
    let lo = d_fire - 3 * step_mm - tol;
    let hi = d_fire + tol;
    assert!(
        (lo..=hi).contains(&d_rem_mm),
        "{name}: fired at d_rem {d_rem_mm} mm, expected within [{lo}, {hi}] (D_fire {d_fire} mm)"
    );
}

#[test]
fn highway_29_mps_fires_near_1247_m() {
    let d = Drive::new(2500, 29_000);
    let (k, d_rem, t_hat, dec) = fire_of(approach_route("r-1", 2500), &d, 90);
    assert_window("29 m/s", d_rem, 1247, 29_000);
    // "realized lead about 41 s": between 38 and 43 s of time-to-anchor
    assert!((38_000..=43_500).contains(&t_hat), "t_hat {t_hat}");
    assert_eq!(dec["event"], "approach");
    assert_eq!(
        dec["copy"], "ahead",
        "P1 corridor tier uses the `ahead` copy"
    );
    assert_eq!(dec["path_tier"], 1);
    assert!(k >= 4, "arming needs a 4 s baseline, fired at index {k}");
}

#[test]
fn urban_15_mps_fires_near_645_m() {
    let d = Drive::new(2500, 15_000);
    let (_, d_rem, t_hat, _) = fire_of(approach_route("r-1", 2500), &d, 200);
    assert_window("15 m/s", d_rem, 645, 15_000);
    assert!((38_000..=43_500).contains(&t_hat), "t_hat {t_hat}");
}

#[test]
fn cyclist_5_mps_fires_near_215_m() {
    let mut sp = approach_route("r-1", 1500);
    set(&mut sp, "trigger.mode", json!("cycle"));
    let d = Drive::new(1500, 5_000);
    let (_, d_rem, _, _) = fire_of(sp, &d, 400);
    assert_window("cycle 5 m/s", d_rem, 215, 5_000);
}

#[test]
fn traffic_jam_3_mps_uses_the_4_mps_floor_and_fires_near_172_m() {
    // v_eff = max(vf, 4 m/s floor) = 4 m/s, so D_fire = 4 x 43 = 172 m (not 3 x 43 = 129 m).
    let d = Drive::new(600, 3_000);
    let (_, d_rem, _, _) = fire_of(approach_route("r-1", 600), &d, 400);
    assert_window("jam 3 m/s", d_rem, 172, 3_000);
    assert!(
        d_rem > 129_000 + 3 * 3_000,
        "must not fire at the unfloored 129 m distance: {d_rem}"
    );
}

#[test]
fn crawl_rule_clamps_to_d_lo_150_m_for_vehicle() {
    // lead 15 s (policy minimum), v_eff floor 4 m/s: 4 x 18 = 72 m < d_lo 150 m -> clamp up to 150 m.
    let mut sp = approach_route("r-1", 600);
    set(&mut sp, "trigger.lead_s", json!(15));
    let d = Drive::new(600, 3_000);
    let (_, d_rem, _, _) = fire_of(sp, &d, 400);
    assert_window("crawl", d_rem, 150, 3_000);
}

#[test]
fn upper_clamp_d_hi_3000_m_default() {
    // lead 120 s at 29 m/s: 29 x 123 = 3567 m > d_hi 3000 m -> D_fire = 3000 m.
    let mut sp = approach_route("r-1", 3400);
    set(&mut sp, "trigger.lead_s", json!(120));
    let d = Drive::new(3400, 29_000);
    let (_, d_rem, _, _) = fire_of(sp, &d, 120);
    assert_window("d_hi", d_rem, 3000, 29_000);
    assert!(d_rem <= 3_000_000, "never fires beyond d_hi: {d_rem}");
}

#[test]
fn faster_traffic_fires_earlier_monotonically() {
    let mut last = i64::MAX;
    // distances must be strictly decreasing as speed decreases (D_fire is linear in v_eff)
    for v in [29_000_i64, 20_000, 15_000, 10_000, 6_000] {
        let d = Drive::new(2500, v);
        let (_, d_rem, _, _) = fire_of(approach_route("r-1", 2500), &d, 400);
        assert!(d_rem < last, "v={v}: {d_rem} not below previous {last}");
        last = d_rem;
        assert_window(&format!("v={v}"), d_rem, v * 43 / 1000, v);
    }
}

#[test]
fn fires_once_per_pass_and_not_on_the_way_out() {
    // latched{fired}: one fire per pass; passing the anchor does not re-arm.
    let d = Drive::new(2500, 20_000);
    let mut rig = Rig::new().with_signpost(approach_route("r-1", 2500));
    let mut fires = 0;
    for k in 0..300 {
        // keep driving north through and far beyond the anchor (route ends at k = 125)
        let s = rig.step(&d.fix(k));
        assert!(s.is_ok(), "{}", s.text);
        fires += s.fires().len();
    }
    assert_eq!(fires, 1, "exactly one fire per pass");
}

#[test]
fn late_join_is_a_terminal_drop_not_a_fire() {
    // Section 11: a first armed fix with t_hat < L_min (= max(0.4 x 40 s, 10 s) = 16 s) and
    // d_rem > d_lo (150 m) must not fire. Start 250 m out at 15 m/s: the first fix that can arm
    // (4 s baseline) is ~190 m out, t_hat ~12.7 s.
    let mut d = Drive::new(2500, 15_000);
    d.start_along_mm = 2_500_000 - 250_000;
    let mut rig = Rig::new().with_signpost(approach_route("r-1", 2500));
    let mut late = false;
    for k in 0..40 {
        let s = rig.step(&d.fix(k));
        assert!(
            !s.fired("r-1"),
            "late join fired at step {k}: {:?}",
            s.fires()
        );
        late |= s.drops("r-1").contains(&"TOO_LATE_FOR_LEAD".to_string());
    }
    assert!(late, "expected a TOO_LATE_FOR_LEAD drop");
}

#[test]
fn late_join_after_a_three_minute_gps_drop_does_not_fire() {
    // 6 km route; drive 20 s at 15 m/s, lose GPS for 180 s (2.7 km), reappear 300 m before the
    // anchor... the first post-gap armed fix has t_hat < 16 s and d_rem > 150 m.
    let len_mm = 6_000_000;
    let mut d = Drive::new(6000, 15_000);
    let mut rig = Rig::new().with_signpost(approach_route("r-1", 6000));
    for k in 0..20 {
        let s = rig.step(&d.fix(k));
        assert!(!s.fired("r-1"));
    }
    // resume 250 m from the anchor, 200 s after the start
    d.start_along_mm = len_mm - 250_000;
    d.t0 = BASE_T + 200_000;
    let mut late = false;
    for k in 0..40 {
        let s = rig.step(&d.fix(k));
        assert!(s.is_ok(), "{}", s.text);
        assert!(!s.fired("r-1"), "fired after GPS drop at step {k}");
        late |= s.drops("r-1").contains(&"TOO_LATE_FOR_LEAD".to_string());
    }
    assert!(late, "expected TOO_LATE_FOR_LEAD after the gap");
}

// ---------------------------------------------------------------------------------------------
// ENTER / EXIT radii, section 11: radius 150 m, acc_eff 10 m: ENTER at <= 140 m, EXIT at >= 160 m
// ---------------------------------------------------------------------------------------------

fn enter_fixes(
    rig: &mut Rig,
    n: i64,
    east_m: i64,
    t0: i64,
    dt_ms: i64,
    speed: Option<i64>,
) -> Vec<Step> {
    (0..n)
        .map(|k| {
            let mut f = fix_at(t0 + k * dt_ms, offset(ORIGIN, east_m * 1000, 0));
            match speed {
                Some(v) => f["speed_mmps"] = json!(v),
                None => unset(&mut f, "speed_mmps"),
            }
            rig.step(&f)
        })
        .collect()
}

/// Walk east from `from_m` to `to_m` at 50 m per second (below the 83 m/s FIX_JUMP threshold),
/// one fix per second starting at `t`; returns the next free time.
fn walk(rig: &mut Rig, from_m: i64, to_m: i64, mut t: i64) -> i64 {
    let mut e = from_m;
    while e != to_m {
        e = if to_m > e {
            (e + 50).min(to_m)
        } else {
            (e - 50).max(to_m)
        };
        let s = rig.step(&fix_at(t, offset(ORIGIN, e * 1000, 0)));
        assert!(s.fix_ok(), "walk fix rejected: {}", s.fix_reason());
        t += 1000;
    }
    t
}

#[test]
fn enter_fires_at_138_m_but_not_at_142_m_radius_150() {
    let mut r_in = Rig::new().with_signpost(enter_circle("c-1", 150));
    let steps = enter_fixes(&mut r_in, 8, 138, BASE_T, 1000, Some(10_000));
    assert!(
        steps.iter().any(|s| s.fired("c-1")),
        "138 m is inside by acc_eff"
    );

    let mut r_out = Rig::new().with_signpost(enter_circle("c-1", 150));
    let steps = enter_fixes(&mut r_out, 30, 142, BASE_T, 1000, Some(10_000));
    assert!(
        steps.iter().all(|s| !s.fired("c-1")),
        "142 m straddles: never ENTERs"
    );
}

#[test]
fn enter_needs_three_fixes_and_three_seconds_when_moving() {
    // n >= 3 and last_t - first_t >= 3000 ms (vf >= 1500 mm/s): fires on the 4th 1 Hz fix.
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    let steps = enter_fixes(&mut rig, 6, 0, BASE_T, 1000, Some(10_000));
    let first = steps.iter().position(|s| s.fired("c-1")).expect("fires");
    assert_eq!(first, 3, "4th fix (index 3)");
    // 2 s spacing: the 3rd fix already spans 4 s
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    let steps = enter_fixes(&mut rig, 6, 0, BASE_T, 2000, Some(10_000));
    assert_eq!(steps.iter().position(|s| s.fired("c-1")), Some(2));
    // 0.5 s spacing: n = 3 reached but the span is only 1 s, the 8th fix spans 3.5 s... first >= 3000 at index 6
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
    let steps = enter_fixes(&mut rig, 10, 0, BASE_T, 500, Some(10_000));
    assert_eq!(steps.iter().position(|s| s.fired("c-1")), Some(6));
}

#[test]
fn enter_dwell_is_10_s_when_slow_or_speed_unknown() {
    for speed in [None, Some(1_000_i64), Some(1_499)] {
        let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
        let steps = enter_fixes(&mut rig, 14, 0, BASE_T, 1000, speed);
        assert_eq!(
            steps.iter().position(|s| s.fired("c-1")),
            Some(10),
            "speed {speed:?}: needs last_t - first_t >= 10000"
        );
    }
}

#[test]
fn a_straddling_fix_resets_the_candidate_count() {
    // inside, inside, STRADDLE, inside, inside: still not entered (count restarted at the straddle)
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 150));
    let plan = [100_i64, 100, 150, 100, 100];
    for (k, e) in plan.iter().enumerate() {
        let f = fix_at(BASE_T + k as i64 * 1000, offset(ORIGIN, e * 1000, 0));
        let s = rig.step(&f);
        assert!(!s.fired("c-1"));
    }
    let det = rig.history.last().unwrap().det("c-1").expect("tracked");
    assert_eq!(det["st"]["s"], "cand");
    assert_eq!(
        det["st"]["n"], 2,
        "count restarted at 1 on the first inside fix after the straddle"
    );
    // Inside with gap <= 10000 ms increments n; a gap of 10001 ms restarts it.
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 150));
    let s = rig.step(&fix_at(BASE_T, offset(ORIGIN, 0, 0)));
    assert_eq!(s.det("c-1").unwrap()["st"]["n"], 1);
    let s = rig.step(&fix_at(BASE_T + 10_001, offset(ORIGIN, 0, 0)));
    assert_eq!(
        s.det("c-1").unwrap()["st"]["n"],
        1,
        "gap 10001 ms restarts the candidate"
    );
    let s = rig.step(&fix_at(BASE_T + 20_001, offset(ORIGIN, 0, 0)));
    assert_eq!(
        s.det("c-1").unwrap()["st"]["n"],
        2,
        "gap exactly 10000 ms still counts"
    );
}

#[test]
fn exit_at_160_m_needs_five_outside_fixes_then_cools_for_30_minutes() {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 150));
    let steps = enter_fixes(&mut rig, 6, 0, BASE_T, 1000, Some(10_000));
    assert!(steps.iter().any(|s| s.fired("c-1")));
    assert_eq!(
        rig.history.last().unwrap().det("c-1").unwrap()["st"]["s"],
        "in"
    );

    // 150..160 m is the hysteresis band: 155 m (straddle) never counts as outside
    let mut t = walk(&mut rig, 0, 155, BASE_T + 6_000);
    for k in 0..10 {
        let s = rig.step(&fix_at(t + k * 1000, offset(ORIGIN, 155_000, 0)));
        assert_eq!(
            s.det("c-1").unwrap()["st"]["s"],
            "in",
            "155 m is not Outside"
        );
    }
    t += 10_000;
    // 165 m is Outside (depth <= -acc_eff). 4 fixes: still `in`; the 5th confirms the exit.
    for k in 0..4 {
        let s = rig.step(&fix_at(t + k * 1000, offset(ORIGIN, 165_000, 0)));
        assert_eq!(s.det("c-1").unwrap()["st"]["s"], "in", "outside fix {k}");
    }
    let last_t = t + 4 * 1000;
    let s = rig.step(&fix_at(last_t, offset(ORIGIN, 165_000, 0)));
    let st = s.det("c-1").unwrap()["st"].clone();
    assert_eq!(st["s"], "cool");
    assert_eq!(
        st["until_ms"],
        last_t + 1_800_000,
        "cool until last_t + 30 min"
    );
    // EXIT is internal: nothing delivered
    assert!(s.fires().is_empty());
}

#[test]
fn exit_by_time_two_outside_fixes_ten_seconds_apart() {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 150));
    enter_fixes(&mut rig, 6, 0, BASE_T, 1000, Some(10_000));
    let t = walk(&mut rig, 0, 120, BASE_T + 6_000); // 120 m: inside
    let s = rig.step(&fix_at(t, offset(ORIGIN, 170_000, 0))); // outside #1 (x_first_t = t)
    assert_eq!(s.det("c-1").unwrap()["st"]["s"], "in");
    // an inside fix zeroes the outside run
    let s = rig.step(&fix_at(t + 1_000, offset(ORIGIN, 120_000, 0)));
    assert_eq!(s.det("c-1").unwrap()["st"]["xn"], 0);
    let t = t + 2_000;
    let s = rig.step(&fix_at(t, offset(ORIGIN, 170_000, 0))); // outside #1 again
    assert_eq!(s.det("c-1").unwrap()["st"]["s"], "in");
    let s = rig.step(&fix_at(t + 9_999, offset(ORIGIN, 170_000, 0))); // xn=2 but 9.999 s
    assert_eq!(
        s.det("c-1").unwrap()["st"]["s"],
        "in",
        "9.999 s since the first outside fix"
    );
    let s = rig.step(&fix_at(t + 10_000, offset(ORIGIN, 170_000, 0))); // xn=3, 10 s
    let st = s.det("c-1").unwrap()["st"].clone();
    assert_eq!(st["s"], "cool", "xn >= 2 and 10 s since x_first_t");
    assert_eq!(st["until_ms"], t + 10_000 + 1_800_000);
}

#[test]
fn cool_ignores_fixes_until_the_deadline_and_the_releasing_fix_is_not_counted() {
    let mut rig = Rig::new().with_signpost(enter_circle("c-1", 150));
    enter_fixes(&mut rig, 6, 0, BASE_T, 1000, Some(10_000));
    let mut t = walk(&mut rig, 0, 170, BASE_T + 6_000);
    for _ in 0..5 {
        rig.step(&fix_at(t, offset(ORIGIN, 170_000, 0)));
        t += 1000;
    }
    let until = rig.history.last().unwrap().det("c-1").unwrap()["st"]["until_ms"]
        .as_i64()
        .unwrap();
    // re-enter long before the deadline: ignored, never fires
    for k in 0..8 {
        let s = rig.step(&fix_at(t + 100_000 + k * 1000, offset(ORIGIN, 0, 0)));
        assert!(s.fix_ok(), "{}", s.fix_reason());
        assert!(!s.fired("c-1"));
        assert_eq!(s.det("c-1").unwrap()["st"]["s"], "cool");
    }
    // the fix at exactly `until` releases the cool-down but does not count towards the candidate
    let s = rig.call(&fix_at(until, offset(ORIGIN, 0, 0)), until);
    assert!(!s.fired("c-1"));
    // then n=1, 2, 3 and the 4th inside fix (span 3000) fires; the app interval (1800 s) has passed
    let mut fired_at = None;
    for k in 1..=6 {
        let s = rig.call(
            &fix_at(until + k * 1000, offset(ORIGIN, 0, 0)),
            until + k * 1000,
        );
        if s.fired("c-1") {
            fired_at = Some(k);
            break;
        }
    }
    assert_eq!(
        fired_at,
        Some(4),
        "released at `until`, first counted fix at +1 s, fires at +4 s"
    );
}
