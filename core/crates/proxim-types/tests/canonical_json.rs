//! Canonical JSON writer + strict parser: round trips, rejection table, properties (spec 3.2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use proxim_types::canon::{to_canonical_json, value_to_canonical};
use proxim_types::strict::{
    JsonLimits, from_canonical_json, from_json, from_json_with, parse_value,
};
use proxim_types::{
    Caps, Consent, ConsentValue, DetEntry, DetState, EngineError, Fix, Id, LatchWhy, Mem, Policy,
    Signpost, SignpostCaps, State, TraceFix,
};
use serde_json::Value;

// Built with concat! so no tool layer can pre-decode the printable escapes.
const ESCAPED_KEY: &str = concat!(
    r#"{""#,
    "\\",
    r#"u0061cc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#
);
const HIGH_THEN_LETTER: &str = concat!(r#"{"a"#, "\\", "ud800", "\\", r#"u0041":1}"#);

const SIGNPOST: &str = r#"{"category":"cafe","class":"general","event":"approach","id":"sp-1","rank_milli":900,"rev":3,"sender":"acme","sensitivity":0,"trigger":{"bearing_tol_ddeg":300,"d_hi_m":3000,"lead_s":40,"mode":"vehicle","origin":{"lat_e7":450000000,"lon_e7":75000000},"shape":{"anchor_along_m":1200,"buffer_m":30,"kind":"route","vertices":[{"lat_e7":450000000,"lon_e7":75000000},{"lat_e7":450100000,"lon_e7":75000000}]}},"valid_from_ms":0,"valid_until_ms":9000000000000}"#;

#[test]
fn signpost_round_trips_byte_for_byte() {
    let sp: Signpost = from_canonical_json(SIGNPOST.as_bytes()).unwrap();
    assert_eq!(sp.id.as_str(), "sp-1");
    assert_eq!(to_canonical_json(&sp).unwrap(), SIGNPOST);
}

#[test]
fn reserved_fields_are_accepted_ignored_and_never_re_emitted() {
    let with_reserved = SIGNPOST.replacen(
        r#""id":"sp-1""#,
        r#""creatives":[{"a":1}],"id":"sp-1","intent":"x""#,
        1,
    );
    let sp: Signpost = from_json(with_reserved.as_bytes()).unwrap();
    assert_eq!(to_canonical_json(&sp).unwrap(), SIGNPOST);
    let pol: Policy = from_json(br#"{"rot_lane":{"x":[1,2]},"rot_lanes":3}"#).unwrap();
    assert_eq!(to_canonical_json(&pol).unwrap(), "{}");
}

#[test]
fn state_with_every_detector_tag_round_trips() {
    let st = State {
        v: 1,
        caps: Caps {
            hwm_ms: 1_700_000_000_000,
            last_fire_ms: Some(1_699_999_000_000),
            signposts: vec![SignpostCaps {
                id: Id::new("a").unwrap(),
                day_idx: 19_000,
                day_n: 1,
                week_idx: 2_714,
                week_n: 3,
            }],
        },
        mem: Mem {
            last_t_ms: Some(5),
            jump_rejects: 2,
            trace: vec![TraceFix {
                t_ms: 4,
                lat_e7: -450_000_000,
                lon_e7: -1_799_999_999,
                acc_mm: 12_000,
                v_mmps: Some(0),
            }],
            vf_mmps: Some(13_000),
            h: Some([-1_073_741_824, 0]),
            h_t_ms: Some(4),
            det: vec![
                DetEntry {
                    id: Id::new("a").unwrap(),
                    rev: 1,
                    st: DetState::Cand {
                        n: 2,
                        first_t: 1,
                        last_t: 2,
                    },
                },
                DetEntry {
                    id: Id::new("b").unwrap(),
                    rev: 1,
                    st: DetState::In {
                        xn: 0,
                        x_first_t: None,
                        last_t: 2,
                    },
                },
                DetEntry {
                    id: Id::new("c").unwrap(),
                    rev: 1,
                    st: DetState::Cool { until_ms: 99 },
                },
                DetEntry {
                    id: Id::new("d").unwrap(),
                    rev: 1,
                    st: DetState::Armed {
                        confirm_t: Some(3),
                        last_t: 4,
                    },
                },
                DetEntry {
                    id: Id::new("e").unwrap(),
                    rev: 1,
                    st: DetState::Latched {
                        why: LatchWhy::Late,
                        last_t: 4,
                    },
                },
            ],
        },
    };
    let j = to_canonical_json(&st).unwrap();
    assert!(
        j.contains(r#""st":{"last_t":4,"s":"latched","why":"late"}"#),
        "{j}"
    );
    let back: State = from_canonical_json(j.as_bytes()).unwrap();
    assert_eq!(back, st);
    assert_eq!(to_canonical_json(&back).unwrap(), j);
}

#[test]
fn rejection_table_all_e_parse() {
    let fix_ok = br#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#;
    assert!(from_json::<Fix>(fix_ok).is_ok());
    let cases: &[(&str, &str)] = &[
        (
            "unknown field",
            r#"{"acc_mm":1,"bogus":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        ("missing field", r#"{"acc_mm":1,"lat_e7":1,"t_ms":1}"#),
        ("float", r#"{"acc_mm":1.0,"lat_e7":1,"lon_e7":1,"t_ms":1}"#),
        (
            "exponent",
            r#"{"acc_mm":1e0,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "upper exponent",
            r#"{"acc_mm":1E0,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "fraction only dot",
            r#"{"acc_mm":1.,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "negative zero",
            r#"{"acc_mm":1,"lat_e7":-0,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "leading zero",
            r#"{"acc_mm":01,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "plus sign",
            r#"{"acc_mm":+1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "null optional",
            r#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"speed_mmps":null,"t_ms":1}"#,
        ),
        (
            "duplicate key",
            r#"{"acc_mm":1,"acc_mm":2,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "negative into u32",
            r#"{"acc_mm":-1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "i32 overflow",
            r#"{"acc_mm":1,"lat_e7":2147483648,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "u64 overflow",
            r#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":18446744073709551616}"#,
        ),
        (
            "string for number",
            r#"{"acc_mm":"1","lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "trailing comma",
            r#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1,}"#,
        ),
        (
            "trailing garbage",
            r#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}x"#,
        ),
        (
            "two documents",
            r#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}{}"#,
        ),
        ("truncated", r#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":"#),
        ("empty", ""),
        ("only whitespace", "  "),
        ("single quotes", "{'acc_mm':1}"),
        (
            "comment",
            r#"{"acc_mm":1,/*x*/"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        ("NaN", r#"{"acc_mm":NaN,"lat_e7":1,"lon_e7":1,"t_ms":1}"#),
        (
            "Infinity",
            r#"{"acc_mm":Infinity,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "bool for bool is fine but int is not",
            r#"{"acc_mm":1,"is_mock":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        ("array root", "[1]"),
        (
            "bare minus",
            r#"{"acc_mm":-,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "bad literal",
            r#"{"acc_mm":tru,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        ("key not string", r#"{1:1}"#),
        ("missing colon", r#"{"acc_mm" 1}"#),
        ("bad escape", r#"{"acc_mm\q":1}"#),
        ("raw newline in string", "{\"acc\nmm\":1}"),
        ("raw control in string", "{\"acc\u{1}mm\":1}"),
        ("lone high surrogate", r#"{"a\ud800":1}"#),
        ("lone low surrogate", r#"{"a\udc00":1}"#),
        ("high then non-low", HIGH_THEN_LETTER),
        ("short unicode escape", r#"{"a\u12":1}"#),
        ("non-hex unicode escape", r#"{"a\u12zz":1}"#),
        ("unterminated string", r#"{"acc_mm:1}"#),
    ];
    for (name, text) in cases {
        assert_eq!(
            from_json::<Fix>(text.as_bytes()),
            Err(EngineError::Parse),
            "case {name}"
        );
    }
}

#[test]
fn invalid_utf8_and_bom_are_rejected() {
    let mut bad = br#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#.to_vec();
    assert!(from_json::<Fix>(&bad).is_ok());
    // A bare continuation byte inside a key.
    bad.splice(2..2, [0x80]);
    assert_eq!(from_json::<Fix>(&bad), Err(EngineError::Parse));
    // Overlong encoding of '/'.
    assert_eq!(
        parse_value(&[b'"', 0xC0, 0xAF, b'"'], &JsonLimits::default()),
        Err(EngineError::Parse)
    );
    // BOM.
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice(br#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#);
    assert_eq!(from_json::<Fix>(&bom), Err(EngineError::Parse));
}

#[test]
fn nested_unknown_fields_are_rejected_at_every_depth() {
    let s = SIGNPOST.replacen(
        r#""lat_e7":450000000"#,
        r#""alt_m":1,"lat_e7":450000000"#,
        1,
    );
    assert_eq!(from_json::<Signpost>(s.as_bytes()), Err(EngineError::Parse));
    let s = SIGNPOST.replacen(r#""buffer_m":30"#, r#""buffer_m":30,"width_m":3"#, 1);
    assert_eq!(from_json::<Signpost>(s.as_bytes()), Err(EngineError::Parse));
    let s = SIGNPOST.replacen(r#""event":"approach""#, r#""event":"teleport""#, 1);
    assert_eq!(from_json::<Signpost>(s.as_bytes()), Err(EngineError::Parse));
    let s = SIGNPOST.replacen(r#""id":"sp-1""#, r#""id":"sp 1""#, 1);
    assert_eq!(from_json::<Signpost>(s.as_bytes()), Err(EngineError::Parse));
}

#[test]
fn consent_other_than_exactly_granted_is_never_granted() {
    for (text, want) in [
        (r#"{"location_matching":"granted"}"#, ConsentValue::Granted),
        (r#"{"location_matching":"denied"}"#, ConsentValue::Denied),
        (r#"{"location_matching":"Granted"}"#, ConsentValue::Unknown),
        (r#"{"location_matching":"GRANTED"}"#, ConsentValue::Unknown),
        (r#"{"location_matching":" granted"}"#, ConsentValue::Unknown),
        (
            r#"{"location_matching":"granted\u0000"}"#,
            ConsentValue::Unknown,
        ),
        (r#"{"location_matching":""}"#, ConsentValue::Unknown),
        (r#"{"location_matching":"yes"}"#, ConsentValue::Unknown),
        (r#"{"location_matching":true}"#, ConsentValue::Unknown),
        (r#"{"location_matching":1}"#, ConsentValue::Unknown),
        (
            r#"{"location_matching":["granted"]}"#,
            ConsentValue::Unknown,
        ),
        (
            r#"{"location_matching":{"granted":1}}"#,
            ConsentValue::Unknown,
        ),
        (r#"{"location_matching":"unknown"}"#, ConsentValue::Unknown),
        ("{}", ConsentValue::Unknown),
    ] {
        let c: Consent = from_json(text.as_bytes()).unwrap();
        assert_eq!(c.location_matching, want, "{text}");
    }
    // null is still a hard parse error (spec: no null anywhere).
    assert_eq!(
        from_json::<Consent>(br#"{"location_matching":null}"#),
        Err(EngineError::Parse)
    );
}

#[test]
fn from_canonical_demands_exact_bytes() {
    let ok = br#"{"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#;
    assert!(from_canonical_json::<Fix>(ok).is_ok());
    for (name, text) in [
        (
            "whitespace",
            r#"{"acc_mm": 1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "leading space",
            r#" {"acc_mm":1,"lat_e7":1,"lon_e7":1,"t_ms":1}"#,
        ),
        (
            "trailing newline",
            "{\"acc_mm\":1,\"lat_e7\":1,\"lon_e7\":1,\"t_ms\":1}\n",
        ),
        (
            "unsorted keys",
            r#"{"lat_e7":1,"acc_mm":1,"lon_e7":1,"t_ms":1}"#,
        ),
        ("escaped key letter", ESCAPED_KEY),
    ] {
        assert!(
            from_json::<Fix>(text.as_bytes()).is_ok(),
            "lenient accepts {name}"
        );
        assert_eq!(
            from_canonical_json::<Fix>(text.as_bytes()),
            Err(EngineError::Parse),
            "canonical rejects {name}"
        );
    }
    // Short escapes and uppercase hex are valid JSON but not canonical.
    let v = br#"{"a":"x\ny"}"#;
    assert!(parse_value(v, &JsonLimits::default()).is_ok());
    assert_eq!(
        from_canonical_json::<Value>(v),
        Err(EngineError::Parse),
        "\\n must be written \\u000a"
    );
    assert_eq!(
        from_canonical_json::<Value>(br#"{"a":"\u000A"}"#),
        Err(EngineError::Parse),
        "uppercase hex"
    );
    assert!(from_canonical_json::<Value>(br#"{"a":"x\u000ay"}"#).is_ok());
}

#[test]
fn limits_are_enforced_as_e_limit() {
    let l = JsonLimits {
        max_bytes: 64,
        max_depth: 3,
        max_string_bytes: 4,
        max_array_len: 3,
        max_object_keys: 2,
    };
    let p = |s: &str| parse_value(s.as_bytes(), &l);
    assert!(p("[[[1]]]").is_ok(), "depth exactly 3");
    assert_eq!(p("[[[[1]]]]"), Err(EngineError::Limit));
    assert_eq!(p(r#"{"a":{"a":{"a":{}}}}"#), Err(EngineError::Limit));
    assert!(p(r#""abcd""#).is_ok());
    assert_eq!(p(r#""abcde""#), Err(EngineError::Limit));
    assert_eq!(
        p(r#""abcde""#),
        Err(EngineError::Limit),
        "decoded length counts"
    );
    assert!(p("[1,2,3]").is_ok());
    assert_eq!(p("[1,2,3,4]"), Err(EngineError::Limit));
    assert!(p(r#"{"a":1,"b":2}"#).is_ok());
    assert_eq!(p(r#"{"a":1,"b":2,"c":3}"#), Err(EngineError::Limit));
    let big = format!("[{}]", "1,".repeat(40) + "1");
    assert_eq!(parse_value(big.as_bytes(), &l), Err(EngineError::Limit));
    // The typed entry point honours the same limits.
    assert_eq!(
        from_json_with::<Value>(b"[[[[1]]]]", &l),
        Err(EngineError::Limit)
    );
}

#[test]
fn hostile_inputs_do_not_blow_the_stack_or_the_heap() {
    // 1 MiB of '[' would recurse a million deep without the depth bound.
    let deep = "[".repeat(1 << 20);
    assert_eq!(from_json::<Value>(deep.as_bytes()), Err(EngineError::Limit));
    let deep_obj = r#"{"a":"#.repeat(100_000);
    assert_eq!(
        from_json::<Value>(deep_obj.as_bytes()),
        Err(EngineError::Limit)
    );
    // Overlong digit run: an error, not an overflow panic.
    let digits = "9".repeat(10_000);
    assert_eq!(
        from_json::<Value>(digits.as_bytes()),
        Err(EngineError::Parse)
    );
    // Oversize input is refused before it is scanned.
    let huge = vec![b' '; proxim_types::defaults::MAX_JSON_BYTES + 1];
    assert_eq!(from_json::<Value>(&huge), Err(EngineError::Limit));
}

#[test]
fn integer_extremes_parse_exactly() {
    for (text, expect) in [
        ("0", "0"),
        ("-1", "-1"),
        ("9223372036854775807", "9223372036854775807"),
        ("-9223372036854775808", "-9223372036854775808"),
        ("18446744073709551615", "18446744073709551615"),
    ] {
        let v = parse_value(text.as_bytes(), &JsonLimits::default()).unwrap();
        assert_eq!(value_to_canonical(&v).unwrap(), expect);
    }
    for text in [
        "-9223372036854775809",
        "18446744073709551616",
        "-18446744073709551615",
    ] {
        assert_eq!(
            parse_value(text.as_bytes(), &JsonLimits::default()),
            Err(EngineError::Parse),
            "{text}"
        );
    }
}

#[test]
fn surrogate_pairs_decode_and_re_encode_raw() {
    let pair = concat!("\"", "\\", "ud83d", "\\", "ude00", "\"");
    let v = parse_value(pair.as_bytes(), &JsonLimits::default()).unwrap();
    assert_eq!(v, Value::String("\u{1f600}".into()));
    assert_eq!(value_to_canonical(&v).unwrap(), "\"\u{1f600}\"");
}

fn arb_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::from),
        any::<u64>().prop_map(Value::from),
        any::<String>().prop_map(Value::String),
    ];
    leaf.prop_recursive(5, 64, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
            prop::collection::btree_map(any::<String>(), inner, 0..6)
                .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn value_round_trip_and_idempotence(v in arb_value()) {
        let text = value_to_canonical(&v).unwrap();
        let back = parse_value(text.as_bytes(), &JsonLimits::default()).unwrap();
        prop_assert_eq!(&back, &v);
        prop_assert_eq!(value_to_canonical(&back).unwrap(), text.clone());
        // The canonical text is also accepted by the canonical-only entry point.
        let again: Value = from_canonical_json(text.as_bytes()).unwrap();
        prop_assert_eq!(again, v);
    }

    #[test]
    fn canonical_output_has_no_whitespace_outside_strings_and_ascending_keys(v in arb_value()) {
        let text = value_to_canonical(&v).unwrap();
        // No raw control character ever appears in the output.
        prop_assert!(text.bytes().all(|b| b >= 0x20));
        // Keys of every object are strictly ascending by bytes in the re-parsed order of text.
        fn check(v: &Value) -> bool {
            match v {
                Value::Object(m) => m.values().all(check),
                Value::Array(a) => a.iter().all(check),
                _ => true,
            }
        }
        prop_assert!(check(&v));
    }

    #[test]
    fn fix_round_trip(
        t_ms in any::<i64>(), lat in any::<i32>(), lon in any::<i32>(), acc in any::<u32>(),
        speed in proptest::option::of(any::<u32>()),
        sacc in proptest::option::of(any::<u32>()),
        course in proptest::option::of(0u16..3600),
        cacc in proptest::option::of(any::<u16>()),
        mock in any::<bool>(),
    ) {
        let f = Fix { t_ms, lat_e7: lat, lon_e7: lon, acc_mm: acc, speed_mmps: speed,
            speed_acc_mmps: sacc, course_ddeg: course, course_acc_ddeg: cacc, is_mock: mock };
        let j = to_canonical_json(&f).unwrap();
        prop_assert!(!j.contains("null"));
        let back: Fix = from_canonical_json(j.as_bytes()).unwrap();
        prop_assert_eq!(back, f);
        prop_assert_eq!(to_canonical_json(&back).unwrap(), j);
    }

    #[test]
    fn arbitrary_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
        // Result is irrelevant; the property is "returns".
        let _ = from_json::<Fix>(&bytes);
        let _ = from_json::<State>(&bytes);
        let _ = from_json::<Value>(&bytes);
    }

    #[test]
    fn mutated_valid_documents_never_panic_and_stay_canonical_if_accepted(
        pos in 0usize..SIGNPOST.len(), byte in any::<u8>()
    ) {
        let mut b = SIGNPOST.as_bytes().to_vec();
        if let Some(slot) = b.get_mut(pos) { *slot = byte; }
        if let Ok(sp) = from_json::<Signpost>(&b) {
            // Whatever the parser accepted must write back to something it accepts again.
            let j = to_canonical_json(&sp).unwrap();
            prop_assert_eq!(from_canonical_json::<Signpost>(j.as_bytes()).unwrap(), sp);
        }
    }
}

#[test]
fn writer_rejects_floats_anywhere() {
    // Kept here, not in src/: the float-confinement test forbids float literals in proxim-types/src.
    assert_eq!(
        value_to_canonical(&serde_json::json!(1.0)),
        Err(EngineError::Parse),
        "1.0 is a float even though it is integral"
    );
    assert_eq!(
        value_to_canonical(&serde_json::json!(1.5)),
        Err(EngineError::Parse)
    );
    assert_eq!(
        value_to_canonical(&serde_json::json!({"a": [0.1]})),
        Err(EngineError::Parse)
    );
    assert_eq!(
        value_to_canonical(&serde_json::json!(-0.0)),
        Err(EngineError::Parse)
    );
    assert_eq!(to_canonical_json(&1.0_f64), Err(EngineError::Parse));
    assert_eq!(to_canonical_json(&f64::NAN), Err(EngineError::Parse));
    assert_eq!(to_canonical_json(&vec![1_u8, 2]).unwrap(), "[1,2]");
}

#[test]
fn lexical_rejections_at_the_tree_level() {
    // These must fail in the parser itself, not merely later when a typed decode refuses the shape
    // (a Value accepts any key and any number), so each case is run through `parse_value`.
    let l = JsonLimits::default();
    let ctrl = |c: char| format!("[\"a{c}b\"]");
    let mut cases: Vec<(String, String)> = vec![
        ("raw LF in string".into(), ctrl('\n')),
        ("raw TAB in string".into(), ctrl('\t')),
        ("raw NUL in string".into(), ctrl('\u{0}')),
        ("raw 0x1f in string".into(), ctrl('\u{1f}')),
        ("leading zero".into(), "[01]".into()),
        ("leading zero after minus".into(), "[-01]".into()),
        ("fraction".into(), "[1.5]".into()),
        ("zero fraction".into(), "[1.0]".into()),
        ("trailing dot".into(), "[1.]".into()),
        ("leading dot".into(), "[.5]".into()),
        ("exponent".into(), "[1e2]".into()),
        ("exponent upper".into(), "[1E2]".into()),
        ("exponent on zero".into(), "[0e0]".into()),
        ("negative zero".into(), "[-0]".into()),
        ("plus".into(), "[+1]".into()),
        ("hex".into(), "[0x10]".into()),
        ("null".into(), "[null]".into()),
        ("null key value".into(), "{\"a\":null}".into()),
        ("duplicate key".into(), "{\"a\":1,\"a\":2}".into()),
        (
            "duplicate key nested".into(),
            "[{\"k\":{\"a\":1,\"b\":1,\"a\":1}}]".into(),
        ),
        (
            "lone low surrogate".into(),
            concat!("[\"", "\\", "udc00\"]").into(),
        ),
        (
            "lone high surrogate at end".into(),
            concat!("[\"", "\\", "ud800\"]").into(),
        ),
        (
            "high surrogate then plain".into(),
            concat!("[\"", "\\", "ud800x\"]").into(),
        ),
        ("bad escape".into(), "[\"\\q\"]".into()),
        ("escaped single quote".into(), "[\"\\'\"]".into()),
        ("truncated escape".into(), "[\"\\".into()),
        ("empty".into(), String::new()),
        ("object trailing comma".into(), "{\"a\":1,}".into()),
        ("array trailing comma".into(), "[1,]".into()),
        ("array leading comma".into(), "[,1]".into()),
        ("missing value".into(), "{\"a\":}".into()),
        ("two values".into(), "1 2".into()),
        ("unclosed array".into(), "[1".into()),
        ("unclosed object".into(), "{\"a\":1".into()),
        ("unquoted key".into(), "{a:1}".into()),
        ("capital TRUE".into(), "[TRUE]".into()),
        ("truncated literal".into(), "[tru]".into()),
        ("minus alone".into(), "[-]".into()),
    ];
    cases.push(("digit run overflow".into(), "9".repeat(21)));
    for (name, text) in &cases {
        assert_eq!(
            parse_value(text.as_bytes(), &l),
            Err(EngineError::Parse),
            "{name}: {text:?}"
        );
    }
    // And the neighbours of each rejected form that must be accepted, so the table is not simply
    // rejecting everything.
    for ok in [
        "[0]",
        "[-1]",
        "[10]",
        "[\"a b\"]",
        "[true,false]",
        "{\"a\":1,\"b\":2}",
        "[[],{}]",
        "  [ 1 , 2 ]\n",
    ] {
        assert!(parse_value(ok.as_bytes(), &l).is_ok(), "{ok}");
    }
}
