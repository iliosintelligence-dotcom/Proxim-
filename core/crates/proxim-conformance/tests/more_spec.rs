//! Further spec probes: reserved-field tolerance for any value type (spec 10), geometry coordinates
//! out of range (F12), L_min = max(0.4 x lead_s, 10 s) and d_pass.
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

#[test]
fn reserved_fields_are_ignored_whatever_json_value_they_hold() {
    let plain = Call::base().run().text;
    let sp_fields = [
        "intent",
        "content_mode",
        "precision_class",
        "min_mode",
        "max_acc_m",
        "arm_scope",
        "relation",
        "selector",
        "live_at_ms",
        "fresh_required_s",
        "cooldown_s",
        "creatives",
        "pack_kind",
    ];
    let values: [Value; 7] = [
        json!(1),
        json!("x"),
        json!(true),
        json!([]),
        json!({}),
        json!([1, "a"]),
        json!({"a": [1]}),
    ];
    let mut refused = vec![];
    for f in sp_fields {
        for v in &values {
            let mut c = Call::base();
            c.signposts[0][f] = v.clone();
            let t = c.run().text;
            if t != plain {
                refused.push(format!("signpost.{f}={v}: {}", &t[..t.len().min(80)]));
            }
        }
    }
    // `rot_days`, `rot_lane` and `rot_lanes` are typed from engine_spec 2 (7.3), so they are no
    // longer "any JSON value"; their own tests are in proxim-decide/tests/rotation_decide.rs.
    assert!(
        refused.is_empty(),
        "reserved fields must be accepted and ignored:\n{}",
        refused.join("\n")
    );
}

#[test]
fn f12_coordinates_outside_the_valid_range_are_a_drop_not_a_crash() {
    for (name, path, p) in [
        (
            "origin lat 91",
            "trigger.origin",
            pos((910_000_000, 70_000_000)),
        ),
        (
            "origin lon 181",
            "trigger.origin",
            pos((450_000_000, 1_810_000_000)),
        ),
        (
            "circle centre lat -91",
            "trigger.shape.center",
            pos((-910_000_000, 70_000_000)),
        ),
        (
            "circle centre lon -181",
            "trigger.shape.center",
            pos((450_000_000, -1_810_000_000)),
        ),
    ] {
        let mut sp = enter_circle("c-1", 200);
        set(&mut sp, path, p);
        let mut c = Call::base();
        c.signposts = vec![sp];
        let s = c.run();
        assert!(
            s.is_ok(),
            "{name}: per-signpost defect must not fail the call: {}",
            s.text
        );
        assert_eq!(
            s.drops("c-1"),
            vec!["TRIGGER_INVALID".to_string()],
            "{name}: {}",
            s.text
        );
    }
    // the same for a route vertex
    let mut sp = approach_route("c-1", 2000);
    sp["trigger"]["shape"]["vertices"][0] = pos((910_000_000, 70_000_000));
    let mut c = Call::base();
    c.signposts = vec![sp];
    let s = c.run();
    assert!(
        s.is_ok() && s.drops("c-1") == ["TRIGGER_INVALID"],
        "{}",
        s.text
    );
}

/// Drive that starts so the first fix able to arm (index 4) has the requested `t_hat`.
fn late_probe(lead_s: i64, d_rem_at_arm_m: i64, speed: i64) -> (bool, bool) {
    let mut sp = approach_route("r-1", 2500);
    set(&mut sp, "trigger.lead_s", json!(lead_s));
    let mut d = Drive::new(2500, speed);
    // arming is at fix 4: along(4) = start + 4 steps
    d.start_along_mm = 2_500_000 - d_rem_at_arm_m * 1000 - 4 * speed;
    let mut rig = Rig::new().with_signpost(sp);
    let (mut late, mut fired) = (false, false);
    for k in 0..40 {
        let s = rig.step(&d.fix(k));
        late |= s.drops("r-1").contains(&"TOO_LATE_FOR_LEAD".to_string());
        fired |= s.fired("r-1");
    }
    (late, fired)
}

#[test]
fn l_min_is_the_larger_of_40_percent_of_lead_and_10_seconds() {
    // 15 m/s, arming ~12.7 s out (190 m, above d_lo = 150 m)
    // lead 40 s: L_min = 16 s  -> late
    // lead 15 s: L_min = 10 s  -> not late (12.7 s >= 10 s)
    // lead 25 s: L_min = 10 s  -> not late
    // lead 60 s: L_min = 24 s  -> late
    let (late40, fired40) = late_probe(40, 190, 15_000);
    assert!(late40 && !fired40, "lead 40: late join expected");
    let (late15, _) = late_probe(15, 190, 15_000);
    assert!(!late15, "lead 15 s has L_min 10 s: 12.7 s is enough");
    let (late25, _) = late_probe(25, 190, 15_000);
    assert!(!late25, "lead 25 s: 0.4 x 25 = 10 s");
    let (late60, fired60) = late_probe(60, 330, 15_000);
    assert!(late60 && !fired60, "lead 60 s: 22 s < 24 s -> late");
    let (late40b, _) = late_probe(40, 330, 15_000);
    assert!(!late40b, "22 s >= 16 s is fine for lead 40");
}

