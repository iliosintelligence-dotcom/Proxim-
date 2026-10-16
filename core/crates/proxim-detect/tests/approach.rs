//! APPROACH detector: time-to-anchor firing, gates, late join, latch and re-arm (spec 6.2, 11).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stderr,
    clippy::cast_possible_truncation
)]

mod common;

use common::{
    Driver, ORIGIN, ROUTE_M, along_of, armed_fields, drive_north, fix_at, pos, route, signpost,
    trigger, with_mode,
};
use proxim_detect::Prepared;
use proxim_path::{PathSummary, fire_distance_mm, path_update};
use proxim_types::{DetState, EventKind, Fix, LatchWhy, Mem, Mode, ReasonCode, Shape};

const ANCHOR_MM: i64 = ROUTE_M * 1000;

fn states(d: &Driver) -> Vec<Option<DetState>> {
    d.steps.iter().map(|s| s.out.state).collect()
}

fn reasons(d: &Driver) -> Vec<Option<ReasonCode>> {
    d.steps.iter().map(|s| s.out.reason).collect()
}

/// Path summary after feeding `fixes` to the filter only (no detector).
fn summary_after(fixes: &[Fix]) -> (Mem, PathSummary) {
    let mut mem = Mem::default();
    let mut last = None;
    for f in fixes {
        let (m, s) = path_update(&mem, f).unwrap();
        mem = m;
        last = Some(s);
    }
    (mem, last.unwrap())
}

/// Steady 1 Hz approach from the start of the route at `v` mm/s; returns the driver.
fn steady(sp: &proxim_types::Signpost, v: u32) -> Driver {
    let mut d = Driver::new(sp);
    let n = (ANCHOR_MM / i64::from(v)) as usize + 5;
    d.feed_all(&drive_north(0, 0, v, n, 0));
    d
}

/// First step whose state carries a confirmation, and the fire step.
fn confirm_and_fire(d: &Driver) -> (usize, usize) {
    let c = d
        .steps
        .iter()
        .position(|s| {
            matches!(
                s.out.state,
                Some(DetState::Armed {
                    confirm_t: Some(_),
                    ..
                })
            )
        })
        .expect("a confirmation");
    let f = d
        .steps
        .iter()
        .position(|s| s.out.event.is_some())
        .expect("a fire");
    (c, f)
}

fn d_rem_of(_d: &Driver, i: usize, v: u32) -> i64 {
    ANCHOR_MM - i64::from(v) * i as i64
}

#[test]
fn spec_fire_distances_for_lead_40() {
    // (v mm/s, mode, expected D_fire mm): 29 m/s -> 1247 m, 15 m/s -> 645 m, cyclist 5 m/s ->
    // 215 m, jam 3 m/s (floored to 4 m/s) -> 172 m. The closed form is exact; the detector's
    // first confirming fix lies in (D_fire - v, D_fire] and it fires on the next fix, one second
    // (one step of v) later, because the rule needs two confirming fixes >= 1 s apart.
    for (v, mode, d_fire) in [
        (29_000, Mode::Vehicle, 1_247_000_i64),
        (15_000, Mode::Vehicle, 645_000),
        (5_000, Mode::Cycle, 215_000),
        (3_000, Mode::Vehicle, 172_000),
    ] {
        let d_lo = if mode == Mode::Vehicle {
            150_000
        } else {
            100_000
        };
        let v_eff = proxim_path::v_eff_for(mode, Some(v));
        assert_eq!(
            fire_distance_mm(v_eff, 40, 3000, d_lo, 3_000_000).unwrap(),
            d_fire
        );

        let d = steady(&with_mode(route(30), mode), v);
        let (c, f) = confirm_and_fire(&d);
        assert_eq!(
            f,
            c + 1,
            "fires on the second confirming fix, 1 s later (v = {v})"
        );
        let at_confirm = d_rem_of(&d, c, v);
        let ev = d.steps[f].out.event.unwrap();
        // Tolerance: 0.1 m for e7 rounding of positions (11 mm) plus the floor in the ETA.
        assert!(
            at_confirm <= d_fire + 100 && at_confirm > d_fire - i64::from(v) - 100,
            "v {v}: confirm at {at_confirm}, D_fire {d_fire}"
        );
        let at_fire = ev.d_rem_mm.unwrap();
        assert!(
            (at_fire - d_rem_of(&d, f, v)).abs() <= 100,
            "v {v}: {at_fire}"
        );
        assert!(
            at_fire <= d_fire - i64::from(v) + 100 && at_fire > d_fire - 2 * i64::from(v) - 100
        );
        // Realized lead (t_hat at the fire fix) is within 2 s below lead + t_comp = 43 s.
        let t_hat = ev.t_hat_ms.unwrap();
        assert!((40_900..=43_000).contains(&t_hat), "v {v}: t_hat {t_hat}");
        assert_eq!(ev.kind, EventKind::Approach);
        assert_eq!(ev.t_ms, d.steps[f].fix.t_ms);
        assert_eq!((ev.path_tier, ev.path_conf_milli), (1, 800));
        assert!(ev.dist_to_shape_mm <= 50, "on the axis of the route");
        // Exactly one event over the whole pass, and the detector ends latched.
        assert_eq!(d.events().len(), 1);
        assert!(matches!(
            d.steps[f].out.state,
            Some(DetState::Latched {
                why: LatchWhy::Fired,
                ..
            })
        ));
    }
}

#[test]
fn arming_happens_at_d_hi_and_never_earlier() {
    let d = steady(&route(30), 20_000);
    let first_armed = states(&d)
        .iter()
        .position(|s| matches!(s, Some(DetState::Armed { .. })))
        .unwrap();
    let rem = d_rem_of(&d, first_armed, 20_000);
    assert!((2_980_000..=3_000_000).contains(&rem), "armed at {rem}");
    // Before that the reason is BEYOND_WINDOW, after the first four fixes: CLOSING needs a
    // baseline fix at least 4 s old (errata E-P12), so fixes 0..=3 at 1 Hz are NOT_CLOSING.
    let r = reasons(&d);
    assert!(r[..4].iter().all(|x| *x == Some(ReasonCode::NotClosing)));
    assert!(
        r[4..first_armed]
            .iter()
            .all(|x| *x == Some(ReasonCode::BeyondWindow))
    );
    // A narrower d_hi moves the arming point and caps the fire distance.
    let mut sp = route(30);
    sp.trigger.d_hi_m = Some(1000);
    let d = steady(&sp, 29_000);
    let (_, f) = confirm_and_fire(&d);
    let rem = d.steps[f].out.event.unwrap().d_rem_mm.unwrap();
    // Armed on the first fix within 1000 m, confirmed on the next, fired on the third: the arming
    // fix is not itself a confirming fix, so the fire is up to three steps inside d_hi.
    assert!(
        rem <= 1_000_000 - 29_000 && rem > 1_000_000 - 3 * 29_000 - 100,
        "{rem}"
    );
}

