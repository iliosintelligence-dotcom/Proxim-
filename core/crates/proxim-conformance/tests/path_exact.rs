//! Spec 5 "exact" filters, checked against an independent re-implementation of the stated
//! arithmetic: speed EWMA (5.1), heading blend (5.2), dead reckoning and time to anchor (5.4).
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
use proxim_geo::{Polyline, frame_for, normalize_q30, project_to_polyline, rsr, unit_from_course};
use proxim_types::{Enu, Position};
use serde_json::json;

fn alpha(dt: i64, tau: i64) -> i128 {
    i128::from(dt) * 65_536 / i128::from(tau + dt)
}

#[test]
fn speed_filter_is_the_rational_ewma_of_5_1() {
    let mut rng = Rng::new(2026);
    let mut rig = Rig::new();
    let mut vf: Option<i64> = None;
    let mut last_t: Option<i64> = None;
    let mut t = BASE_T;
    for i in 0..300 {
        let dt_step = rng.range_i64(500, 5000); // keep <= 5000 ms: the reseed path is separate
        t += dt_step;
        let v = rng.range_i64(0, 40_000);
        let mut f = fix_at(t, ORIGIN);
        f["speed_mmps"] = json!(v);
        unset(&mut f, "course_ddeg");
        let s = rig.step(&f);
        assert!(s.fix_ok(), "{}", s.fix_reason());
        // reference
        let dt = last_t.map_or(0, |l| (t - l).clamp(0, 60_000));
        vf = Some(match vf {
            None => v,
            Some(prev) if dt > 5000 => {
                let _ = prev;
                v
            }
            Some(prev) => {
                let a = alpha(dt, 8000);
                prev + rsr((i128::from(v) - i128::from(prev)) * a, 16).unwrap() as i64
            }
        });
        last_t = Some(t);
        assert_eq!(s.mem()["vf_mmps"], vf.unwrap(), "step {i}: dt {dt} v {v}");
        assert_eq!(
            s.output["path"]["speed_mmps"],
            vf.unwrap(),
            "visible speed at step {i}"
        );
    }
}

#[test]
fn speed_accuracy_above_5000_mmps_discards_the_doppler_value() {
    // stationary device claiming 20 m/s: with speed_acc 5000 it is believed, with 5001 it is not
    // (the position-derived speed is then 0)
    for (acc, believed) in [(5000, true), (5001, false)] {
        let mut rig = Rig::new();
        let mut last = None;
        for k in 0..12 {
            let mut f = fix_at(BASE_T + k * 1000, ORIGIN);
            f["speed_mmps"] = json!(20_000);
            f["speed_acc_mmps"] = json!(acc);
            last = Some(rig.step(&f));
        }
        let vf = last.unwrap().mem()["vf_mmps"].as_i64();
        if believed {
            assert_eq!(vf, Some(20_000));
        } else {
            assert!(
                vf.is_none_or(|v| v < 2000),
                "speed_acc {acc}: vf {vf:?} should come from displacement (0)"
            );
        }
    }
}

#[test]
fn speed_from_displacement_uses_the_trace_entry_aged_3_to_10_s() {
    let mut rig = Rig::new();
    let mut vf_seen = vec![];
    for k in 0..15_i64 {
        let mut f = fix_at(BASE_T + k * 1000, offset(ORIGIN, 0, k * 10_000));
        unset(&mut f, "speed_mmps");
        unset(&mut f, "course_ddeg");
        let s = rig.step(&f);
        vf_seen.push((k, s.output["path"]["speed_mmps"].as_i64()));
    }
    for (k, v) in vf_seen {
        if k < 3 {
            assert!(
                v.is_none(),
                "no measurement before a 3 s old fix exists (k={k}): {v:?}"
            );
        } else {
            let v = v.unwrap_or_else(|| panic!("k={k}: speed should be known"));
            assert!((9_900..=10_100).contains(&v), "k={k}: {v}");
        }
    }
}

#[test]
fn speed_is_hidden_after_10_s_without_a_measurement_but_vf_is_unchanged() {
    let mut rig = Rig::new();
    for k in 0..5 {
        rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
    }
    let vf = rig.history.last().unwrap().mem()["vf_mmps"].clone();
    assert_eq!(vf, 10_000);
    // a fix with no speed 12 s later: no trace entry aged 3..10 s -> no measurement
    let mut f = fix_at(BASE_T + 16_000, ORIGIN);
    unset(&mut f, "speed_mmps");
    unset(&mut f, "course_ddeg");
    let s = rig.step(&f);
    assert!(
        s.output["path"].get("speed_mmps").is_none(),
        "hidden: {}",
        s.text
    );
    assert_eq!(s.mem()["vf_mmps"], vf, "vf itself is unchanged");
}

