//! Spec 3.2: canonical JSON. For random VALID values of every wire type, generated from the field
//! lists of spec 3.3 (not from the Rust types): `parse(write(x)) == x`, `write(parse(t)) == t` for
//! canonical `t`, and the writer's output satisfies the independent canonical checker.
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
use proxim_types::canon::{to_canonical_json, value_to_canonical};
use proxim_types::strict::{JsonLimits, from_canonical_json, from_json, parse_value};
use proxim_types::{Clock, Consent, Fix, Output, Policy, Signpost, State};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

// ---- generators (spec 3.3 field lists; only non-default values for optional fields) -------------

fn id(rng: &mut Rng) -> String {
    const CS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._:-";
    let long = rng.chance(1, 8);
    let n = 1 + rng.below(if long { 64 } else { 12 }) as usize;
    (0..n)
        .map(|_| CS[rng.below(CS.len() as u64) as usize] as char)
        .collect()
}

fn position(rng: &mut Rng) -> Value {
    json!({"lat_e7": rng.range_i64(-900_000_000, 900_000_000), "lon_e7": rng.range_i64(-1_800_000_000, 1_799_999_999)})
}

fn maybe(
    rng: &mut Rng,
    o: &mut serde_json::Map<String, Value>,
    k: &str,
    f: impl FnOnce(&mut Rng) -> Value,
) {
    if rng.chance(1, 2) {
        let v = f(rng);
        o.insert(k.into(), v);
    }
}

fn gen_fix(rng: &mut Rng) -> Value {
    let mut o = serde_json::Map::new();
    o.insert("t_ms".into(), json!(rng.range_i64(0, 4_000_000_000_000)));
    let p = position(rng);
    o.insert("lat_e7".into(), p["lat_e7"].clone());
    o.insert("lon_e7".into(), p["lon_e7"].clone());
    o.insert("acc_mm".into(), json!(rng.range_i64(0, 4_000_000_000)));
    maybe(rng, &mut o, "speed_mmps", |rng| {
        json!(rng.range_i64(0, 4_000_000_000))
    });
    maybe(rng, &mut o, "speed_acc_mmps", |rng| {
        json!(rng.range_i64(0, 4_000_000_000))
    });
    maybe(rng, &mut o, "course_ddeg", |rng| {
        json!(rng.range_i64(0, 3599))
    });
    maybe(rng, &mut o, "course_acc_ddeg", |rng| {
        json!(rng.range_i64(0, 3600))
    });
    maybe(rng, &mut o, "is_mock", |_| json!(true));
    Value::Object(o)
}

fn gen_clock(rng: &mut Rng) -> Value {
    json!({"now_ms": rng.range_i64(0, i64::MAX), "tz_offset_min": rng.range_i64(-840, 840)})
}

fn tricky_string(rng: &mut Rng) -> String {
    const PARTS: &[&str] = &[
        "a",
        "Z",
        "0",
        " ",
        "\"",
        "\\",
        "/",
        "\n",
        "\t",
        "\u{0}",
        "\u{1f}",
        "\u{7f}",
        "\u{80}",
        "é",
        "€",
        "\u{ffff}",
        "😀",
        "\u{10ffff}",
        "\u{202e}",
        "\u{2028}",
        "'",
        "<",
        "&",
        "%",
        "\u{8}",
        "\u{c}",
        "\r",
    ];
    (0..rng.below(20)).map(|_| *rng.pick(PARTS)).collect()
}

fn gen_consent(rng: &mut Rng) -> Value {
    let mut o = serde_json::Map::new();
    // The writer always emits the four scopes (absent = unknown), so canonical text carries all four.
    for k in [
        "location_matching",
        "nearby_health_events",
        "aggregate_reporting",
        "local_notifications",
    ] {
        o.insert(
            k.into(),
            json!(*rng.pick(&["granted", "denied", "unknown"])),
        );
    }
    maybe(rng, &mut o, "health_receipt", |rng| {
        json!(tricky_string(rng))
    });
    Value::Object(o)
}