#[test]
fn crawl_rule_fires_at_d_lo_when_the_time_rule_would_fire_closer() {
    // lead 15 s: D_fire = 4 m/s x 18 s = 72 m < d_lo 150 m: the crawl rule fires at 150 m.
    let mut sp = route(30);
    sp.trigger.lead_s = Some(15);
    let mut d = Driver::new(&sp);
    d.feed_all(&drive_north(0, ANCHOR_MM - 400_000, 4000, 120, 0));
    let (c, f) = confirm_and_fire(&d);
    assert_eq!(f, c + 1);
    let at = d.steps[f].out.event.unwrap().d_rem_mm.unwrap();
    assert!(
        (140_000..=150_000).contains(&at),
        "fired at {at} mm (d_lo = 150 m)"
    );
    assert_eq!(d.events().len(), 1);
}

#[test]
fn d_pass_blocks_arming_too_close_to_the_anchor() {
    // 29 m/s: d_pass = 5 s x 29 = 145 m. Starting at 140 m: BEYOND_WINDOW, no arming, no fire.
    let mut d = Driver::new(&route(30));
    d.feed_all(&drive_north(0, ANCHOR_MM - 140_000, 29_000, 5, 0));
    assert!(d.events().is_empty());
    assert_eq!(states(&d).last().unwrap(), &None);
    assert_eq!(reasons(&d).last().unwrap(), &Some(ReasonCode::BeyondWindow));
    // 60 m floor at low speed: at 4 m/s starting 61 m out arms (and then crawl-fires).
    let mut d = Driver::new(&route(30));
    d.feed_all(&drive_north(0, ANCHOR_MM - 100_000, 4000, 6, 0));
    assert!(
        states(&d)
            .iter()
            .any(|s| matches!(s, Some(DetState::Armed { .. })))
    );
}

#[test]
fn drive_past_on_a_parallel_road() {
    // Buffer 30 m. 100 m and 31 m away: never a candidate. 25 m (inside the buffer): fires, which
    // is the documented false positive for parallel roads within the buffer (spec 11).
    for (e, fires) in [
        (100_000, false),
        (31_000, false),
        (-31_000, false),
        (25_000, true),
        (-25_000, true),
    ] {
        let mut d = Driver::new(&route(30));
        d.feed_all(&drive_north(0, 0, 20_000, 320, e));
        assert_eq!(d.events().len(), usize::from(fires), "offset {e} mm");
        if !fires {
            assert!(states(&d).iter().all(Option::is_none));
            assert!(
                reasons(&d)
                    .iter()
                    .skip(3)
                    .any(|r| *r == Some(ReasonCode::OffCorridor))
            );
        }
    }
}

#[test]
fn opposite_carriageway_and_wrong_direction_never_fire() {
    // Driving south, away from the anchor, on the route itself.
    let fixes: Vec<Fix> = (0..200)
        .map(|i| {
            fix_at(
                i * 1000,
                0,
                (ANCHOR_MM - 20_000 * i) as i32,
                5000,
                Some(20_000),
                Some(1800),
            )
        })
        .collect();
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    assert!(d.events().is_empty());
    assert!(states(&d).iter().all(Option::is_none));
    assert!(reasons(&d).contains(&Some(ReasonCode::BearingMismatch)));
}

#[test]
fn abrupt_u_turn_is_stopped_by_closing_even_while_the_heading_is_stale() {
    // 15 m/s north to 1500 m from the anchor (armed since 3000 m, D_fire = 645 m), then an
    // instantaneous reversal. At 1 Hz the heading filter cannot overturn an exactly opposite
    // measurement quickly (a = 0.2 per step), so the bearing gate still passes for a while; the
    // along-route positions say the vehicle is moving away: NOT_CLOSING returns it to idle.
    let mut fixes = drive_north(0, 0, 15_000, 301, 0); // 4500 m reached at i = 300
    for k in 1..=60_i64 {
        fixes.push(fix_at(
            (300 + k) * 1000,
            0,
            (4_500_000 - 15_000 * k) as i32,
            5000,
            Some(15_000),
            Some(1800),
        ));
    }
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    assert!(d.events().is_empty(), "no fire on the way out");
    let r = reasons(&d);
    let after: Vec<_> = r[301..].iter().collect();
    assert!(after.iter().any(|x| **x == Some(ReasonCode::NotClosing)));
    // It was armed before the turn and idle afterwards.
    assert!(matches!(states(&d)[300], Some(DetState::Armed { .. })));
    assert_eq!(states(&d)[310], None);
    assert_eq!(*states(&d).last().unwrap(), None);
}

#[test]
fn u_turn_then_second_approach_fires_once_at_the_normal_distance() {
    // Drive in to 4500 m, reverse for 40 s, reverse again and approach properly.
    let mut fixes = drive_north(0, 0, 15_000, 301, 0);
    let mut t = 301_i64;
    let mut n = 4_500_000_i64;
    for _ in 0..40 {
        n -= 15_000;
        fixes.push(fix_at(
            t * 1000,
            0,
            n as i32,
            5000,
            Some(15_000),
            Some(1800),
        ));
        t += 1;
    }
    // Turn around gradually (two intermediate courses), then north.
    for c in [2700, 3150, 0, 0] {
        n += 15_000;
        fixes.push(fix_at(t * 1000, 0, n as i32, 5000, Some(15_000), Some(c)));
        t += 1;
    }
    while n < ANCHOR_MM - 5_000 {
        n += 15_000;
        fixes.push(fix_at(t * 1000, 0, n as i32, 5000, Some(15_000), Some(0)));
        t += 1;
    }
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    let ev = d.events();
    assert_eq!(ev.len(), 1, "{ev:?}");
    let rem = ev[0].1.d_rem_mm.unwrap();
    assert!((600_000..=650_000).contains(&rem), "fired at {rem}");
}