#[test]
fn inside_d_lo_a_short_t_hat_is_not_a_late_join() {
    // the late drop needs d_rem > d_lo (150 m): at 140 m with 9 s to go the crawl rule applies
    let mut sp = approach_route("r-1", 2500);
    set(&mut sp, "trigger.lead_s", json!(40));
    let mut d = Drive::new(2500, 15_000);
    d.start_along_mm = 2_500_000 - 200_000;
    let mut rig = Rig::new().with_signpost(sp);
    let mut late = false;
    for k in 0..30 {
        late |= rig
            .step(&d.fix(k))
            .drops("r-1")
            .contains(&"TOO_LATE_FOR_LEAD".to_string());
    }
    // 200 m out at start: arming at fix 4 happens at ~140 m (<= d_lo): not late
    assert!(
        !late,
        "d_rem <= d_lo at arming is the crawl rule, not a late join"
    );
}

#[test]
fn d_pass_is_the_larger_of_60_m_and_five_seconds_of_travel() {
    // at 29 m/s d_pass = 145 m: a start 100 m out is BEYOND_WINDOW and never fires or latches late
    let mut d = Drive::new(2500, 29_000);
    d.start_along_mm = 2_500_000 - 100_000 - 4 * 29_000;
    let mut rig = Rig::new().with_signpost(approach_route("r-1", 2500));
    let mut saw = false;
    for k in 0..10 {
        let s = rig.step(&d.fix(k));
        assert!(!s.fired("r-1"));
        saw |= s.reasons("r-1").contains(&"BEYOND_WINDOW".to_string());
    }
    assert!(saw);
    // at 8 m/s d_pass is the 60 m floor: arming 100 m out is inside the window and fires (crawl rule)
    let mut d = Drive::new(2500, 8_000);
    d.start_along_mm = 2_500_000 - 100_000 - 4 * 8_000;
    let mut rig = Rig::new().with_signpost(approach_route("r-1", 2500));
    let mut fired = false;
    for k in 0..12 {
        fired |= rig.step(&d.fix(k)).fired("r-1");
    }
    assert!(
        fired,
        "8 m/s, 100 m out: inside d_lo and beyond d_pass (60 m)"
    );
}

// ---- heading from displacement (5.2) and selection tie-break with an absent t_hat (7) -------------

fn heading_valid_after(speed_mmps: i64, acc_mm: u32) -> bool {
    let mut rig = Rig::new();
    let mut valid = false;
    for k in 0..14_i64 {
        let mut f = fix_at(BASE_T + k * 1000, offset(ORIGIN, 0, k * speed_mmps));
        unset(&mut f, "speed_mmps");
        unset(&mut f, "course_ddeg");
        f["acc_mm"] = json!(acc_mm);
        valid = rig.step(&f).output["path"]["heading_valid"] == true;
    }
    valid
}

#[test]
fn displacement_heading_needs_15_m_and_three_times_acc_eff() {
    // newest trace entry aged >= 3 s: displacement 3 v must be >= max(15 m, 3 acc_eff)
    assert!(heading_valid_after(11_000, 5_000), "33 m >= 30 m");
    assert!(!heading_valid_after(8_000, 5_000), "24 m < 3 x 10 m");
    assert!(
        heading_valid_after(11_000, 10_000),
        "acc 10 m -> acc_eff 10 m -> 30 m"
    );
    assert!(!heading_valid_after(11_000, 20_000), "33 m < 3 x 20 m");
    assert!(heading_valid_after(25_000, 20_000), "75 m >= 60 m");
}

#[test]
fn equal_scores_prefer_an_event_with_a_t_hat_over_one_without() {
    // approach z-1: t_hat 41.667 s against a 40 s lead, timing fit 980 (spec 7.5), so its score is
    // 1000 x 800 x 980 / 1e6 = 784. enter a-1 with rank 1568: 1568 x 500 / 1000 = 784.
    // Equal scores: t_hat ascending with ABSENT LAST, so z-1 beats a-1 although its id is larger.
    let mut found = None;
    for off_m in (0..400).step_by(3) {
        let mut rig = Rig::new();
        let centre = offset(ORIGIN, 0, -(300 + off_m) * 1000);
        let mut a = enter_circle_at("a-1", centre, 300);
        a["rank_milli"] = json!(1568);
        rig.signposts = vec![a, approach_route("z-1", 2500)];
        let d = Drive::new(2500, 15_000);
        for k in 0..200 {
            let s = rig.step(&d.fix(k));
            let lost = s.drops("a-1").contains(&"LOST_SLOT".to_string())
                || s.drops("z-1").contains(&"LOST_SLOT".to_string());
            if !s.fires().is_empty() || lost {
                if lost {
                    found = Some(s);
                }
                break;
            }
        }
        if found.is_some() {
            break;
        }
    }
    let s = found.expect("some circle placement makes both events land on the same fix");
    assert!(s.fired("z-1"), "absent t_hat sorts last: {}", s.text);
    assert_eq!(s.drops("a-1"), vec!["LOST_SLOT".to_string()]);
}