fn gen_policy(rng: &mut Rng) -> Value {
    let mut o = serde_json::Map::new();
    maybe(rng, &mut o, "simulator_mode", |_| json!(true));
    maybe(rng, &mut o, "child_directed", |_| json!(true));
    maybe(rng, &mut o, "explain", |_| json!(true));
    maybe(rng, &mut o, "categories_blocked", |rng| {
        json!((0..1 + rng.below(3)).map(|_| id(rng)).collect::<Vec<_>>())
    });
    maybe(rng, &mut o, "senders_blocked", |rng| {
        json!((0..1 + rng.below(3)).map(|_| id(rng)).collect::<Vec<_>>())
    });
    maybe(rng, &mut o, "sensitivity_ceiling", |rng| {
        json!(rng.below(2))
    });
    maybe(rng, &mut o, "lead_s_min", |rng| {
        json!(rng.range_i64(16, 60))
    });
    maybe(rng, &mut o, "lead_s_max", |rng| {
        json!(rng.range_i64(61, 119))
    });
    maybe(rng, &mut o, "cap_signpost_day", |_| json!(1));
    maybe(rng, &mut o, "cap_signpost_week", |rng| {
        json!(rng.range_i64(1, 4))
    });
    maybe(rng, &mut o, "app_min_interval_s", |rng| {
        json!(rng.range_i64(1801, 100_000))
    });
    maybe(rng, &mut o, "slots", |_| json!(0));
    maybe(rng, &mut o, "max_acc_mm", |rng| {
        json!(rng.range_i64(1, 99_999))
    });
    maybe(rng, &mut o, "standoff_zones", |rng| {
        json!(
            (0..1 + rng.below(4))
                .map(|_| {
                    let mut z = json!({"id": id(rng), "center": position(rng)});
                    if rng.chance(1, 2) {
                        z["standoff_m"] = json!(rng.range_i64(1001, 50_000));
                    }
                    z
                })
                .collect::<Vec<_>>()
        )
    });
    Value::Object(o)
}

fn gen_signpost(rng: &mut Rng) -> Value {
    let mut sp = json!({
        "id": id(rng), "rev": rng.range_i64(1, 100_000), "class": *rng.pick(&["general", "health"]),
        "category": id(rng), "sender": id(rng), "sensitivity": rng.range_i64(0, 255),
        "event": *rng.pick(&["enter", "approach", "exit", "cross", "dwell", "pass", "sequence"]),
        "valid_from_ms": rng.range_i64(0, 4_000_000_000_000), "valid_until_ms": rng.range_i64(0, i64::MAX),
    });
    let o = sp.as_object_mut().unwrap();
    maybe(rng, o, "min_path_tier", |rng| json!(rng.range_i64(1, 3)));
    maybe(rng, o, "min_path_conf_milli", |rng| {
        json!(rng.range_i64(0, 499))
    });
    maybe(rng, o, "rank_milli", |rng| json!(rng.range_i64(0, 5000)));
    let mut t = serde_json::Map::new();
    t.insert("origin".into(), position(rng));
    maybe(rng, &mut t, "reach_m", |rng| {
        json!(rng.range_i64(1, 100_000))
    });
    maybe(rng, &mut t, "max_event_age_s", |rng| {
        json!(rng.range_i64(1, 3600))
    });
    maybe(rng, &mut t, "max_event_dist_m", |rng| {
        json!(rng.range_i64(1, 100_000))
    });
    let shape = match rng.below(3) {
        0 => {
            json!({"kind": "circle", "center": position(rng), "radius_m": rng.range_i64(150, 60_000)})
        }
        1 => {
            let mut s = json!({"kind": "polygon", "vertices": (0..3 + rng.below(8)).map(|_| position(rng)).collect::<Vec<_>>()});
            if rng.chance(1, 2) {
                s["inradius_m"] = json!(rng.range_i64(1, 5000));
            }
            s
        }
        _ => {
            let mut s = json!({"kind": "route", "buffer_m": rng.range_i64(30, 500), "vertices": (0..2 + rng.below(8)).map(|_| position(rng)).collect::<Vec<_>>()});
            if rng.chance(1, 2) {
                s["anchor_along_m"] = json!(rng.range_i64(0, 100_000));
            }
            s
        }
    };
    t.insert("shape".into(), shape);
    if rng.chance(1, 2) {
        t.insert("lead_s".into(), json!(rng.range_i64(15, 120)));
        maybe(rng, &mut t, "d_lo_m", |rng| json!(rng.range_i64(1, 1000)));
        maybe(rng, &mut t, "d_hi_m", |rng| json!(rng.range_i64(1, 5000)));
        maybe(rng, &mut t, "mode", |rng| {
            json!(*rng.pick(&["vehicle", "cycle", "foot"]))
        });
        maybe(rng, &mut t, "speed_min_mmps", |rng| {
            json!(rng.range_i64(0, 50_000))
        });
        maybe(rng, &mut t, "speed_max_mmps", |rng| {
            json!(rng.range_i64(0, 50_000))
        });
        maybe(rng, &mut t, "bearing_tol_ddeg", |rng| {
            json!(rng.range_i64(150, 450))
        });
    }
    sp["trigger"] = Value::Object(t);
    sp
}