#[test]
fn gps_gap_then_late_join_is_dropped_not_fired() {
    // 20 m/s, armed since 3000 m, last good fix 2000 m out. A 90 s gap puts the vehicle ~200 m
    // from the anchor. The first fixes after the gap fail the 3-fixes-in-60-s rule and CLOSING
    // (the pre-gap fixes are older than 60 s, so no baseline is 4 s old until the fifth post-gap
    // fix); when that fix passes every gate, t_hat is about 9 s < L_min 16 s and d_rem > d_lo:
    // latched{late} and TOO_LATE_FOR_LEAD, no event.
    let mut fixes = drive_north(0, 0, 20_000, 201, 0); // n = 4000 m at i = 200 (2000 m left)
    let t0 = 200_000 + 90_000;
    for k in 0..6_i64 {
        fixes.push(fix_at(
            t0 + k * 1000,
            0,
            (5_740_000 + 20_000 * k) as i32,
            5000,
            Some(20_000),
            Some(0),
        ));
    }
    for explain in [true, false] {
        let mut d = Driver::new(&route(30));
        d.explain = explain;
        d.feed_all(&fixes);
        assert!(d.events().is_empty(), "explain {explain}");
        let late = d
            .steps
            .iter()
            .position(|s| s.out.reason == Some(ReasonCode::TooLateForLead))
            .expect("TOO_LATE_FOR_LEAD is terminal and always reported");
        assert_eq!(late, 201 + 4, "needs a post-gap baseline 4 s old");
        assert_eq!(
            d.steps[late].out.state,
            Some(DetState::Latched {
                why: LatchWhy::Late,
                last_t: d.steps[late].fix.t_ms
            })
        );
        // Still latched at the end: the late join is not retried on the next fixes.
        assert!(matches!(
            d.st,
            Some(DetState::Latched {
                why: LatchWhy::Late,
                ..
            })
        ));
        // Before it, the post-gap fixes were rejected for lack of fixes in the last 60 s.
        if explain {
            assert_eq!(d.steps[201].out.state, None);
        }
    }
    // The same gap but the vehicle crawls (4 m/s) and lands within d_lo: crawl rule, no late drop.
    let mut fixes = drive_north(0, 0, 4000, 10, 0);
    let t0 = 9000 + 60_000;
    for k in 0..6_i64 {
        fixes.push(fix_at(
            t0 + k * 1000,
            0,
            (5_860_000 + 4000 * k) as i32,
            5000,
            Some(4000),
            Some(0),
        ));
    }
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    assert!(
        d.steps
            .iter()
            .all(|s| s.out.reason != Some(ReasonCode::TooLateForLead))
    );
}

#[test]
fn l_min_boundary_decides_between_arm_and_late_join() {
    // lead 40 s -> L_min = 16 s. Build five fixes at 20 m/s ending at a given distance from the
    // anchor and look at the arming decision.
    let outcome = |rem_mm: i64| {
        let n_last = ANCHOR_MM - rem_mm;
        // Five fixes: the first four cannot be CLOSING (no 4 s old baseline yet), the fifth arms.
        let fixes: Vec<Fix> = (0..5_i64)
            .map(|k| {
                let n = n_last - 20_000 * (4 - k);
                fix_at(k * 1000, 0, n as i32, 5000, Some(20_000), Some(0))
            })
            .collect();
        let mut d = Driver::new(&route(30));
        d.feed_all(&fixes);
        d.steps.last().unwrap().out
    };
    // 330 m / 20 m/s = 16.5 s: arms. 310 m = 15.5 s: late. (Margins of 0.5 s absorb e7 rounding.)
    assert!(matches!(
        outcome(330_000).state,
        Some(DetState::Armed { .. })
    ));
    let late = outcome(310_000);
    assert!(matches!(
        late.state,
        Some(DetState::Latched {
            why: LatchWhy::Late,
            ..
        })
    ));
    assert_eq!(late.reason, Some(ReasonCode::TooLateForLead));
    // Inside d_lo (150 m) a short ETA is not "late": the crawl rule applies. d_pass for 20 m/s is
    // 100 m: at 145 m the vehicle is still before d_pass, so it arms; at 90 m it is beyond window.
    assert!(matches!(
        outcome(145_000).state,
        Some(DetState::Armed { .. })
    ));
    assert_eq!(outcome(90_000).state, None);
    assert_eq!(outcome(90_000).reason, Some(ReasonCode::BeyondWindow));
}

#[test]
fn confirmation_needs_two_fixes_a_second_apart_and_a_gap_clears_it() {
    // 29 m/s sampled at 2 Hz. The first fix with t_hat <= 43 s sets confirm_t; the next one is
    // only 500 ms later and must not fire; the one after (1000 ms) does.
    let mut d = Driver::new(&route(30));
    let v: i64 = 29_000;
    let fixes: Vec<Fix> = (0..420)
        .map(|k| fix_at(k * 500, 0, (v * k / 2) as i32, 5000, Some(29_000), Some(0)))
        .collect();
    d.feed_all(&fixes);
    let (c, f) = confirm_and_fire(&d);
    assert_eq!(
        f,
        c + 2,
        "confirm at k, 500 ms later still waiting, fires at k + 2"
    );
    assert!(
        matches!(d.steps[c + 1].out.state, Some(DetState::Armed { confirm_t: Some(t), .. }) if t == d.steps[c].fix.t_ms)
    );
    assert!(d.steps[c + 1].out.event.is_none());

    // A gap above 5 s clears the confirmation: the fix after the gap re-confirms instead of firing.
    let mut d = Driver::new(&route(30));
    let mut fixes = drive_north(0, 0, 29_000, 170, 0); // confirm at i = 164, fired at 165: stop before
    fixes.truncate(165);
    d.feed_all(&fixes);
    assert!(
        matches!(
            d.st,
            Some(DetState::Armed {
                confirm_t: Some(164_000),
                ..
            })
        ),
        "{:?}",
        d.st
    );
    // next fix 6 s later (29 m/s x 6 s = 174 m further)
    let o = d.feed(
        fix_at(170_000, 0, 29_000 * 170, 5000, Some(29_000), Some(0)),
        0,
    );
    assert!(o.event.is_none(), "{o:?}");
    let (c, a, l) = armed_fields(o.state).unwrap();
    assert_eq!((c, l), (Some(170_000), 170_000));
    assert!(
        (a.unwrap() - 29_000 * 170).abs() <= 25,
        "a_prev is this fix's along: {a:?}"
    );
    let o = d.feed(
        fix_at(171_000, 0, 29_000 * 171, 5000, Some(29_000), Some(0)),
        0,
    );
    assert!(o.event.is_some());
}

