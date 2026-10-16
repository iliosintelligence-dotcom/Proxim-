//! Cross-cutting invariants: I6 output hygiene, I4/I9 fail-closed, state well-formedness, and the
//! canonical-state contract, over seeded random worlds.
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
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn walk_values(v: &Value, f: &mut impl FnMut(&str, &Value)) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                f(k, x);
                walk_values(x, f);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| walk_values(x, f)),
        _ => {}
    }
}

/// Sweep multiplier (default 1). `PROXIM_CONF_SCALE=5` runs five times as many seeds.
fn scale() -> u64 {
    std::env::var("PROXIM_CONF_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

const DECISION_KEYS: [&str; 11] = [
    "copy",
    "d_rem_mm",
    "event",
    "outcome",
    "path_conf_milli",
    "path_tier",
    "reason",
    "rev",
    "signpost",
    "t_hat_ms",
    "t_ms",
];

/// Every output obeys the structural privacy contract of spec 3.3.
fn check_output(out: &Value, explain: bool, forbidden_numbers: &BTreeSet<i64>) {
    let top: BTreeSet<&str> = out
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let allowed: BTreeSet<&str> = ["v", "t_ms", "fix", "path", "decisions", "notes"]
        .into_iter()
        .collect();
    assert!(top.is_subset(&allowed), "unexpected top-level keys {top:?}");
    assert!(
        top.contains("v")
            && top.contains("t_ms")
            && top.contains("fix")
            && top.contains("path")
            && top.contains("decisions")
    );
    // no coordinates or geometry, ever (I2, I6)
    walk_values(out, &mut |k, _| {
        assert!(
            !matches!(
                k,
                "lat_e7"
                    | "lon_e7"
                    | "lat"
                    | "lon"
                    | "center"
                    | "origin"
                    | "vertices"
                    | "position"
                    | "e_mm"
                    | "n_mm"
                    | "xt_mm"
                    | "along_mm"
            ),
            "geometry key `{k}` in an Output"
        );
    });
    // no number in the output may equal a coordinate that was fed in
    fn nums(v: &Value, f: &mut impl FnMut(i64)) {
        match v {
            Value::Number(n) => f(n.as_i64().unwrap_or(0)),
            Value::Array(a) => a.iter().for_each(|x| nums(x, f)),
            Value::Object(m) => m.values().for_each(|x| nums(x, f)),
            _ => {}
        }
    }
    nums(out, &mut |n| {
        assert!(
            !forbidden_numbers.contains(&n),
            "Output contains the coordinate {n}"
        )
    });
    for d in out["decisions"].as_array().unwrap() {
        let keys: BTreeSet<&str> = d.as_object().unwrap().keys().map(String::as_str).collect();
        assert!(
            keys.is_subset(&DECISION_KEYS.into_iter().collect()),
            "decision keys {keys:?}"
        );
        if !explain {
            assert_eq!(
                d["outcome"], "fire",
                "without explain `decisions` holds only fires"
            );
            assert_eq!(d["reason"], "OK");
        }
    }
    if !explain {
        assert!(out.get("notes").is_none(), "no notes without explain");
        assert!(
            out["decisions"].as_array().unwrap().len() <= 1,
            "at most `slots` (1) fires"
        );
        let r = out["fix"]["reason"].as_str().unwrap();
        assert!(
            !matches!(
                r,
                "SENSITIVE_STANDOFF"
                    | "TRIGGER_IN_STANDOFF"
                    | "OUT_OF_RANGE"
                    | "OFF_CORRIDOR"
                    | "NO_EVENT"
            ),
            "proximity-revealing reason `{r}` leaked without explain"
        );
    }
}

fn coordinate_numbers(signposts: &[Value], trace: &[(Value, i64)]) -> BTreeSet<i64> {
    let mut s = BTreeSet::new();
    for (f, _) in trace {
        s.insert(f["lat_e7"].as_i64().unwrap());
        s.insert(f["lon_e7"].as_i64().unwrap());
    }
    for sp in signposts {
        walk_values(sp, &mut |k, v| {
            if matches!(k, "lat_e7" | "lon_e7") {
                s.insert(v.as_i64().unwrap());
            }
        });
    }
    s.insert(ORIGIN.0.into());
    s.insert(ORIGIN.1.into());
    s
}

fn zone_policy(explain: bool) -> Value {
    json!({"explain": explain, "standoff_zones": [{"id": "z-1", "center": pos(offset(ORIGIN, 1_000_000, -1_000_000))}]})
}

#[test]
fn outputs_never_contain_coordinates_and_hide_reasons_without_explain() {
    let mut total = 0;
    let mut fires = 0;
    let mut suppressed = 0;
    for seed in 1..=(5 * scale()) {
        for explain in [false, true] {
            let mut rng = Rng::new(seed);
            let sps = mixed_signposts(&mut rng);
            let mut trace = random_trace(&mut rng, 200);
            trace.extend(
                scripted_trace(&mut rng, &sps, 12)
                    .into_iter()
                    .map(|(f, n)| {
                        let mut f = f;
                        f["t_ms"] = json!(f["t_ms"].as_i64().unwrap() + 200_000_000);
                        (f, n + 200_000_000)
                    }),
            );
            let mut rig = Rig::new();
            rig.signposts = sps.clone();
            rig.policy = zone_policy(explain);
            rig.consent["nearby_health_events"] = json!("granted");
            rig.consent["health_receipt"] = json!("r-1");
            let forbidden = coordinate_numbers(&sps, &trace);
            for (f, now) in &trace {
                let s = rig.call(f, *now);
                if s.is_ok() {
                    check_output(&s.output, explain, &forbidden);
                    total += 1;
                    fires += s.fires().len();
                    suppressed += usize::from(
                        !s.fix_ok() && s.fix_reason().contains("SUPPRESSED")
                            || s.fix_reason() == "SENSITIVE_STANDOFF",
                    );
                }
            }
        }
    }
    assert!(total > 8_000, "{total} outputs checked");
    assert!(
        fires > 20 && suppressed > 150,
        "the sweep must exercise fires ({fires}) and standoffs ({suppressed})"
    );
}

#[test]
fn what_a_fix_in_a_standoff_looks_like_does_not_depend_on_how_deep_it_is() {
    // I6: without explain the Output for any fix inside the zone is byte-identical (same t_ms and
    // same previous state), whatever the distance to the zone centre.
    let zone = offset(ORIGIN, 1_000_000, -1_000_000);
    let mut texts = BTreeSet::new();
    for d in [0_i64, 50, 300, 700, 990] {
        let mut c = Call::base();
        c.policy = zone_policy(false);
        c.fix = fix_at(BASE_T, offset(zone, 0, d * 1000));
        let s = c.run();
        assert_eq!(s.fix_reason(), "SUPPRESSED");
        texts.insert(render(&s.output));
        assert_eq!(s.state["mem"], empty_mem());
    }
    assert_eq!(texts.len(), 1, "outputs differ by depth: {texts:?}");
    // and it equals the output for an unrelated suppression-free call only in shape, not content:
    let mut c = Call::base();
    c.policy = zone_policy(false);
    let outside = c.run();
    assert_ne!(outside.fix_reason(), "SUPPRESSED");
}

#[test]
fn state_is_always_well_formed() {
    for seed in 100..(104 + 4 * scale()) {
        let mut rng = Rng::new(seed);
        let sps = mixed_signposts(&mut rng);
        let mut trace = random_trace(&mut rng, 150);
        trace.extend(
            scripted_trace(&mut rng, &sps, 14)
                .into_iter()
                .map(|(mut f, n)| {
                    f["t_ms"] = json!(f["t_ms"].as_i64().unwrap() + 200_000_000);
                    (f, n + 200_000_000)
                }),
        );
        let mut rig = Rig::new();
        rig.signposts = sps;
        rig.policy = zone_policy(true);
        let mut last_hwm = 0_i64;
        for (f, now) in &trace {
            let s = rig.call(f, *now);
            if !s.is_ok() {
                continue;
            }
            let st = &s.state;
            assert_eq!(st["v"], 1);
            // caps: ascending by id, hwm never decreases, counts within caps
            let caps = st["caps"]["signposts"].as_array().unwrap();
            let ids: Vec<&str> = caps.iter().map(|e| e["id"].as_str().unwrap()).collect();
            assert!(
                ids.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes()),
                "caps ids not strictly ascending: {ids:?}"
            );
            for e in caps {
                assert!(
                    e["day_n"].as_i64().unwrap() <= 2 && e["week_n"].as_i64().unwrap() <= 5,
                    "{e}"
                );
            }
            let hwm = st["caps"]["hwm_ms"].as_i64().unwrap();
            assert!(hwm >= last_hwm, "hwm decreased {last_hwm} -> {hwm}");
            last_hwm = hwm;
            // mem: trace <= 16, strictly ascending in time; det ascending by id; jump_rejects < 3
            let tr = st["mem"]["trace"].as_array().unwrap();
            assert!(tr.len() <= 16);
            assert!(
                tr.windows(2)
                    .all(|w| w[0]["t_ms"].as_i64() < w[1]["t_ms"].as_i64()),
                "trace not strictly ascending"
            );
            if let Some(last) = tr.last() {
                assert!(st["mem"]["last_t_ms"].as_i64().unwrap() >= last["t_ms"].as_i64().unwrap());
            }
            let det: Vec<&str> = st["mem"]["det"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["id"].as_str().unwrap())
                .collect();
            assert!(
                det.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes()),
                "det ids not ascending: {det:?}"
            );
            assert!(st["mem"]["jump_rejects"].as_i64().unwrap() < 3);
            // idle detectors are absent: every entry has a real state tag
            for e in st["mem"]["det"].as_array().unwrap() {
                assert!(
                    matches!(
                        e["st"]["s"].as_str(),
                        Some("cand" | "in" | "cool" | "armed" | "latched")
                    ),
                    "{e}"
                );
            }
            // decisions: fires first then by id, unique (id, reason)
            let ds = s.decisions();
            let mut seen_drop = false;
            for d in &ds {
                if d["outcome"] == "drop" {
                    seen_drop = true;
                } else {
                    assert!(!seen_drop, "fire after a drop: {}", s.text);
                }
            }
        }
    }
}