fn gen_state(rng: &mut Rng) -> Value {
    let mut ids: Vec<String> = (0..rng.below(6)).map(|_| id(rng)).collect();
    ids.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    ids.dedup();
    let caps_sp: Vec<Value> = ids
        .iter()
        .map(|i| json!({"id": i, "day_idx": rng.range_i64(0, 100_000), "day_n": rng.range_i64(0, 2), "week_idx": rng.range_i64(0, 20_000), "week_n": rng.range_i64(0, 5)}))
        .collect();
    let mut caps = json!({"hwm_ms": rng.range_i64(0, 4_000_000_000_000), "signposts": caps_sp});
    if rng.chance(1, 2) {
        caps["last_fire_ms"] = json!(rng.range_i64(0, 4_000_000_000_000));
    }
    let mut t = rng.range_i64(0, 1_000_000_000_000);
    let trace: Vec<Value> = (0..rng.below(17))
        .map(|_| {
            t += rng.range_i64(1, 5000);
            let p = position(rng);
            let mut f = json!({"t_ms": t, "lat_e7": p["lat_e7"], "lon_e7": p["lon_e7"], "acc_mm": rng.range_i64(0, 100_000)});
            if rng.chance(1, 2) {
                f["v_mmps"] = json!(rng.range_i64(0, 60_000));
            }
            f
        })
        .collect();
    let det: Vec<Value> = ids
        .iter()
        .map(|i| {
            let st = match rng.below(5) {
                0 => {
                    json!({"s": "cand", "n": rng.range_i64(1, 9), "first_t": t, "last_t": t + 1000})
                }
                1 => {
                    let mut s = json!({"s": "in", "xn": rng.range_i64(0, 6), "last_t": t});
                    if rng.chance(1, 2) {
                        s["x_first_t"] = json!(t - 500);
                    }
                    s
                }
                2 => json!({"s": "cool", "until_ms": t + 1_800_000}),
                3 => {
                    let mut s = json!({"s": "armed", "last_t": t});
                    if rng.chance(1, 2) {
                        s["confirm_t"] = json!(t - 1000);
                    }
                    if rng.chance(1, 2) {
                        s["a_prev_mm"] = json!(rng.range_i64(-5000, 5_000_000));
                    }
                    s
                }
                _ => json!({"s": "latched", "why": *rng.pick(&["fired", "late"]), "last_t": t}),
            };
            json!({"id": i, "rev": rng.range_i64(1, 9), "st": st})
        })
        .collect();
    let mut mem = json!({"jump_rejects": rng.range_i64(0, 2), "trace": trace, "det": det});
    let m = mem.as_object_mut().unwrap();
    maybe(rng, m, "last_t_ms", |_| json!(t));
    maybe(rng, m, "vf_mmps", |rng| json!(rng.range_i64(0, 60_000)));
    if rng.chance(1, 2) {
        m.insert(
            "h".into(),
            json!([
                rng.range_i64(-(1 << 30), 1 << 30),
                rng.range_i64(-(1 << 30), 1 << 30)
            ]),
        );
        m.insert("h_t_ms".into(), json!(t));
    }
    json!({"v": 1, "caps": caps, "mem": mem})
}