#[test]
fn a_non_confirming_armed_fix_clears_the_confirmation() {
    let fixes = drive_north(0, 0, 29_000, 165, 0); // last fix i = 164 confirms
    let (_, summary) = summary_after(&fixes);
    let prep = Prepared::new(&route(30)).unwrap();
    let f = *fixes.last().unwrap();
    // Armed with a stale confirmation at t = 163 s, but this fix has t_hat > 43 s? It does not:
    // craft the opposite: a fix far from the anchor where nothing holds yet.
    let o = prep
        .step(
            Some(DetState::Armed {
                confirm_t: Some(163_000),
                a_prev_mm: None,
                last_t: 163_000,
            }),
            &summary,
            &f,
            f.t_ms,
            true,
        )
        .unwrap();
    assert!(
        o.event.is_some(),
        "holds and the confirmation is 1 s old: fires"
    );
    // Now a fix at 2500 m (t_hat 86 s): armed, nothing holds, confirmation cleared.
    let fixes2 = drive_north(0, 0, 29_000, 121, 0); // i = 120: 3480 m from the anchor... use 130
    let fixes2: Vec<Fix> = fixes2
        .into_iter()
        .chain(drive_north(121_000, 3_509_000, 29_000, 10, 0))
        .collect();
    let (_, s2) = summary_after(&fixes2);
    let last = *fixes2.last().unwrap();
    let o = prep
        .step(
            Some(DetState::Armed {
                confirm_t: Some(last.t_ms - 1000),
                a_prev_mm: None,
                last_t: last.t_ms - 1000,
            }),
            &s2,
            &last,
            last.t_ms,
            true,
        )
        .unwrap();
    assert!(o.event.is_none());
    let (c, a, l) = armed_fields(o.state).unwrap();
    assert_eq!((c, l), (None, last.t_ms));
    assert!((a.unwrap() - along_of(&last)).abs() <= 25, "{a:?}");
}

#[test]
fn latch_holds_through_the_anchor_and_a_nearby_u_turn_and_rearms_beyond_1_2_d_hi() {
    // Fire on the first pass; keep driving past the end of the route; U-turn and come back within
    // 3600 m (1.2 x d_hi) of the anchor: still latched, no second fire.
    let mut fixes = drive_north(0, 0, 20_000, 300, 0); // fires around i = 270
    let mut t = 300_i64;
    let mut n = 20_000_i64 * 299;
    let push = |fixes: &mut Vec<Fix>, t: &mut i64, n: &mut i64, dn: i64, course: u16| {
        *n += dn;
        fixes.push(fix_at(
            *t * 1000,
            0,
            *n as i32,
            5000,
            Some(20_000),
            Some(course),
        ));
        *t += 1;
    };
    for _ in 0..20 {
        push(&mut fixes, &mut t, &mut n, 20_000, 0);
    } // beyond the anchor
    for _ in 0..80 {
        push(&mut fixes, &mut t, &mut n, -20_000, 1800);
    } // back, to ~4300 m
    for _ in 0..40 {
        push(&mut fixes, &mut t, &mut n, 20_000, 0);
    } // forward again
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    assert_eq!(d.events().len(), 1, "one fire per pass");
    assert!(matches!(
        d.st,
        Some(DetState::Latched {
            why: LatchWhy::Fired,
            ..
        })
    ));
    // Now leave to more than 1.2 x 3000 m from the anchor: re-armed (idle), then a second pass
    // fires again.
    let mut more = fixes.clone();
    for _ in 0..170 {
        push(&mut more, &mut t, &mut n, -20_000, 1800);
    } // down to ~2200 m: 3800 m from the anchor
    let idx_far = more.len();
    for _ in 0..260 {
        push(&mut more, &mut t, &mut n, 20_000, 0);
    } // approach again
    let mut d = Driver::new(&route(30));
    d.feed_all(&more);
    let latched_until_far = states(&d)[fixes.len()..idx_far]
        .iter()
        .position(|s| s.is_none());
    assert!(
        latched_until_far.is_some(),
        "released once the fix is beyond 1.2 x d_hi"
    );
    assert_eq!(
        d.events().len(),
        2,
        "second pass fires again: {:?}",
        d.events().iter().map(|e| e.0).collect::<Vec<_>>()
    );
}

#[test]
fn passing_the_anchor_alone_does_not_rearm() {
    // After the fire the vehicle passes the end of the route (anchor) and keeps going: latched.
    let mut fixes = drive_north(0, 0, 20_000, 300, 0);
    for k in 0..60_i64 {
        fixes.push(fix_at(
            (300 + k) * 1000,
            0,
            (20_000 * (299 + k + 1)) as i32,
            5000,
            Some(20_000),
            Some(0),
        ));
    }
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    assert_eq!(d.events().len(), 1);
    assert!(matches!(
        d.st,
        Some(DetState::Latched {
            why: LatchWhy::Fired,
            ..
        })
    ));
}

#[test]
fn stale_fixes_and_dead_reckoning() {
    // A fix whose age is 10 s evaluates; 10.001 s is FIX_STALE and returns to idle.
    let fixes = drive_north(0, 0, 20_000, 300, 0);
    let mut d = Driver::new(&route(30));
    for f in &fixes {
        d.feed(*f, 10_001);
    }
    assert!(d.events().is_empty());
    assert!(states(&d).iter().all(Option::is_none));
    assert!(reasons(&d).iter().all(|r| *r == Some(ReasonCode::FixStale)));
    let mut d = Driver::new(&route(30));
    for f in &fixes {
        d.feed(*f, 10_000);
    }
    assert_eq!(d.events().len(), 1);
    // Dead reckoning: 5 s old fixes see d_rem reduced by v x 5 s = 100 m, so the fire comes
    // 100 m earlier along the road than with fresh fixes.
    let mut fresh = Driver::new(&route(30));
    let mut aged = Driver::new(&route(30));
    for f in &fixes {
        fresh.feed(*f, 0);
        aged.feed(*f, 5000);
    }
    let (_, ef) = fresh.events()[0];
    let (ia, ea) = aged.events()[0];
    let pos_fresh = ANCHOR_MM - ef.d_rem_mm.unwrap();
    let pos_aged = ANCHOR_MM - ea.d_rem_mm.unwrap() - 100_000; // position of the aged fix itself
    assert!(pos_aged < pos_fresh, "the aged run fires on an earlier fix");
    let own = ANCHOR_MM - 20_000 * ia as i64;
    assert!(
        (ea.d_rem_mm.unwrap() - (own - 100_000)).abs() <= 100,
        "d_rem is dead reckoned by v_eff x age"
    );
    // A fix a little in the future (now < t) is age 0, not an error.
    let mut d = Driver::new(&route(30));
    for f in &fixes {
        d.feed(*f, -3000);
    }
    assert_eq!(d.events().len(), 1);
}

