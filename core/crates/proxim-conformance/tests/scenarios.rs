//! Awkward scenarios from the brief: many overlapping signposts, tunnels and GPS loss, re-arm
//! distance, quick reject, zero-narrowed policies, and bounded behaviour at the 4096-signpost limit.
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

#[test]
fn three_hundred_overlapping_signposts_give_one_fire_then_a_bounded_tail() {
    let mut rig = Rig::new();
    rig.signposts = (0..300)
        .map(|i| enter_circle(&format!("s-{i:03}"), 200))
        .collect();
    let mut first = None;
    for k in 0..4 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        if k == 3 {
            first = Some(s);
        }
    }
    let s = first.unwrap();
    assert_eq!(s.fires().len(), 1, "one slot");
    assert!(s.fired("s-000"), "equal scores: lowest id wins");
    let lost = s
        .decisions()
        .iter()
        .filter(|d| d["reason"] == "LOST_SLOT")
        .count();
    assert_eq!(lost, 299);
    // losers revert; at the next fix they all fire-eligible again but the app interval holds them
    let s2 = rig.step(&fix_at(BASE_T + 4000, ORIGIN));
    assert!(s2.fires().is_empty());
    let held = s2
        .decisions()
        .iter()
        .filter(|d| d["reason"] == "CAP_APP_INTERVAL")
        .count();
    assert_eq!(held, 299, "{}", &s2.text[..s2.text.len().min(300)]);
    // thereafter every detector is consumed: quiet
    let s3 = rig.step(&fix_at(BASE_T + 5000, ORIGIN));
    assert!(
        s3.fires().is_empty() && s3.drops("s-001").is_empty(),
        "{}",
        &s3.text[..s3.text.len().min(300)]
    );
    // only the winner has a cap entry
    let ids: Vec<String> = s3.caps()["signposts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["s-000"]);
}

#[test]
fn limit_of_4096_signposts_is_handled_call_after_call() {
    let mut rig = Rig::new();
    rig.signposts = (0..4096)
        .map(|i| {
            enter_circle_at(
                &format!("s-{i:04}"),
                offset(
                    ORIGIN,
                    (i % 64) * 200_000 - 6_000_000,
                    (i / 64) * 200_000 - 6_000_000,
                ),
                150,
            )
        })
        .collect();
    for k in 0..6 {
        let s = rig.step(&fix_at(BASE_T + k * 1000, ORIGIN));
        assert!(s.is_ok(), "{}", &s.text[..s.text.len().min(200)]);
        assert!(s.output["decisions"].as_array().unwrap().len() <= 1);
    }
    let st = rig.state_json();
    assert!(st["mem"]["det"].as_array().unwrap().len() <= 4096);
}

#[test]
fn signposts_far_from_the_fix_leave_no_trace() {
    // quick reject: idle and beyond reach records nothing, not even a note, and no detector entry
    let mut rig = Rig::new();
    rig.signposts = (0..50)
        .map(|i| {
            enter_circle_at(
                &format!("far-{i:02}"),
                offset(ORIGIN, 80_000_000 + i * 1000, 0),
                200,
            )
        })
        .collect();
    let s = rig.step(&fix_at(BASE_T, ORIGIN));
    assert!(s.is_ok() && s.fix_ok());
    assert!(s.decisions().is_empty(), "{}", s.text);
    assert!(
        s.output
            .get("notes")
            .is_none_or(|n| n.as_array().unwrap().is_empty()),
        "{}",
        s.text
    );
    assert_eq!(s.mem()["det"], json!([]));
}

#[test]
fn zero_narrowed_policies_block_everything() {
    for (k, v) in [
        ("slots", 0),
        ("cap_signpost_day", 0),
        ("cap_signpost_week", 0),
    ] {
        let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
        rig.policy[k] = json!(v);
        for t in 0..8 {
            let s = rig.step(&fix_at(BASE_T + t * 1000, ORIGIN));
            assert!(s.fires().is_empty(), "{k}=0 must not allow a fire");
        }
    }
}

fn tunnel_run(start_along_m: i64, gap_s: i64, speed: i64) -> (Vec<Step>, Vec<i64>) {
    let len = 3000;
    let mut rig = Rig::new().with_signpost(approach_route("r-1", len));
    let d = Drive::new(len, speed);
    let mut steps = vec![];
    let mut fire_at = vec![];
    let total = (len * 1000 / speed) + 10;
    let start_k = start_along_m * 1000 / speed;
    let mut k = 0;
    while k < total {
        if k >= start_k && k < start_k + gap_s {
            k += 1;
            continue; // no fixes inside the tunnel
        }
        let s = rig.step(&d.fix(k));
        assert!(s.is_ok(), "{}", s.text);
        if s.fired("r-1") {
            fire_at.push(k);
        }
        steps.push(s);
        k += 1;
    }
    (steps, fire_at)
}