const REASONS: &[&str] = &[
    "OK",
    "SUPPRESSED",
    "CONSENT_DENIED",
    "FIX_STALE",
    "FIX_JUMP",
    "SENSITIVE_STANDOFF",
    "TRIGGER_INVALID",
    "OFF_CORRIDOR",
    "TOO_LATE_FOR_LEAD",
    "CAP_SIGNPOST_DAY",
    "LOST_SLOT",
    "NO_EVENT",
];

fn gen_output(rng: &mut Rng) -> Value {
    let mut decisions: Vec<Value> = (0..rng.below(4))
        .map(|_| {
            let mut d = json!({
                "signpost": id(rng), "rev": rng.range_i64(1, 9), "outcome": *rng.pick(&["fire", "drop"]),
                "reason": *rng.pick(REASONS), "event": *rng.pick(&["enter", "approach"]),
                "t_ms": rng.range_i64(0, 4_000_000_000_000), "path_tier": rng.range_i64(0, 3),
                "path_conf_milli": rng.range_i64(0, 1000), "copy": *rng.pick(&["near_you", "ahead", "on_route"]),
            });
            let o = d.as_object_mut().unwrap();
            maybe(rng, o, "d_rem_mm", |rng| json!(rng.range_i64(0, 5_000_000)));
            maybe(rng, o, "t_hat_ms", |rng| json!(rng.range_i64(0, 500_000)));
            d
        })
        .collect();
    decisions.sort_by_key(|d| d["signpost"].as_str().unwrap().to_string());
    let mut path = json!({"tier": rng.range_i64(0, 3), "conf_milli": rng.range_i64(0, 1000), "heading_valid": rng.chance(1, 2)});
    maybe(rng, path.as_object_mut().unwrap(), "speed_mmps", |rng| {
        json!(rng.range_i64(0, 60_000))
    });
    let mut out = json!({"v": 1, "t_ms": rng.range_i64(0, 4_000_000_000_000), "fix": {"ok": rng.chance(1, 2), "reason": *rng.pick(REASONS)}, "path": path, "decisions": decisions});
    if rng.chance(1, 2) {
        out["notes"] = json!(
            (0..1 + rng.below(3))
                .map(|_| json!({"signpost": id(rng), "reason": *rng.pick(REASONS)}))
                .collect::<Vec<_>>()
        );
    }
    out
}

// ---- the laws ------------------------------------------------------------------------------------

fn laws<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(name: &str, v: &Value) {
    let t = render(v);
    check_canonical(&t)
        .unwrap_or_else(|e| panic!("{name}: generator emitted non-canonical text ({e}): {t}"));
    let x: T = from_canonical_json(t.as_bytes())
        .unwrap_or_else(|e| panic!("{name}: valid input refused ({e}): {t}"));
    let w = to_canonical_json(&x).unwrap();
    check_canonical(&w)
        .unwrap_or_else(|e| panic!("{name}: writer output not canonical ({e}): {w}"));
    assert_eq!(w, t, "{name}: write(parse(t)) != t");
    let y: T = from_canonical_json(w.as_bytes()).unwrap();
    assert_eq!(x, y, "{name}: parse(write(x)) != x");
    // the tolerant parser agrees with the canonical one on canonical text
    let z: T = from_json(t.as_bytes()).unwrap();
    assert_eq!(x, z, "{name}: from_json differs");
    // and the untyped canonicaliser is the identity on canonical text
    let tree = parse_value(t.as_bytes(), &JsonLimits::default()).unwrap();
    assert_eq!(
        value_to_canonical(&tree).unwrap(),
        t,
        "{name}: value_to_canonical is not the identity"
    );
}