#[test]
fn accuracy_gate_and_confidence_ladder() {
    // 29 m/s so that CLOSING holds even at 65 m accuracy (net advance over the 4 s+ baseline far above tol 32.5 m).
    let run = |acc: u32| {
        let fixes: Vec<Fix> = (0..210)
            .map(|i| fix_at(i * 1000, 0, (29_000 * i) as i32, acc, Some(29_000), Some(0)))
            .collect();
        let mut d = Driver::new(&route(30));
        d.feed_all(&fixes);
        d
    };
    // acc_eff 65 m passes the gate: confidence 600 + 100 (>= 5 fixes) + 100 (tight heading) - 200.
    let d = run(65_000);
    assert_eq!(d.events().len(), 1);
    assert_eq!(d.events()[0].1.path_conf_milli, 600);
    // 65.001 m is too coarse: never armed.
    let d = run(65_001);
    assert!(d.events().is_empty());
    assert!(
        reasons(&d)
            .iter()
            .skip(3)
            .all(|r| *r == Some(ReasonCode::FixAccuracyTooCoarse))
    );
    // 40 m: -100 -> 700; 5 m: 800.
    assert_eq!(run(40_000).events()[0].1.path_conf_milli, 700);
    assert_eq!(run(5_000).events()[0].1.path_conf_milli, 800);
    assert_eq!(run(10_000).events()[0].1.path_conf_milli, 800);
    assert_eq!(run(30_000).events()[0].1.path_conf_milli, 800);
    assert_eq!(run(30_001).events()[0].1.path_conf_milli, 700);
    assert_eq!(run(50_000).events()[0].1.path_conf_milli, 700);
    assert_eq!(run(50_001).events()[0].1.path_conf_milli, 600);
}

#[test]
fn min_path_floors_for_approach() {
    let run = |tier: Option<u8>, conf: Option<u16>, acc: u32| {
        let mut sp = route(30);
        sp.min_path_tier = tier;
        sp.min_path_conf_milli = conf;
        let fixes: Vec<Fix> = (0..210)
            .map(|i| fix_at(i * 1000, 0, (29_000 * i) as i32, acc, Some(29_000), Some(0)))
            .collect();
        let mut d = Driver::new(&sp);
        d.feed_all(&fixes);
        d
    };
    assert_eq!(run(Some(1), Some(800), 5000).events().len(), 1);
    assert_eq!(
        run(Some(0), Some(801), 5000).events().len(),
        0,
        "conf 800 < 801"
    );
    assert!(
        reasons(&run(Some(0), Some(801), 5000))
            .iter()
            .skip(3)
            .any(|r| *r == Some(ReasonCode::PathConfidenceLow))
    );
    let d = run(Some(2), None, 5000);
    assert_eq!(
        d.events().len(),
        0,
        "P2 is reserved: a floor of 2 never passes"
    );
    assert!(
        reasons(&d)
            .iter()
            .skip(3)
            .any(|r| *r == Some(ReasonCode::PathTierLow))
    );
    // The default floor 500 rejects the 400 that 65 m accuracy gives without bonuses: use a
    // slow-sampled track so there are only 3 fixes in 60 s (no five-fix bonus) and a heading
    // 30 degrees off (no tight bonus): 600 - 200 = 400 < 500.
    let d = run(None, Some(650), 65_000);
    assert_eq!(d.events().len(), 0);
    assert!(reasons(&d).contains(&Some(ReasonCode::PathConfidenceLow)));
}

#[test]
fn bearing_tolerance_edges() {
    let run = |course: u16, tol: Option<u16>| {
        let mut sp = route(30);
        sp.trigger.bearing_tol_ddeg = tol;
        let fixes: Vec<Fix> = (0..210)
            .map(|i| {
                fix_at(
                    i * 1000,
                    0,
                    (29_000 * i) as i32,
                    5000,
                    Some(29_000),
                    Some(course),
                )
            })
            .collect();
        let mut d = Driver::new(&sp);
        d.feed_all(&fixes);
        d
    };
    assert_eq!(
        run(300, None).events().len(),
        1,
        "30.0 degrees off with the default 30.0 tolerance"
    );
    let d = run(301, None);
    assert_eq!(d.events().len(), 0, "30.1 degrees off");
    // Every fix on the route fails the bearing gate (the last ones are past the end: off corridor).
    assert!(reasons(&d).iter().all(|r| matches!(
        r,
        Some(ReasonCode::BearingMismatch | ReasonCode::OffCorridor)
    )));
    assert!(reasons(&d).contains(&Some(ReasonCode::BearingMismatch)));
    assert_eq!(
        run(3300, None).events().len(),
        1,
        "30 degrees to the other side"
    );
    assert_eq!(run(3299, None).events().len(), 0);
    assert_eq!(run(450, Some(450)).events().len(), 1);
    assert_eq!(run(451, Some(450)).events().len(), 0);
    assert_eq!(run(150, Some(150)).events().len(), 1);
    assert_eq!(run(151, Some(150)).events().len(), 0);
    // With no OS course the heading comes from positions: still works at 29 m/s.
    let fixes: Vec<Fix> = (0..210)
        .map(|i| fix_at(i * 1000, 0, (29_000 * i) as i32, 5000, Some(29_000), None))
        .collect();
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    assert_eq!(d.events().len(), 1);
    // No heading at all (no course, too slow to move 30 m in 3 s... but fast enough to close):
    // heading invalid fails the bearing gate.
    let fixes: Vec<Fix> = (0..400)
        .map(|i| fix_at(i * 1000, 0, (9_000 * i) as i32, 5000, Some(9_000), None))
        .collect();
    let mut d = Driver::new(&route(30));
    d.feed_all(&fixes);
    assert_eq!(
        d.events().len(),
        0,
        "9 m/s x 3 s = 27 m < 30 m: no displacement heading"
    );
    assert!(reasons(&d).contains(&Some(ReasonCode::BearingMismatch)));
}

#[test]
fn speed_band_with_hysteresis() {
    // Free-flowing band: >= 8.3 m/s. Idle below it; once armed, 7.3 m/s still holds.
    let mut sp = route(30);
    sp.trigger.speed_min_mmps = Some(8300);
    let mut d = Driver::new(&sp);
    d.feed_all(&drive_north(0, 0, 8000, 400, 0));
    assert!(d.events().is_empty());
    assert!(
        reasons(&d)
            .iter()
            .skip(3)
            .any(|r| *r == Some(ReasonCode::SpeedOutOfBand))
    );
    // Fast to arm (10 m/s), slow to 7.5 m/s: still armed (filtered speed above 7.3); then 7.0 m/s:
    // the filtered speed falls below 7.3 and the detector returns to idle with SPEED_OUT_OF_BAND.
    let mut d = Driver::new(&sp);
    let mut t = 0_i64;
    let mut n = 0_i64;
    let mut feed = |d: &mut Driver, v: u32, count: usize| {
        for _ in 0..count {
            n += i64::from(v);
            t += 1000;
            d.feed(fix_at(t, 0, n as i32, 5000, Some(v), Some(0)), 0);
        }
    };
    feed(&mut d, 10_000, 320); // n = 3200 m: armed since 3000 m; D_fire = 430 m
    assert!(matches!(d.st, Some(DetState::Armed { .. })), "{:?}", d.st);
    feed(&mut d, 7_500, 10);
    assert!(
        matches!(d.st, Some(DetState::Armed { .. })),
        "{:?} vf {:?}",
        d.st,
        d.steps.last().unwrap().summary.vf_mmps
    );
    feed(&mut d, 7_000, 60);
    assert_eq!(d.st, None);
    assert_eq!(
        d.steps.last().unwrap().out.reason,
        Some(ReasonCode::SpeedOutOfBand)
    );
    assert!(d.events().is_empty());
}