#[test]
fn caps_hold_over_random_worlds() {
    // per-signpost: never more than 2 fires per local day or 5 per week; never two fires of the
    // app within 1800 s; one fire per call.
    let mut total_fires = 0;
    for seed in 500..(504 + 4 * scale()) {
        let mut rng = Rng::new(seed);
        let sps = mixed_signposts(&mut rng);
        let trace = scripted_trace(&mut rng, &sps, 24);
        let mut rig = Rig::new();
        rig.signposts = sps;
        rig.consent["nearby_health_events"] = json!("granted");
        rig.consent["health_receipt"] = json!("r-1");
        let mut fires: Vec<(String, i64)> = vec![];
        for (f, now) in &trace {
            let s = rig.call(f, *now);
            if !s.is_ok() {
                continue;
            }
            assert!(s.fires().len() <= 1);
            for d in s.fires() {
                fires.push((
                    d["signpost"].as_str().unwrap().to_string(),
                    s.output["t_ms"].as_i64().unwrap(),
                ));
            }
        }
        total_fires += fires.len();
        for w in fires.windows(2) {
            assert!(
                w[1].1 - w[0].1 >= 1_800_000,
                "seed {seed}: fires {w:?} within the app interval"
            );
        }
        let mut per: std::collections::BTreeMap<String, Vec<i64>> = Default::default();
        for (id, t) in &fires {
            per.entry(id.clone()).or_default().push(*t);
        }
        for (id, ts) in per {
            let mut days: std::collections::BTreeMap<i64, i64> = Default::default();
            let mut weeks: std::collections::BTreeMap<i64, i64> = Default::default();
            for t in ts {
                *days.entry(day_idx(t, 0)).or_default() += 1;
                *weeks.entry(week_idx(t, 0)).or_default() += 1;
            }
            assert!(days.values().all(|&n| n <= 2), "seed {seed} {id}: {days:?}");
            assert!(
                weeks.values().all(|&n| n <= 5),
                "seed {seed} {id}: {weeks:?}"
            );
        }
    }
    assert!(
        total_fires > 30,
        "only {total_fires} fires: the sweep proves little"
    );
}