#[test]
fn round_trip_fix() {
    let mut rng = Rng::new(1);
    for _ in 0..1500 {
        laws::<Fix>("Fix", &gen_fix(&mut rng));
    }
}

#[test]
fn round_trip_clock_and_consent_and_policy() {
    let mut rng = Rng::new(2);
    for _ in 0..1000 {
        laws::<Clock>("Clock", &gen_clock(&mut rng));
        laws::<Consent>("Consent", &gen_consent(&mut rng));
        laws::<Policy>("Policy", &gen_policy(&mut rng));
    }
}

#[test]
fn round_trip_signpost() {
    let mut rng = Rng::new(3);
    for _ in 0..1500 {
        laws::<Signpost>("Signpost", &gen_signpost(&mut rng));
    }
}

#[test]
fn round_trip_state_and_output() {
    let mut rng = Rng::new(4);
    for _ in 0..1000 {
        laws::<State>("State", &gen_state(&mut rng));
        laws::<Output>("Output", &gen_output(&mut rng));
    }
}

#[test]
fn string_escaping_is_minimal_and_lossless() {
    // only `"`, `\` and controls below 0x20 are escaped, controls as lowercase \u00xx; everything
    // else (including 0x7f, non-ASCII and astral characters) is written raw
    let mut rng = Rng::new(5);
    for _ in 0..2000 {
        let s = tricky_string(&mut rng);
        let c = json!({"location_matching": "granted", "nearby_health_events": "unknown", "aggregate_reporting": "unknown", "local_notifications": "unknown", "health_receipt": s});
        let x: Consent = from_canonical_json(render(&c).as_bytes()).unwrap();
        let w = to_canonical_json(&x).unwrap();
        assert_eq!(w, render(&c), "receipt {s:?}");
        let back: Value = serde_json::from_str(&w).unwrap();
        assert_eq!(back["health_receipt"], s.as_str(), "lossless");
    }
    let c = json!({"health_receipt": "q\"\\\u{1}\u{1f}\u{7f}é😀"});
    let w = to_canonical_json(&from_json::<Consent>(render(&c).as_bytes()).unwrap()).unwrap();
    assert!(
        w.contains("\"health_receipt\":\"q\\\"\\\\\\u0001\\u001f\u{7f}é😀\""),
        "{w}"
    );
}

#[test]
fn tolerant_parser_normalises_escape_and_whitespace_variants_to_canonical() {
    let t = " {\n \"tz_offset_min\" : -60 ,\t\"now_ms\":  5 } ";
    let c: Clock = from_json(t.as_bytes()).unwrap();
    assert_eq!(
        to_canonical_json(&c).unwrap(),
        "{\"now_ms\":5,\"tz_offset_min\":-60}"
    );
    let t = r#"{"health_receipt":"\u0041\/\u00e9\ud83d\ude00\n"}"#;
    let c: Consent = from_json(t.as_bytes()).unwrap();
    let w = to_canonical_json(&c).unwrap();
    assert!(w.contains("\"health_receipt\":\"A/é😀\\u000a\""), "{w}");
    check_canonical(&w).unwrap();
}