#[test]
fn speed_band_hysteresis_lower_edge_is_exact() {
    // Craft: armed state, one fix at constant filtered speed v. Band [8300, inf), hysteresis 1000.
    let mut sp = route(30);
    sp.trigger.speed_min_mmps = Some(8300);
    let prep = Prepared::new(&sp).unwrap();
    let mk = |v: u32| {
        let fixes = drive_north(0, 5_000_000, v, 8, 0);
        let (_, s) = summary_after(&fixes);
        (fixes, s)
    };
    for (v, armed_ok, idle_ok) in [
        (7_300_u32, true, false),
        (7_299, false, false),
        (8_300, true, true),
        (8_299, true, false),
    ] {
        let (fixes, s) = mk(v);
        let f = *fixes.last().unwrap();
        let armed = Some(DetState::Armed {
            confirm_t: None,
            a_prev_mm: None,
            last_t: f.t_ms - 1000,
        });
        let a = prep.step(armed, &s, &f, f.t_ms, true).unwrap();
        let i = prep.step(None, &s, &f, f.t_ms, true).unwrap();
        assert_eq!(
            matches!(a.state, Some(DetState::Armed { .. })),
            armed_ok,
            "armed v {v}: {:?}",
            a.reason
        );
        assert_eq!(
            matches!(i.state, Some(DetState::Armed { .. })),
            idle_ok,
            "idle v {v}: {:?}",
            i.reason
        );
        if !armed_ok {
            assert_eq!(a.reason, Some(ReasonCode::SpeedOutOfBand));
        }
    }
}

#[test]
fn foot_mode_fires_at_d_lo_with_capped_confidence() {
    // 1.4 m/s sampled every 4 s. Foot: v_eff 1.4 m/s, D_fire = 60 m < d_lo = max(100 m, 3 x 10 m)
    // -> fires at about 100 m.
    let sp = with_mode(route(30), Mode::Foot);
    let fixes: Vec<Fix> = (0..1300)
        .map(|i| {
            fix_at(
                i * 4000,
                0,
                (1400 * 4 * i) as i32,
                5000,
                Some(1400),
                Some(0),
            )
        })
        .collect();
    let mut d = Driver::new(&sp);
    d.feed_all(&fixes);
    let ev = d.events();
    assert_eq!(ev.len(), 1);
    let e = ev[0].1;
    assert!(
        (90_000..=100_000).contains(&e.d_rem_mm.unwrap()),
        "{:?}",
        e.d_rem_mm
    );
    assert_eq!(e.path_conf_milli, 600, "foot is capped at 600");
    // At 1 Hz the walker fires too (errata E-P12): the time-span CLOSING rule sees the 15 s of
    // walking in the trace, not 2.8 m over three fixes.
    let fixes: Vec<Fix> = (0..4500)
        .map(|i| fix_at(i * 1000, 0, (1400 * i) as i32, 5000, Some(1400), Some(0)))
        .collect();
    let mut d = Driver::new(&sp);
    d.feed_all(&fixes);
    let ev = d.events();
    assert_eq!(
        ev.len(),
        1,
        "{:?}",
        reasons(&d).iter().rev().take(5).collect::<Vec<_>>()
    );
    let e = ev[0].1;
    assert!(
        (90_000..=100_000).contains(&e.d_rem_mm.unwrap()),
        "{:?}",
        e.d_rem_mm
    );
    assert_eq!((e.path_tier, e.path_conf_milli), (1, 600));
    // Foot band 0.5..2.5 m/s: 3 m/s is out of band.
    let fixes: Vec<Fix> = (0..1500)
        .map(|i| {
            fix_at(
                i * 4000,
                0,
                (3000 * 4 * i) as i32,
                5000,
                Some(3000),
                Some(0),
            )
        })
        .collect();
    let mut d = Driver::new(&sp);
    d.feed_all(&fixes);
    assert!(d.events().is_empty());
    assert!(reasons(&d).contains(&Some(ReasonCode::SpeedOutOfBand)));
}

#[test]
fn out_of_order_and_foreign_states() {
    let fixes = drive_north(0, 0, 29_000, 166, 0);
    let (_, s) = summary_after(&fixes);
    let f = *fixes.last().unwrap();
    let prep = Prepared::new(&route(30)).unwrap();
    // Armed or latched with last_t >= fix.t: unchanged, FIX_OUT_OF_ORDER.
    for st in [
        DetState::Armed {
            confirm_t: Some(1),
            a_prev_mm: Some(7),
            last_t: f.t_ms,
        },
        DetState::Latched {
            why: LatchWhy::Fired,
            last_t: f.t_ms + 5,
        },
    ] {
        let o = prep.step(Some(st), &s, &f, f.t_ms, true).unwrap();
        assert_eq!(
            (o.state, o.event, o.reason),
            (Some(st), None, Some(ReasonCode::FixOutOfOrder))
        );
    }
    // Cand/In/Cool handed to an APPROACH detector restart from idle (no panic, no event).
    for st in [
        DetState::Cand {
            n: 9,
            first_t: 0,
            last_t: 0,
        },
        DetState::In {
            xn: 0,
            x_first_t: None,
            last_t: 0,
        },
        DetState::Cool { until_ms: i64::MAX },
    ] {
        let o = prep.step(Some(st), &s, &f, f.t_ms, false).unwrap();
        assert!(o.event.is_none());
        assert!(matches!(o.state, None | Some(DetState::Armed { .. })));
    }
}