#[test]
fn heading_filter_is_the_rational_blend_of_5_2() {
    let mut rng = Rng::new(77);
    let mut rig = Rig::new();
    let mut h: Option<[i32; 2]> = None;
    let mut h_t: Option<i64> = None;
    let mut t = BASE_T;
    let mut reseeds = 0;
    for i in 0..400 {
        t += rng.range_i64(500, 6000);
        // courses concentrated in a half circle so blends are rarely exactly opposite
        let c = rng.range_i64(0, 3599) as u16;
        let mut f = fix_at(t, ORIGIN);
        f["course_ddeg"] = json!(c);
        f["speed_mmps"] = json!(10_000);
        let s = rig.step(&f);
        assert!(s.fix_ok());
        let m = unit_from_course(c).unwrap();
        let next = match (h, h_t) {
            (Some(h0), Some(ht)) => {
                let dt = t - ht;
                if dt > 5000 {
                    reseeds += 1;
                    m
                } else {
                    let a = alpha(dt, 4000);
                    let x =
                        rsr(i128::from(h0[0]) * (65_536 - a) + i128::from(m[0]) * a, 16).unwrap();
                    let y =
                        rsr(i128::from(h0[1]) * (65_536 - a) + i128::from(m[1]) * a, 16).unwrap();
                    normalize_q30(x as i64, y as i64).unwrap_or_else(|| {
                        reseeds += 1;
                        m
                    })
                }
            }
            _ => m,
        };
        h = Some(next);
        h_t = Some(t);
        assert_eq!(
            s.mem()["h"],
            json!([next[0], next[1]]),
            "heading at step {i} (course {c})"
        );
        assert_eq!(s.mem()["h_t_ms"], t);
        assert_eq!(s.output["path"]["heading_valid"], true);
    }
    assert!(
        reseeds > 10,
        "the reseed branch must be exercised ({reseeds})"
    );
}

#[test]
fn heading_validity_lasts_10_seconds() {
    let mut rig = Rig::new();
    let s = rig.step(&fix_at(BASE_T, ORIGIN));
    assert_eq!(s.output["path"]["heading_valid"], true);
    // no course, stationary -> no measurement and the heading ages
    for (dt, valid) in [
        (1000_i64, true),
        (5000, true),
        (9000, true),
        (11_000, false),
        (20_000, false),
    ] {
        let mut f = fix_at(BASE_T + dt, ORIGIN);
        unset(&mut f, "course_ddeg");
        let s = rig.peek(&f, BASE_T + dt);
        assert_eq!(
            s.output["path"]["heading_valid"], valid,
            "{dt} ms after the last update"
        );
    }
}

#[test]
fn heading_needs_speed_of_800_mmps_for_the_os_course() {
    let mut rig = Rig::new();
    for k in 0..6 {
        let mut f = fix_at(BASE_T + k * 1000, ORIGIN);
        f["speed_mmps"] = json!(700);
        f["course_ddeg"] = json!(900);
        let s = rig.step(&f);
        assert_eq!(
            s.output["path"]["heading_valid"], false,
            "step {k}: vf 700 < 800: the course is not used"
        );
    }
    let mut rig = Rig::new();
    let mut f = fix_at(BASE_T, ORIGIN);
    f["speed_mmps"] = json!(800);
    f["course_ddeg"] = json!(900);
    assert_eq!(rig.step(&f).output["path"]["heading_valid"], true);
}

// ---- dead reckoning and time to anchor (5.4) ----------------------------------------------------

/// Independent `along` of a fix on the standard route `approach_route(_, len_m)` (frame at ORIGIN).
fn along_mm(len_m: i64, p: P) -> (i64, i64) {
    let fr = frame_for(Position {
        lat_e7: ORIGIN.0,
        lon_e7: ORIGIN.1,
    })
    .unwrap();
    let start = offset(ORIGIN, 0, -len_m * 1000);
    let v: Vec<Enu> = [start, ORIGIN]
        .iter()
        .map(|q| {
            fr.project(Position {
                lat_e7: q.0,
                lon_e7: q.1,
            })
            .unwrap()
        })
        .collect();
    let pl = Polyline::new(&v).unwrap();
    let pr = project_to_polyline(
        &pl,
        fr.project(Position {
            lat_e7: p.0,
            lon_e7: p.1,
        })
        .unwrap(),
        None,
    )
    .unwrap();
    (pl.length_mm(), pr.along_mm)
}

#[test]
fn d_rem_and_t_hat_follow_5_4_exactly() {
    for (speed, floor, len) in [
        (15_000_i64, 4000_i64, 2500_i64),
        (3_000, 4000, 600),
        (29_000, 4000, 2500),
    ] {
        for lag_ms in [0_i64, 3000, 9000] {
            let d = Drive::new(len, speed);
            let mut rig = Rig::new().with_signpost(approach_route("r-1", len));
            let mut found = None;
            for k in 0..400 {
                let t = d.t_ms(k);
                let s = rig.call(&d.fix(k), t + lag_ms);
                if let Some(f) = s.fires().into_iter().next() {
                    found = Some((k, f, s));
                    break;
                }
            }
            let (k, f, s) =
                found.unwrap_or_else(|| panic!("no fire for speed {speed} lag {lag_ms}"));
            let (length, along) = along_mm(len, d.point(k));
            let vf = s.mem()["vf_mmps"].as_i64().unwrap();
            let v_eff = vf.max(floor);
            let rem = length - along;
            let want_d = (rem - (v_eff * lag_ms).div_euclid(1000)).max(0);
            let got_d = f["d_rem_mm"].as_i64().unwrap();
            assert!(
                (got_d - want_d).abs() <= 1,
                "speed {speed} lag {lag_ms}: d_rem {got_d} vs {want_d}"
            );
            let got_t = f["t_hat_ms"].as_i64().unwrap();
            assert_eq!(
                got_t,
                got_d * 1000 / v_eff,
                "t_hat = floor(d_rem x 1000 / v_eff), speed {speed} lag {lag_ms}"
            );
            assert_eq!(f["t_ms"], d.t_ms(k), "decision t_ms is the fix time");
        }
    }
}