#[test]
fn from_canonical_json_rejects_every_non_canonical_spelling() {
    let good = r#"{"now_ms":5,"tz_offset_min":-60}"#;
    assert!(from_canonical_json::<Clock>(good.as_bytes()).is_ok());
    for bad in [
        " {\"now_ms\":5,\"tz_offset_min\":-60}",
        "{\"now_ms\":5,\"tz_offset_min\":-60} ",
        "{\"now_ms\": 5,\"tz_offset_min\":-60}",
        "{\"tz_offset_min\":-60,\"now_ms\":5}",
        "{\"now_ms\":+5,\"tz_offset_min\":-60}",
        "{\"now_ms\":05,\"tz_offset_min\":-60}",
        "{\"now_ms\":5.0,\"tz_offset_min\":-60}",
        "{\"now_ms\":5e0,\"tz_offset_min\":-60}",
        "{\"now_ms\":5,\"tz_offset_min\":-0}",
        "{\"now_ms\":null,\"tz_offset_min\":-60}",
        "{\"now_ms\":5,\"now_ms\":5,\"tz_offset_min\":-60}",
        "{\"now_ms\":5,\"tz_offset_min\":-60}x",
        "\u{feff}{\"now_ms\":5,\"tz_offset_min\":-60}",
        "{\"now_ms\":5,\"tz_offset_min\":-60,}",
        "{\"now_ms\":5,\"tz_offset_min\":-60,\"extra\":1}",
    ] {
        assert!(
            from_canonical_json::<Clock>(bad.as_bytes()).is_err(),
            "accepted non-canonical: {bad:?}"
        );
    }
    for bad in [
        r#"{"health_receipt":"\u0041"}"#,
        r#"{"health_receipt":"\/"}"#,
        r#"{"health_receipt":"\n"}"#,
        r#"{"health_receipt":"\u001F"}"#,
        "{\"health_receipt\":\"a\nb\"}",
        r#"{"health_receipt":"\u007f"}"#,
    ] {
        assert!(
            from_canonical_json::<Consent>(bad.as_bytes()).is_err(),
            "accepted non-canonical: {bad:?}"
        );
    }
}

#[test]
fn numbers_are_integers_in_every_parser_path() {
    // Even the tolerant parser refuses floats, exponents, '+', leading zeros, -0 and null (spec 3.2)
    for bad in [
        "1.0", "1e3", "1E3", "+1", "01", "-0", "null", "0x1", "1_0", ".5", "5.", "NaN", "Infinity",
    ] {
        let t = format!("{{\"now_ms\":{bad},\"tz_offset_min\":0}}");
        assert!(
            from_json::<Clock>(t.as_bytes()).is_err(),
            "tolerant parser accepted {bad}"
        );
    }
    // integer bounds of the typed fields
    assert!(from_json::<Clock>(br#"{"now_ms":9223372036854775807,"tz_offset_min":32767}"#).is_ok());
    assert!(from_json::<Clock>(br#"{"now_ms":9223372036854775808,"tz_offset_min":0}"#).is_err());
    assert!(from_json::<Clock>(br#"{"now_ms":0,"tz_offset_min":32768}"#).is_err());
    assert!(from_json::<Clock>(br#"{"now_ms":0,"tz_offset_min":-32769}"#).is_err());
}

#[test]
fn consent_values_other_than_exactly_granted_or_denied_parse_as_unknown() {
    // spec 3.2: any value other than exactly "granted" parses as `unknown` (the string "denied" stays denied)
    let parse = |v: Value| -> Value {
        let c: Consent = from_json(render(&json!({"location_matching": v})).as_bytes()).unwrap();
        serde_json::from_str::<Value>(&to_canonical_json(&c).unwrap()).unwrap()["location_matching"]
            .clone()
    };
    assert_eq!(parse(json!("granted")), "granted");
    assert_eq!(parse(json!("denied")), "denied");
    for v in [
        json!("unknown"),
        json!("Granted"),
        json!("GRANTED"),
        json!(" granted"),
        json!(""),
        json!("yes"),
        json!(1),
        json!(true),
        json!([]),
        json!({}),
        json!("granted\u{0}"),
    ] {
        let got = parse(v.clone());
        assert!(got == "unknown" || got.is_null(), "{v} parsed as {got}");
        assert_ne!(got, "granted");
    }
}