#[test]
fn reach_returns_armed_to_idle() {
    // reach 3500 m from the origin: armed from n = 3000 m, idle again once n > 3500 m.
    let fixes = drive_north(0, 0, 20_000, 220, 0);
    let mut sp = route(30);
    sp.trigger.reach_m = Some(3500);
    let mut d = Driver::new(&sp);
    d.feed_all(&fixes);
    assert!(d.events().is_empty());
    let st = states(&d);
    assert!(st.iter().any(|s| matches!(s, Some(DetState::Armed { .. }))));
    let last = d.steps.last().unwrap();
    assert_eq!(last.out.state, None);
    assert_eq!(last.out.reason, Some(ReasonCode::OutOfRange));
    let first_out = reasons(&d)
        .iter()
        .position(|r| *r == Some(ReasonCode::OutOfRange))
        .unwrap();
    assert!(
        matches!(st[first_out - 1], Some(DetState::Armed { .. })),
        "armed just before leaving reach"
    );
    assert_eq!(st[first_out], None);
    // Default reach = extent + d_hi + 100 m: a fix 9.5 km from the origin is out; the route's own
    // end (6 km, extent 6 km + 3.1 km) is in.
    let prep = Prepared::new(&route(30)).unwrap();
    assert!((prep.reach_mm() - 9_100_000).abs() <= 50);
    assert!(!prep.beyond_reach(pos(0, 9_000_000)));
    assert!(prep.beyond_reach(pos(0, 9_200_000)));
    assert!(prep.beyond_reach(proxim_types::Position {
        lat_e7: 900_000_000,
        lon_e7: 0
    }));
}

#[test]
fn invalid_route_triggers_are_dropped() {
    let base = || route(30);
    let check = |f: &dyn Fn(&mut proxim_types::Signpost), ok: bool| {
        let mut sp = base();
        f(&mut sp);
        assert_eq!(Prepared::new(&sp).is_ok(), ok, "{:?}", sp.trigger);
        if !ok {
            assert_eq!(Prepared::new(&sp).unwrap_err(), ReasonCode::TriggerInvalid);
        }
    };
    check(&|_| {}, true);
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN, pos(0, 1_000_000)],
                buffer_m: 29,
                anchor_along_m: None,
            }
        },
        false,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN, pos(0, 1_000_000)],
                buffer_m: 30,
                anchor_along_m: None,
            }
        },
        true,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN, pos(0, 1_000_000)],
                buffer_m: 500,
                anchor_along_m: None,
            }
        },
        true,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN, pos(0, 1_000_000)],
                buffer_m: 501,
                anchor_along_m: None,
            }
        },
        false,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN],
                buffer_m: 30,
                anchor_along_m: None,
            }
        },
        false,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN, ORIGIN],
                buffer_m: 30,
                anchor_along_m: None,
            }
        },
        false,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN; 1025],
                buffer_m: 30,
                anchor_along_m: None,
            }
        },
        false,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN, pos(0, 1_000_000)],
                buffer_m: 30,
                anchor_along_m: Some(999),
            }
        },
        true,
    );
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![ORIGIN, pos(0, 1_000_000)],
                buffer_m: 30,
                anchor_along_m: Some(1001),
            }
        },
        false,
    );
    check(&|s| s.trigger.lead_s = Some(14), false);
    check(&|s| s.trigger.lead_s = Some(15), true);
    check(&|s| s.trigger.lead_s = Some(120), true);
    check(&|s| s.trigger.lead_s = Some(121), false);
    check(&|s| s.trigger.d_hi_m = Some(5000), true);
    check(&|s| s.trigger.d_hi_m = Some(5001), false);
    check(&|s| s.trigger.d_hi_m = Some(0), false);
    check(
        &|s| {
            s.trigger.d_hi_m = Some(100);
        },
        false,
    ); // d_lo 150 m > d_hi 100 m
    check(
        &|s| {
            s.trigger.d_hi_m = Some(200);
            s.trigger.d_lo_m = Some(200);
        },
        true,
    );
    check(
        &|s| {
            s.trigger.d_lo_m = Some(3001);
        },
        false,
    );
    check(&|s| s.trigger.bearing_tol_ddeg = Some(149), false);
    check(&|s| s.trigger.bearing_tol_ddeg = Some(150), true);
    check(&|s| s.trigger.bearing_tol_ddeg = Some(450), true);
    check(&|s| s.trigger.bearing_tol_ddeg = Some(451), false);
    check(
        &|s| {
            s.trigger.speed_min_mmps = Some(9000);
            s.trigger.speed_max_mmps = Some(8000);
        },
        false,
    );
    check(
        &|s| {
            s.trigger.mode = Some(Mode::Foot);
            s.trigger.speed_min_mmps = Some(3000);
        },
        false,
    );
    // A route whose vertex is beyond the 150 km frame.
    check(
        &|s| {
            s.trigger.shape = Shape::Route {
                vertices: vec![
                    ORIGIN,
                    proxim_types::Position {
                        lat_e7: ORIGIN.lat_e7 + 20_000_000,
                        lon_e7: ORIGIN.lon_e7,
                    },
                ],
                buffer_m: 30,
                anchor_along_m: None,
            }
        },
        false,
    );
    // Event kind mismatch with a circle is covered in enter.rs; here approach without a route:
    let sp = signpost(
        EventKind::Approach,
        trigger(Shape::Circle {
            center: ORIGIN,
            radius_m: 200,
        }),
    );
    assert_eq!(Prepared::new(&sp).unwrap_err(), ReasonCode::TriggerInvalid);
}

#[test]
fn anchor_along_moves_the_fire_point() {
    // Anchor 4 km along the 6 km route: the fire point moves back by 2 km (D_fire is relative to
    // the anchor); fixes beyond the anchor are not needed.
    let mut sp = route(30);
    if let Shape::Route { anchor_along_m, .. } = &mut sp.trigger.shape {
        *anchor_along_m = Some(4000);
    }
    let d = steady(&sp, 29_000);
    let (_, f) = confirm_and_fire(&d);
    let along = 29_000 * f as i64;
    let rem = d.steps[f].out.event.unwrap().d_rem_mm.unwrap();
    assert!((rem - (4_000_000 - along)).abs() <= 100, "{rem}");
    assert!((1_190_000..=1_247_000).contains(&rem));
}