#[test]
fn tunnels_and_gps_loss_never_double_fire_and_never_fire_late() {
    let mut min_lead = i64::MAX;
    let mut fired_runs = 0;
    for start_along_m in [300, 900, 1500, 1900, 2150, 2250, 2350] {
        for gap_s in [3, 8, 20, 30, 55, 90, 180] {
            let (steps, fires) = tunnel_run(start_along_m, gap_s, 15_000);
            assert!(
                fires.len() <= 1,
                "tunnel {gap_s}s @ {start_along_m} m: fired {} times",
                fires.len()
            );
            if let Some(f) = steps.iter().flat_map(Step::fires).next() {
                fired_runs += 1;
                // never a fire with less time to the anchor than L_min = 16 s unless the crawl rule applies (d_rem <= 150 m)
                let t_hat = f["t_hat_ms"].as_i64().unwrap();
                let d_rem = f["d_rem_mm"].as_i64().unwrap();
                assert!(
                    t_hat >= 10_000 || d_rem <= 150_000,
                    "tunnel {gap_s}s @ {start_along_m} m: late fire t_hat {t_hat} d_rem {d_rem}"
                );
                min_lead = min_lead.min(t_hat);
            }
        }
    }
    eprintln!("[tunnels] fired in {fired_runs}/49 runs, shortest realised lead {min_lead} ms");
    assert!(fired_runs >= 20, "most tunnels still fire ({fired_runs})");
}

#[test]
fn a_short_tunnel_before_the_fire_distance_still_fires_once_with_a_normal_lead() {
    // 8 s without fixes at 15 m/s (120 m) well before D_fire = 645 m
    let (steps, fires) = tunnel_run(900, 8, 15_000);
    assert_eq!(fires.len(), 1);
    let f = steps.iter().flat_map(Step::fires).next().unwrap();
    let t_hat = f["t_hat_ms"].as_i64().unwrap();
    assert!((30_000..=43_500).contains(&t_hat), "t_hat {t_hat}");
}

#[test]
fn rearm_needs_a_fix_beyond_1_2_d_hi_of_the_anchor_point() {
    let d = Drive::new(2500, 15_000);
    let build = || {
        let mut rig = Rig::new().with_signpost(approach_route("r-1", 2500));
        for k in 0..150 {
            rig.step(&d.fix(k));
        }
        assert_eq!(
            rig.history.last().unwrap().det("r-1").unwrap()["st"]["s"],
            "latched"
        );
        rig
    };
    let t = d.t_ms(150) + 70_000; // gap > 60 s: no FIX_JUMP bookkeeping
    // 3560 m from the anchor (< 1.2 x 3000 m): still latched
    let mut rig = build();
    let s = rig.call(&fix_at(t, offset(ORIGIN, 0, 3_560_000)), t);
    assert!(s.fix_ok(), "{}", s.fix_reason());
    assert_eq!(
        s.det("r-1").unwrap()["st"]["s"],
        "latched",
        "within 1.2 x d_hi"
    );
    // 3640 m: released
    let mut rig = build();
    let s = rig.call(&fix_at(t, offset(ORIGIN, 0, 3_640_000)), t);
    let det = s.det("r-1");
    assert!(
        det.is_none() || det.unwrap()["st"]["s"] != "latched",
        "beyond 1.2 x d_hi re-arms"
    );
    // distance is Euclidean, not along the route: 3640 m to the east counts too
    let mut rig = build();
    let s = rig.call(&fix_at(t, offset(ORIGIN, 3_640_000, 0)), t);
    let det = s.det("r-1");
    assert!(det.is_none() || det.unwrap()["st"]["s"] != "latched");
}

#[test]
fn enter_speed_threshold_for_the_short_dwell_is_1500_mmps() {
    for (v, expect_idx) in [(1_499_i64, 10_usize), (1_500, 3)] {
        let mut rig = Rig::new().with_signpost(enter_circle("c-1", 200));
        let idx = (0..14).find(|k| {
            let mut f = fix_at(BASE_T + k * 1000, ORIGIN);
            f["speed_mmps"] = json!(v);
            rig.step(&f).fired("c-1")
        });
        assert_eq!(idx, Some(expect_idx as i64), "speed {v}");
    }
}

#[test]
fn many_unrelated_fixes_do_not_grow_state_without_bound() {
    // trace <= 16, det only for tracked signposts, caps only for fired ones
    let mut rig = Rig::new()
        .with_signpost(enter_circle("c-1", 200))
        .with_signpost(approach_route("r-1", 2000));
    let mut max_len = 0;
    for k in 0..3000_i64 {
        let p = offset(
            ORIGIN,
            (k % 40) * 30_000 - 600_000,
            ((k / 40) % 40) * 30_000 - 600_000,
        );
        let mut f = fix_at(BASE_T + k * 2000, p);
        f["speed_mmps"] = json!(10_000);
        let s = rig.call(&f, BASE_T + k * 2000);
        max_len = max_len.max(s.text.len());
    }
    assert!(max_len < 20_000, "state/output grew to {max_len} bytes");
}

#[test]
fn health_receipt_must_be_non_empty_to_unlock_health() {
    let mut sp = enter_circle("c-1", 200);
    sp["class"] = json!("health");
    let mut c = Call::base();
    c.signposts = vec![sp];
    c.consent["nearby_health_events"] = json!("granted");
    c.consent["health_receipt"] = json!("");
    let s = c.run();
    assert!(s.is_ok());
    assert_eq!(
        s.drops("c-1"),
        vec!["HEALTH_CONSENT_MISSING".to_string()],
        "an empty receipt is no receipt: {}",
        s.text
    );
}