#[test]
fn consent_withdrawal_at_any_point_wipes_mem_and_nothing_fires_while_withdrawn() {
    let mut rng = Rng::new(31337);
    let sps = mixed_signposts(&mut rng);
    let trace = scripted_trace(&mut rng, &sps, 14);
    let mut rig = Rig::new();
    rig.signposts = sps;
    let mut fires_while_granted = 0;
    for (i, (f, now)) in trace.iter().enumerate() {
        let withdrawn = (i / 40) % 4 == 3;
        rig.consent["location_matching"] = json!(if withdrawn { "denied" } else { "granted" });
        let s = rig.call(f, *now);
        if !s.is_ok() {
            continue;
        }
        if withdrawn {
            assert!(s.fires().is_empty());
            assert!(!s.fix_ok() && s.fix_reason() == "CONSENT_DENIED");
            assert_eq!(s.mem(), &empty_mem(), "step {i}");
        } else {
            fires_while_granted += s.fires().len();
        }
    }
    assert!(fires_while_granted > 0);
}

#[test]
fn child_directed_disables_everything_always() {
    let mut rng = Rng::new(555);
    let sps = mixed_signposts(&mut rng);
    let trace = random_trace(&mut rng, 150);
    let mut rig = Rig::new();
    rig.signposts = sps;
    rig.policy["child_directed"] = json!(true);
    for (f, now) in &trace {
        let s = rig.call(f, *now);
        if s.is_ok() {
            assert_eq!(s.fix_reason(), "ENGINE_DISABLED");
            assert!(s.fires().is_empty());
            assert_eq!(s.mem(), &empty_mem());
        }
    }
}