#[test]
fn latch_release_distance_is_1_2_d_hi_euclidean() {
    // Anchor at n = 6000 m, d_hi = 3000 m: a latched detector releases only beyond 3600 m from the
    // anchor point, measured in a straight line (not along the route).
    let prep = Prepared::new(&route(30)).unwrap();
    let (_, summary) = summary_after(&drive_north(0, 0, 20_000, 3, 0));
    let step = |e: i32, n: i32| {
        let f = fix_at(10_000, e, n, 5000, Some(20_000), Some(0));
        let st = DetState::Latched {
            why: LatchWhy::Fired,
            last_t: 9000,
        };
        prep.step(Some(st), &summary, &f, f.t_ms, true)
            .unwrap()
            .state
    };
    let latched = Some(DetState::Latched {
        why: LatchWhy::Fired,
        last_t: 10_000,
    });
    assert_eq!(step(0, 6_000_000 - 3_599_000), latched, "3599 m: held");
    assert_eq!(step(0, 6_000_000 - 3_601_000), None, "3601 m: released");
    assert_eq!(
        step(0, 6_000_000 - 3_000_000),
        latched,
        "exactly d_hi: held"
    );
    // Sideways: 3500 m east of the anchor is within 3600 m; 3700 m is not.
    assert_eq!(step(3_500_000, 6_000_000), latched);
    assert_eq!(step(3_700_000, 6_000_000), None);
    // Beyond the anchor, 3500 m past the end of the route: held.
    assert_eq!(step(0, 6_000_000 + 3_500_000), latched);
    // A late latch releases the same way.
    let f = fix_at(
        10_000,
        0,
        6_000_000 - 3_700_000,
        5000,
        Some(20_000),
        Some(0),
    );
    let st = DetState::Latched {
        why: LatchWhy::Late,
        last_t: 9000,
    };
    assert_eq!(
        prep.step(Some(st), &summary, &f, f.t_ms, true)
            .unwrap()
            .state,
        None
    );
}

#[test]
fn d_pass_floor_is_60_m_at_low_speed() {
    // 5 m/s: d_pass = max(60 m, 25 m) = 60 m. Starting 74 m out the detector arms on the third
    // fix (64 m) and is beyond the window once d_rem falls under 60 m: it never fires.
    let mut d = Driver::new(&route(30));
    d.feed_all(&drive_north(0, ANCHOR_MM - 74_000, 5000, 12, 0));
    assert!(d.events().is_empty(), "{:?}", d.events());
    assert!(reasons(&d).contains(&Some(ReasonCode::BeyondWindow)));
    assert_eq!(*states(&d).last().unwrap(), None);
}

#[test]
fn off_corridor_looks_at_all_three_recent_fixes() {
    // Two earlier fixes 40 m off the axis (buffer 30 m), the current one on it, heading north.
    let fixes = [
        fix_at(0, 40_000, 3_000_000, 5000, Some(20_000), Some(0)),
        fix_at(1000, 40_000, 3_020_000, 5000, Some(20_000), Some(0)),
        fix_at(2000, 20_000, 3_040_000, 5000, Some(20_000), Some(0)),
    ];
    let mut d = Driver::new(&route(30));
    let outs = d.feed_all(&fixes);
    assert_eq!(outs[2].reason, Some(ReasonCode::OffCorridor));
    assert_eq!(outs[2].state, None);
}

#[test]
fn sender_band_cannot_go_below_the_mode_minimum() {
    // A sender band minimum of 1 m/s is raised to the vehicle minimum 2.5 m/s: a 2 m/s vehicle
    // is out of band (not merely "tier low").
    let mut sp = route(30);
    sp.trigger.speed_min_mmps = Some(1000);
    let mut d = Driver::new(&sp);
    d.feed_all(&drive_north(0, 5_000_000, 2000, 10, 0));
    assert!(
        reasons(&d)
            .iter()
            .skip(3)
            .all(|r| *r == Some(ReasonCode::SpeedOutOfBand))
    );
}

#[test]
fn armed_window_keeps_a_fix_on_its_own_leg_of_a_hairpin() {
    // Hairpin: 3 km north, 40 m east, 3 km back south; anchor at the end of the first leg;
    // buffer 50 m. The last fix is 22 m east, nearer to the return leg (18 m) than to the
    // outbound leg (22 m). Idle projects it onto the return leg and drops on the bearing; armed
    // with the exact previous along (errata E-P13) the window keeps it on the outbound leg.
    let v = vec![
        ORIGIN,
        pos(0, 3_000_000),
        pos(40_000, 3_000_000),
        pos(40_000, 0),
    ];
    let sp = signpost(
        EventKind::Approach,
        trigger(Shape::Route {
            vertices: v,
            buffer_m: 50,
            anchor_along_m: Some(3000),
        }),
    );
    let prep = Prepared::new(&sp).unwrap();
    let fixes: Vec<Fix> = (0..5_i64)
        .map(|k| {
            let e = if k == 4 { 22_000 } else { 10_000 };
            fix_at(
                k * 1000,
                e,
                (1_000_000 + 20_000 * k) as i32,
                5000,
                Some(20_000),
                Some(0),
            )
        })
        .collect();
    let (_, s) = summary_after(&fixes);
    let f = fixes[4];
    let step = |a_prev_mm| {
        prep.step(
            Some(DetState::Armed {
                confirm_t: None,
                a_prev_mm,
                last_t: 3000,
            }),
            &s,
            &f,
            f.t_ms,
            true,
        )
        .unwrap()
    };
    // The previous fix (k = 3) was at along 1_060_000 on the outbound leg.
    let exact = step(Some(1_060_000));
    let (c, a, l) = armed_fields(exact.state).unwrap_or_else(|| panic!("{:?}", exact.reason));
    assert_eq!((c, l), (None, 4000));
    assert!(
        (a.unwrap() - 1_080_000).abs() <= 25,
        "a_prev follows the fix: {a:?}"
    );
    // The window is authoritative: with an a_prev on the return leg the same fix is held there.
    let wrong = step(Some(4_000_000));
    assert_eq!(wrong.state, None);
    assert_eq!(wrong.reason, Some(ReasonCode::BearingMismatch));
    // A legacy state without a_prev, and the idle detector, search the whole route.
    let legacy = step(None);
    assert_eq!(legacy.state, None);
    assert_eq!(legacy.reason, Some(ReasonCode::BearingMismatch));
    let idle = prep.step(None, &s, &f, f.t_ms, true).unwrap();
    assert_eq!(idle.state, None);
    assert_eq!(idle.reason, Some(ReasonCode::BearingMismatch));
}

#[test]
fn foot_d_lo_grows_with_three_times_the_accuracy() {
    // 2.4 m/s foot, a fix every 5 s, accuracy 40 m: d_lo = max(100 m, 120 m) = 120 m, so the
    // crawl rule (and the clamp of D_fire = 103 m up to d_lo) puts the fire at 96..108 m, not at
    // the 79..91 m that d_lo = 100 m would give.
    let sp = with_mode(route(30), Mode::Foot);
    let fixes: Vec<Fix> = (0..600_i64)
        .map(|i| {
            fix_at(
                i * 5000,
                0,
                (12_000 * i) as i32,
                40_000,
                Some(2400),
                Some(0),
            )
        })
        .collect();
    let mut d = Driver::new(&sp);
    d.feed_all(&fixes);
    let ev = d.events();
    assert_eq!(ev.len(), 1);
    let rem = ev[0].1.d_rem_mm.unwrap();
    assert!((96_000..=108_100).contains(&rem), "{rem}");
    assert_eq!(ev[0].1.path_conf_milli, 600);
}