#[test]
fn v_eff_floors_by_mode() {
    // t_hat divides by max(vf, floor): vehicle 4000, cycle 2000, foot 700 mm/s
    // (foot's 700 mm/s floor is unreachable in practice: below 800 mm/s there is no heading, 5.2)
    for (mode, speed, floor, len) in [
        ("vehicle", 3_000_i64, 4000_i64, 600_i64),
        ("cycle", 1_600, 2000, 500),
    ] {
        let mut sp = approach_route("r-1", len);
        set(&mut sp, "trigger.mode", json!(mode));
        let mut d = Drive::new(len, speed);
        d.start_along_mm = 0;
        let mut rig = Rig::new().with_signpost(sp);
        let mut fire = None;
        for k in 0..2500 {
            let s = rig.step(&d.fix(k));
            if let Some(f) = s.fires().into_iter().next() {
                fire = Some((f, s));
                break;
            }
        }
        // a vehicle at 3 m/s is slower than the cyclist band minimum etc: only assert when it fires
        if let Some((f, s)) = fire {
            let vf = s.mem()["vf_mmps"].as_i64().unwrap();
            let v_eff = vf.max(floor);
            assert_eq!(
                f["t_hat_ms"].as_i64().unwrap(),
                f["d_rem_mm"].as_i64().unwrap() * 1000 / v_eff,
                "{mode}"
            );
        } else {
            panic!("{mode} at {speed} mm/s never fired");
        }
    }
}

#[test]
fn walker_confidence_is_capped_at_600_and_fires_at_d_lo() {
    // E-P-FOOT: the walker cap is 600 unless the evidence is strong (both bonuses and
    // acc_eff <= 20 m), when it is 700. A 25 m fix accuracy is never strong.
    let fire_of = |acc_mm: u32, class: &str| {
        let len = 600;
        let mut sp = approach_route("r-1", len);
        set(&mut sp, "trigger.mode", json!("foot"));
        set(&mut sp, "class", json!(class));
        let mut d = Drive::new(len, 1_400);
        d.start_along_mm = 100_000;
        d.acc_mm = acc_mm;
        let mut rig = Rig::new().with_signpost(sp);
        rig.consent["nearby_health_events"] = json!("granted");
        rig.consent["health_receipt"] = json!("receipt-1");
        for k in 0..600 {
            let s = rig.step(&d.fix(k));
            if let Some(f) = s.fires().into_iter().next() {
                return Some(f);
            }
        }
        None
    };
    let weak = fire_of(25_000, "general").expect("a walker fires");
    assert_eq!(weak["path_conf_milli"], 600, "foot is capped at 600");
    let strong = fire_of(5_000, "general").expect("a walker fires");
    assert_eq!(
        strong["path_conf_milli"], 700,
        "strong foot evidence reaches 700"
    );
    let d_rem = strong["d_rem_mm"].as_i64().unwrap();
    // d_lo for foot = max(100 m, 3 acc_eff = 30 m) = 100 m; D_fire = clamp(1.4 x 43 = 60 m, d_lo, d_hi)
    assert!(
        (90_000..=101_000).contains(&d_rem),
        "walker fire distance {d_rem}"
    );
    // Health (floor 700): a walker receives it only on strong evidence.
    let h = fire_of(5_000, "health").expect("strong evidence reaches the health floor");
    assert_eq!(h["path_conf_milli"], 700);
    assert!(
        fire_of(25_000, "health").is_none(),
        "600 never meets the health floor"
    );
}

#[test]
fn cyclist_d_lo_grows_with_three_times_acc_eff() {
    // acc 40 m -> acc_eff 40 m -> d_lo = max(100 m, 120 m) = 120 m; slow cyclist D_fire = clamp(2 x 43 = 86, d_lo, ..)
    let len = 800;
    let mut sp = approach_route("r-1", len);
    set(&mut sp, "trigger.mode", json!("cycle"));
    let mut d = Drive::new(len, 2_000);
    d.start_along_mm = 300_000;
    d.acc_mm = 40_000;
    let mut rig = Rig::new().with_signpost(sp);
    let mut fire = None;
    for k in 0..600 {
        let s = rig.step(&d.fix(k));
        if let Some(f) = s.fires().into_iter().next() {
            fire = Some(f);
            break;
        }
    }
    let f = fire.expect("fires");
    let d_rem = f["d_rem_mm"].as_i64().unwrap();
    assert!(
        (100_000..=121_000).contains(&d_rem),
        "cycle d_lo 120 m: fired at {d_rem}"
    );
}