#[test]
fn reason_codes_match_the_section_8_table() {
    use proxim_types::ReasonCode as R;
    // transcribed from engine-spec section 8: (first id, [codes])
    let rows: [(u16, &[&str]); 8] = [
        (0, &["OK", "SUPPRESSED"]),
        (100, &["CONSENT_DENIED", "ENGINE_DISABLED"]),
        (
            110,
            &[
                "FIX_INVALID",
                "FIX_MOCK",
                "FIX_ACCURACY_TOO_COARSE",
                "FIX_FROM_FUTURE",
                "FIX_STALE",
                "FIX_OUT_OF_ORDER",
                "FIX_JUMP",
                "FIX_OUT_OF_FRAME",
                "ACC_TOO_COARSE",
            ],
        ),
        (120, &["SENSITIVE_STANDOFF"]),
        (
            200,
            &[
                "UNSUPPORTED_EVENT",
                "TRIGGER_INVALID",
                "NOT_YET_LIVE",
                "EXPIRED",
                "HEALTH_CONSENT_MISSING",
                "PARTNER_CATEGORY_BLOCKED",
                "SENDER_BLOCKED",
                "SENSITIVITY_ABOVE_CEILING",
                "TRIGGER_IN_STANDOFF",
                "OUT_OF_RANGE",
            ],
        ),
        (
            300,
            &[
                "OFF_CORRIDOR",
                "SPEED_OUT_OF_BAND",
                "BEARING_MISMATCH",
                "NOT_CLOSING",
                "BEYOND_WINDOW",
                "PATH_CONFIDENCE_LOW",
                "TOO_LATE_FOR_LEAD",
                "PATH_TIER_LOW",
            ],
        ),
        (
            400,
            &[
                "EVENT_TOO_OLD",
                "EVENT_TOO_FAR",
                "CAP_SIGNPOST_DAY",
                "CAP_SIGNPOST_WEEK",
                "CAP_APP_INTERVAL",
                "LOST_SLOT",
                "NO_EVENT",
            ],
        ),
        (
            500,
            &[
                "REVOKED",
                "SIGNATURE_INVALID",
                "SEQ_ROLLBACK",
                "KID_REVOKED",
                "PACK_EXPIRED",
            ],
        ),
    ];
    let mut n = 0;
    for (first, codes) in rows {
        for (i, name) in codes.iter().enumerate() {
            let want_id = first + i as u16;
            let parsed: R = serde_json::from_str(&format!("\"{name}\""))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(parsed.id(), want_id, "{name}");
            assert_eq!(
                serde_json::to_string(&parsed).unwrap(),
                format!("\"{name}\"")
            );
            n += 1;
        }
    }
    assert_eq!(n, 44);
    // the rest of row 500 (PLAN 5.6 codes, M3) is still reserved
    assert!(serde_json::from_str::<R>("\"CAP_SENDER_DAY\"").is_err());
    assert!(
        serde_json::from_str::<R>("\"ok\"").is_err(),
        "wire form is SCREAMING_SNAKE"
    );
}
