//! Structure-aware fuzzing of `decide_json` (never panic, never hang, always canonical).
//!
//! Inputs are built from a rich, valid world (every field of spec 3.3, every detector state of 3.3,
//! a full trace) and then mutated: deleted, duplicated or extra keys, wrong types, extreme and
//! non-canonical numbers, deep nesting, huge strings and arrays, truncation and byte flips.
//! Everything is driven by a seeded [`Rng`]; a failing case prints its seed and the six inputs.

// The watchdog below is the one place a wall clock is used: it only ABORTS a hung call, it never
// influences an expected value, so test outcomes stay deterministic.
#![allow(clippy::disallowed_types)]

use crate::canon::{check_canonical, render};
use crate::rng::Rng;
use crate::world::*;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Marker that makes [`render_raw`] splice the rest of the string into the output unquoted.
pub const RAW: &str = "\u{0}RAW\u{0}";

pub fn raw(tok: &str) -> Value {
    Value::String(format!("{RAW}{tok}"))
}

/// The six textual arguments of `decide_json` (plus the seed).
#[derive(Clone, Debug)]
pub struct Inputs {
    pub args: [String; 6],
    pub seed: u64,
}

pub const ARG_NAMES: [&str; 6] = ["state", "fix", "signposts", "policy", "consent", "clock"];
pub const ERROR_CODES: [&str; 6] = [
    "E_PARSE",
    "E_OVERFLOW",
    "E_OUT_OF_FRAME",
    "E_UNSUPPORTED_LATITUDE",
    "E_DEGENERATE_GEOMETRY",
    "E_LIMIT",
];

impl Inputs {
    pub fn call(&self) -> String {
        proxim_ffi::decide_json(
            &self.args[0],
            &self.args[1],
            &self.args[2],
            &self.args[3],
            &self.args[4],
            &self.args[5],
            self.seed,
        )
    }
    pub fn describe(&self) -> String {
        let mut s = format!("seed={}\n", self.seed);
        for (n, a) in ARG_NAMES.iter().zip(&self.args) {
            let cut: String = a.chars().take(300).collect();
            s.push_str(&format!("  {n} ({} bytes): {cut:?}\n", a.len()));
        }
        s
    }
}

// ---------------------------------------------------------------------------------------------
// A rich valid world
// ---------------------------------------------------------------------------------------------

/// Every field of the wire types, every detector variant.
pub fn rich_world() -> [Value; 6] {
    let t = BASE_T + 600_000;
    let mut circle = enter_circle("c-1", 200);
    circle["rank_milli"] = json!(1200);
    circle["min_path_tier"] = json!(0);
    circle["min_path_conf_milli"] = json!(500);
    set(&mut circle, "trigger.reach_m", json!(5000));
    set(&mut circle, "trigger.max_event_age_s", json!(30));
    set(&mut circle, "trigger.max_event_dist_m", json!(5000));
    circle["intent"] = json!("promote");
    circle["creatives"] = json!(["a", "b"]);
    let mut poly = enter_circle("p-1", 200);
    let sq = [(-250, -250), (250, -250), (250, 250), (-250, 250)]
        .map(|(e, n)| pos(offset(ORIGIN, e * 1000, n * 1000)));
    set(
        &mut poly,
        "trigger.shape",
        json!({"kind": "polygon", "vertices": sq, "inradius_m": 200}),
    );
    let mut route = approach_route("r-1", 2500);
    set(&mut route, "trigger.d_lo_m", json!(150));
    set(&mut route, "trigger.d_hi_m", json!(3000));
    set(&mut route, "trigger.speed_min_mmps", json!(2500));
    set(&mut route, "trigger.speed_max_mmps", json!(40_000));
    set(&mut route, "trigger.bearing_tol_ddeg", json!(300));
    set(&mut route, "trigger.shape.anchor_along_m", json!(2500));
    let mut health = enter_circle_at("h-1", offset(ORIGIN, 3_000_000, 0), 300);
    health["class"] = json!("health");
    let mut dwell = enter_circle("d-1", 200);
    dwell["event"] = json!("dwell");
    let signposts = json!([circle, poly, route, health, dwell]);

    let trace: Vec<Value> = (0..16)
        .map(|i| {
            let p = offset(ORIGIN, i * 14_000, -1_500_000 + i * 14_000);
            json!({"t_ms": t - (16 - i) * 1000, "lat_e7": p.0, "lon_e7": p.1, "acc_mm": 5000, "v_mmps": 15_000})
        })
        .collect();
    let state = json!({
        "v": 1,
        "caps": {
            "hwm_ms": t - 1000,
            "last_fire_ms": t - 3_600_000,
            "signposts": [
                {"id": "c-1", "day_idx": day_idx(t, 0), "day_n": 1, "week_idx": week_idx(t, 0), "week_n": 3},
                {"id": "r-1", "day_idx": day_idx(t, 0) - 1, "day_n": 2, "week_idx": week_idx(t, 0), "week_n": 5}
            ]
        },
        "mem": {
            "last_t_ms": t - 1000,
            "jump_rejects": 1,
            "trace": trace,
            "vf_mmps": 15_000,
            "h": [0, 1_073_741_824],
            "h_t_ms": t - 1000,
            "det": [
                {"id": "c-1", "rev": 1, "st": {"s": "cand", "n": 2, "first_t": t - 3000, "last_t": t - 1000}},
                {"id": "d-1", "rev": 1, "st": {"s": "in", "xn": 1, "x_first_t": t - 2000, "last_t": t - 1000}},
                {"id": "h-1", "rev": 1, "st": {"s": "cool", "until_ms": t + 1_000_000}},
                {"id": "p-1", "rev": 1, "st": {"s": "latched", "why": "fired", "last_t": t - 1000}},
                {"id": "r-1", "rev": 1, "st": {"s": "armed", "confirm_t": t - 2000, "last_t": t - 1000, "a_prev_mm": 1_200_000}}
            ]
        }
    });
    let here = offset(
        offset(ORIGIN, 15 * 14_000, -1_500_000 + 15 * 14_000),
        0,
        15_000,
    );
    let fix = json!({
        "t_ms": t, "lat_e7": here.0, "lon_e7": here.1, "acc_mm": 8000, "speed_mmps": 14_000,
        "speed_acc_mmps": 900, "course_ddeg": 10, "course_acc_ddeg": 50, "is_mock": false
    });
    let policy = json!({
        "simulator_mode": false, "child_directed": false, "explain": true,
        "categories_blocked": ["bar"], "senders_blocked": ["sender-9"], "sensitivity_ceiling": 2,
        "lead_s_min": 20, "lead_s_max": 100, "cap_signpost_day": 2, "cap_signpost_week": 5,
        "app_min_interval_s": 1800, "slots": 1, "max_acc_mm": 100_000, "rot_lane": 1,
        "standoff_zones": [{"id": "z-1", "center": pos(offset(ORIGIN, 8_000_000, 8_000_000)), "standoff_m": 1500}]
    });
    let consent = json!({
        "location_matching": "granted", "nearby_health_events": "granted",
        "aggregate_reporting": "denied", "local_notifications": "unknown", "health_receipt": "receipt-1"
    });
    let clock = json!({"now_ms": t, "tz_offset_min": 60});
    [state, fix, signposts, policy, consent, clock]
}

pub fn base_inputs() -> Inputs {
    let w = rich_world();
    Inputs {
        args: [0, 1, 2, 3, 4, 5].map(|i| render(&w[i])),
        seed: 0,
    }
}

// ---------------------------------------------------------------------------------------------
// Rendering with raw tokens and duplicate keys
// ---------------------------------------------------------------------------------------------

/// Render like the canonical writer, but strings starting with [`RAW`] are spliced in unquoted and
/// (with `dup`) objects may emit one entry twice.
pub fn render_raw(v: &Value, dup: &mut Option<Rng>) -> String {
    let mut s = String::new();
    go(v, dup, &mut s);
    s
}

fn go(v: &Value, dup: &mut Option<Rng>, out: &mut String) {
    match v {
        Value::String(s) if s.starts_with(RAW) => out.push_str(&s[RAW.len()..]),
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            out.push('{');
            let mut first = true;
            for k in keys {
                let times = match dup.as_mut() {
                    Some(r) => {
                        if r.chance(1, 25) {
                            2
                        } else {
                            1
                        }
                    }
                    None => 1,
                };
                for _ in 0..times {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    out.push_str(&render(&Value::String(k.clone())));
                    out.push(':');
                    go(&m[k], dup, out);
                }
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                go(x, dup, out);
            }
            out.push(']');
        }
        other => {
            // numbers, bools, normal strings: canonical spelling (floats cannot occur: use `raw`)
            out.push_str(&render(other));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Mutations
// ---------------------------------------------------------------------------------------------

const NASTY_NUMBER_TOKENS: &[&str] = &[
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "18446744073709551615",
    "18446744073709551616",
    "2147483647",
    "2147483648",
    "-2147483648",
    "-2147483649",
    "4294967295",
    "4294967296",
    "65535",
    "65536",
    "32767",
    "32768",
    "-32768",
    "-32769",
    "255",
    "256",
    "-1",
    "0",
    "-0",
    "1e999",
    "1E5",
    "1e-9",
    "1.5",
    "0.0",
    "-0.0",
    ".5",
    "5.",
    "+1",
    "00",
    "01",
    "NaN",
    "Infinity",
    "-Infinity",
    "nan",
    "inf",
    "0x10",
    "1_000",
    "1,5",
    "99999999999999999999999999999999999999999999",
    "-99999999999999999999999999999999999999999999",
    "1e400",
    "123456789012345678901234567890.123456789e50",
    "null",
    "true",
    "false",
    "\"\"",
    "{}",
    "[]",
];

const EXTREME_INTS: &[i64] = &[
    i64::MIN,
    i64::MIN + 1,
    i64::MAX,
    i64::MAX - 1,
    i32::MIN as i64,
    i32::MIN as i64 - 1,
    i32::MAX as i64,
    i32::MAX as i64 + 1,
    u32::MAX as i64,
    u32::MAX as i64 + 1,
    i16::MIN as i64,
    i16::MAX as i64,
    65_535,
    65_536,
    255,
    256,
    -1,
    0,
    1,
    3599,
    3600,
    3601,
    900_000_000,
    900_000_001,
    1_800_000_000,
    1_800_000_001,
    -900_000_001,
    -1_800_000_001,
    1 << 62,
    -(1 << 62),
    1 << 53,
    (1 << 53) + 1,
    86_400_000,
    -86_400_000,
    1_800_000,
    5000,
    30_000,
    100_000,
    150_000_000,
    150_000_001,
];

const STRANGE_STRINGS: &[&str] = &[
    "",
    " ",
    "\u{0}",
    "\u{1f}",
    "\u{7f}",
    "\u{80}",
    "\u{ff}",
    "\u{ffff}",
    "\u{10ffff}",
    "\u{feff}",
    "\u{202e}",
    "\u{200b}",
    "\u{1f600}",
    "\\",
    "\"",
    "'",
    "<script>",
    "%s%n",
    "../../etc/passwd",
    "a b",
    "A",
    "granted",
    "Granted",
    "GRANTED",
    "denied",
    "unknown",
    "enter",
    "approach",
    "exit",
    "circle",
    "polygon",
    "route",
    "health",
    "general",
    "vehicle",
    "foot",
    "cycle",
    "fire",
    "drop",
    "OK",
    "null",
    "true",
    "0",
    "-1",
    "1e5",
    "NaN",
    "\u{e9}",
    "\u{0430}",
    "a\u{300}",
];

const RAW_STRING_TOKENS: &[&str] = &[
    "\"\\ud800\"",
    "\"\\udc00\"",
    "\"\\ud800\\ud800\"",
    "\"\\u0000\"",
    "\"\\u00e9\"",
    "\"\\/\"",
    "\"\\x41\"",
    "\"\\",
    "\"abc",
    "\"\\u12\"",
    "\"\\uZZZZ\"",
    "'single'",
    "\"tab\there\"",
    "\"nl\nhere\"",
    "\"\u{ff}\"",
    "\"\\U0041\"",
    "\"\\u004\"",
];

fn paths(v: &Value, cur: &mut Vec<PathStep>, out: &mut Vec<Vec<PathStep>>) {
    out.push(cur.clone());
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                cur.push(PathStep::Key(k.clone()));
                paths(x, cur, out);
                cur.pop();
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                cur.push(PathStep::Idx(i));
                paths(x, cur, out);
                cur.pop();
            }
        }
        _ => {}
    }
}

#[derive(Clone, Debug)]
enum PathStep {
    Key(String),
    Idx(usize),
}

fn get_mut<'a>(v: &'a mut Value, path: &[PathStep]) -> Option<&'a mut Value> {
    let mut cur = v;
    for s in path {
        cur = match s {
            PathStep::Key(k) => cur.get_mut(k)?,
            PathStep::Idx(i) => cur.get_mut(*i)?,
        };
    }
    Some(cur)
}

fn random_leaf(rng: &mut Rng) -> Value {
    match rng.below(12) {
        0 => raw(rng.pick(NASTY_NUMBER_TOKENS)),
        1 => json!(*rng.pick(EXTREME_INTS)),
        2 => json!(rng.next_u64() as i64),
        3 => json!(*rng.pick(STRANGE_STRINGS)),
        4 => raw(rng.pick(RAW_STRING_TOKENS)),
        5 => json!(rng.chance(1, 2)),
        6 => json!([]),
        7 => json!({}),
        8 => json!([1, 2, 3]),
        9 => json!({"a": 1, "b": [true]}),
        10 => json!(rng.range_i64(-1000, 1000)),
        _ => raw("null"),
    }
}

/// Apply one random structural mutation somewhere in `root`.
pub fn mutate(root: &mut Value, rng: &mut Rng) {
    let mut ps = Vec::new();
    paths(root, &mut Vec::new(), &mut ps);
    let path = rng.pick(&ps).clone();
    let op = rng.below(16);
    // operations on the node itself (replace) vs. on its parent (delete)
    match op {
        0 | 1 => {
            // replace by a random leaf (wrong type / extreme number / strange string)
            if let Some(n) = get_mut(root, &path) {
                *n = random_leaf(rng);
            }
        }
        2 => {
            // delete this node from its parent
            if let Some((last, parent)) = path.split_last()
                && let Some(p) = get_mut(root, parent)
            {
                match (p, last) {
                    (Value::Object(m), PathStep::Key(k)) => {
                        m.remove(k);
                    }
                    (Value::Array(a), PathStep::Idx(i)) if *i < a.len() => {
                        a.remove(*i);
                    }
                    _ => {}
                }
            }
        }
        3 => {
            // add an extra key to the object at this node (or its parent)
            let target = get_mut(root, &path).filter(|n| n.is_object());
            if let Some(Value::Object(m)) = target {
                let key = [
                    "zz",
                    "extra",
                    "lat_e7",
                    "rev",
                    "id",
                    "v",
                    "s",
                    "explain",
                    "seed",
                    "__proto__",
                    "",
                ][rng.below(11) as usize];
                m.insert(key.to_string(), random_leaf(rng));
            }
        }
        4 => {
            // replace by deep nesting of arrays or objects
            let depth = *rng.pick(&[2_usize, 10, 31, 32, 33, 64, 100, 1000, 20_000, 100_000]);
            let tok = if rng.chance(1, 2) {
                format!("{}{}", "[".repeat(depth), "]".repeat(depth))
            } else {
                format!("{}1{}", "{\"a\":".repeat(depth), "}".repeat(depth))
            };
            if let Some(n) = get_mut(root, &path) {
                *n = raw(&tok);
            }
        }
        5 => {
            // replace by a long string
            let len = *rng.pick(&[100_usize, 8191, 8192, 8193, 65_536, 1_000_000]);
            if let Some(n) = get_mut(root, &path) {
                *n = Value::String("a".repeat(len));
            }
        }
        6 => {
            // grow an array to a size at / over the limits
            let len = *rng.pick(&[17_usize, 257, 1025, 4097, 8192, 8193, 20_000]);
            if let Some(Value::Array(a)) = get_mut(root, &path)
                && let Some(first) = a.first().cloned().or_else(|| Some(json!(1)))
            {
                // keep the document under ~16 MiB even when the element is itself large
                let each = render_raw(&first, &mut None).len().max(1);
                a.resize(len.min((16 << 20) / each).max(1), first);
            }
        }
        7 => {
            // nudge a number by a small amount / sign flip / scale
            if let Some(n) = get_mut(root, &path)
                && let Some(i) = n.as_i64()
            {
                *n = match rng.below(4) {
                    0 => json!(i.wrapping_add(1)),
                    1 => json!(i.wrapping_sub(1)),
                    2 => json!(i.wrapping_neg()),
                    _ => json!(i.wrapping_mul(1000)),
                };
            }
        }
        8 => {
            // swap with another node's value
            let other = rng.pick(&ps).clone();
            let a = get_mut(root, &path).map(|n| n.clone());
            let b = get_mut(root, &other).map(|n| n.clone());
            if let (Some(a), Some(b)) = (a, b) {
                if let Some(n) = get_mut(root, &path) {
                    *n = b;
                }
                if let Some(n) = get_mut(root, &other) {
                    *n = a;
                }
            }
        }
        9 => {
            // replace a string with a strange one
            if let Some(n) = get_mut(root, &path)
                && n.is_string()
            {
                *n = json!(*rng.pick(STRANGE_STRINGS));
            }
        }
        10 => {
            // duplicate an array element many times
            if let Some(Value::Array(a)) = get_mut(root, &path)
                && !a.is_empty()
            {
                let e = a[rng.below(a.len() as u64) as usize].clone();
                if render_raw(&e, &mut None).len() < (1 << 20) {
                    for _ in 0..rng.below(5) {
                        a.push(e.clone());
                    }
                }
            }
        }
        11 => {
            // reorder an array (breaks "strictly ascending" state invariants)
            if let Some(Value::Array(a)) = get_mut(root, &path) {
                rng.shuffle(a);
            }
        }
        12 => {
            // empty out
            if let Some(n) = get_mut(root, &path) {
                *n = match n {
                    Value::Array(_) => json!([]),
                    Value::Object(_) => json!({}),
                    Value::String(_) => json!(""),
                    _ => json!(0),
                };
            }
        }
        13 => {
            // extreme integer in place (valid JSON integer: exercises arithmetic)
            if let Some(n) = get_mut(root, &path)
                && n.is_number()
            {
                *n = json!(*rng.pick(EXTREME_INTS));
            }
        }
        14 => {
            // type confusion between siblings: object <-> array
            if let Some(n) = get_mut(root, &path) {
                *n = match n.clone() {
                    Value::Object(m) => Value::Array(m.values().cloned().collect()),
                    Value::Array(a) => Value::Object(
                        a.into_iter()
                            .enumerate()
                            .map(|(i, v)| (i.to_string(), v))
                            .collect(),
                    ),
                    other => json!([other]),
                };
            }
        }
        _ => {
            if let Some(n) = get_mut(root, &path) {
                *n = random_leaf(rng);
            }
        }
    }
}

/// Text-level damage: truncation, byte flips, BOM/whitespace, lossy decoding of invalid UTF-8.
pub fn damage_text(text: &str, rng: &mut Rng) -> String {
    let mut bytes = text.as_bytes().to_vec();
    match rng.below(7) {
        0 if !bytes.is_empty() => {
            let cut = rng.below(bytes.len() as u64) as usize;
            bytes.truncate(cut);
        }
        1 if !bytes.is_empty() => {
            for _ in 0..=rng.below(4) {
                let i = rng.below(bytes.len() as u64) as usize;
                bytes[i] ^= 1 << rng.below(8);
            }
        }
        2 if !bytes.is_empty() => {
            let i = rng.below(bytes.len() as u64) as usize;
            bytes.insert(
                i,
                *rng.pick(&[
                    0u8, 0xff, 0xc0, 0x80, b'"', b'\\', b'{', b'}', b'[', b',', b':',
                ]),
            );
        }
        3 => {
            let mut v = vec![0xef, 0xbb, 0xbf];
            v.extend_from_slice(&bytes);
            bytes = v;
        }
        4 => {
            let ws = *rng.pick(&[" ", "\n", "\t\r\n", "\u{a0}", "\u{2028}"]);
            let mut v = ws.as_bytes().to_vec();
            v.extend_from_slice(&bytes);
            v.extend_from_slice(ws.as_bytes());
            bytes = v;
        }
        5 if !bytes.is_empty() => {
            let i = rng.below(bytes.len() as u64) as usize;
            bytes.drain(i..(i + 1 + rng.below(8) as usize).min(bytes.len()));
        }
        _ => bytes.extend_from_slice(b"\n{}"),
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A random JSON-flavoured string: tokens glued together, valid UTF-8, often nearly valid JSON.
pub fn token_soup(rng: &mut Rng) -> String {
    const TOK: &[&str] = &[
        "{",
        "}",
        "[",
        "]",
        ",",
        ":",
        "\"",
        "\\",
        "\"a\"",
        "\"lat_e7\"",
        "\"id\"",
        "\"v\"",
        "1",
        "-1",
        "0",
        "9223372036854775807",
        "1.5",
        "1e5",
        "true",
        "false",
        "null",
        "NaN",
        " ",
        "\n",
        "\\u00",
        "\\n",
        "\u{e9}",
        "\"caps\"",
        "\"mem\"",
        "\"t_ms\"",
        "\"trigger\"",
        "\"shape\"",
        "\"kind\"",
        "\"circle\"",
        "\"granted\"",
        "{\"v\":1}",
        "{\"caps\":{}}",
        "[{}]",
        "[[[[",
        "]]]]",
        "\"\\u0000\"",
        "\u{0}",
        "\u{feff}",
    ];
    let n = rng.below(40);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(rng.pick(TOK));
    }
    s
}

/// One fuzz case: mutate 1-3 arguments of the rich world, sometimes damaging the text.
pub fn make_case(seed: u64) -> Inputs {
    let mut rng = Rng::new(seed);
    let mut w = rich_world();
    let which: Vec<usize> = (0..=rng.below(3)).map(|_| rng.below(6) as usize).collect();
    let mut texts: Vec<Option<String>> = vec![None; 6];
    for i in which {
        let kind = rng.below(10);
        if kind == 0 {
            texts[i] = Some(token_soup(&mut rng));
            continue;
        }
        if kind == 1 {
            texts[i] = Some(damage_text(&render(&w[i]), &mut rng));
            continue;
        }
        for _ in 0..=rng.below(3) {
            mutate(&mut w[i], &mut rng);
        }
        let mut dup = if rng.chance(1, 8) {
            Some(Rng::new(rng.next_u64()))
        } else {
            None
        };
        let mut t = render_raw(&w[i], &mut dup);
        if rng.chance(1, 10) {
            t = damage_text(&t, &mut rng);
        }
        texts[i] = Some(t);
    }
    let args = [0, 1, 2, 3, 4, 5].map(|i| {
        texts[i]
            .clone()
            .unwrap_or_else(|| render_raw(&w[i], &mut None))
    });
    Inputs {
        args,
        seed: rng.next_u64(),
    }
}

// ---------------------------------------------------------------------------------------------
// Response checking
// ---------------------------------------------------------------------------------------------

/// What kind of response came back.
#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    Error,
    Success,
}

/// Verify a response is one of the two canonical shapes. Returns the kind or the violation.
pub fn check_response(text: &str) -> Result<Kind, String> {
    check_canonical(text).map_err(|e| format!("non-canonical response ({e})"))?;
    let v: Value = serde_json::from_str(text).map_err(|e| format!("response is not JSON: {e}"))?;
    let obj = v.as_object().ok_or("response is not an object")?;
    if let Some(e) = obj.get("error") {
        if obj.len() != 1 {
            return Err("error response has extra members".into());
        }
        let e = e.as_object().ok_or("error is not an object")?;
        let code = e
            .get("code")
            .and_then(Value::as_str)
            .ok_or("error without code")?;
        if !ERROR_CODES.contains(&code) {
            return Err(format!("unknown error code {code}"));
        }
        for (k, val) in e {
            match k.as_str() {
                "code" => {}
                "input" => {
                    let n = val.as_str().ok_or("input is not a string")?;
                    if !ARG_NAMES.contains(&n) {
                        return Err(format!("unknown input name {n}"));
                    }
                }
                other => return Err(format!("unexpected error member {other}")),
            }
        }
        return Ok(Kind::Error);
    }
    if obj.len() != 2 || !obj.contains_key("output") || !obj.contains_key("state") {
        return Err("success response must be exactly {output,state}".into());
    }
    let out = &obj["output"];
    if out["v"] != 1
        || !out["decisions"].is_array()
        || !out["fix"]["ok"].is_boolean()
        || !out["fix"]["reason"].is_string()
    {
        return Err("malformed output".into());
    }
    if out.get("notes").is_some_and(|n| !n.is_array()) {
        return Err("malformed notes".into());
    }
    if obj["state"]["v"] != 1 {
        return Err("state.v != 1".into());
    }
    Ok(Kind::Success)
}

// ---------------------------------------------------------------------------------------------
// Runner with a watchdog and a small stack
// ---------------------------------------------------------------------------------------------

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub cases: u64,
    pub errors: u64,
    pub successes: u64,
    pub panics: u64,
    pub hangs: u64,
    pub non_canonical: u64,
    pub slowest_ms: u128,
    pub reingest_failures: u64,
    pub failures: Vec<String>,
}

/// Run `n` cases from `first_seed`, on a thread with `stack_bytes` of stack. A watchdog aborts the
/// process with the offending case if one call exceeds `hang_limit` (a hang cannot be caught).
pub fn run_cases(
    first_seed: u64,
    n: u64,
    stack_bytes: usize,
    hang_limit: Duration,
    make: fn(u64) -> Inputs,
) -> Stats {
    let beat = Arc::new(AtomicU64::new(0));
    let current = Arc::new(Mutex::new(String::new()));
    let done = Arc::new(AtomicU64::new(0));
    {
        let (beat, current, done) = (beat.clone(), current.clone(), done.clone());
        std::thread::spawn(move || {
            let mut last = (beat.load(Ordering::SeqCst), Instant::now());
            while done.load(Ordering::SeqCst) == 0 {
                std::thread::sleep(Duration::from_millis(250));
                let b = beat.load(Ordering::SeqCst);
                if b != last.0 {
                    last = (b, Instant::now());
                } else if last.1.elapsed() > hang_limit {
                    eprintln!(
                        "HANG: decide_json did not return within {hang_limit:?}\n{}",
                        current.lock().map(|g| g.clone()).unwrap_or_default()
                    );
                    std::process::abort();
                }
            }
        });
    }
    let (beat2, current2, done2) = (beat.clone(), current.clone(), done.clone());
    let handle = std::thread::Builder::new()
        .stack_size(stack_bytes)
        .spawn(move || {
            let mut st = Stats::default();
            for i in 0..n {
                let seed = first_seed
                    .wrapping_add(i)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    ^ i;
                let inp = make(seed);
                if let Ok(mut g) = current2.lock() {
                    *g = inp.describe();
                }
                beat2.fetch_add(1, Ordering::SeqCst);
                let t0 = Instant::now();
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inp.call()));
                let ms = t0.elapsed().as_millis();
                st.slowest_ms = st.slowest_ms.max(ms);
                st.cases += 1;
                match res {
                    Err(_) => {
                        st.panics += 1;
                        st.failures.push(format!("PANIC\n{}", inp.describe()));
                    }
                    Ok(text) => match check_response(&text) {
                        Err(why) => {
                            st.non_canonical += 1;
                            st.failures.push(format!(
                                "{why}\nresponse: {}\n{}",
                                &text[..text.len().min(300)],
                                inp.describe()
                            ));
                        }
                        Ok(Kind::Error) => st.errors += 1,
                        Ok(Kind::Success) => {
                            st.successes += 1;
                            // the returned state must be re-ingestible with the same inputs
                            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
                            let mut again = inp.clone();
                            again.args[0] = render(&v["state"]);
                            let r2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                again.call()
                            }));
                            match r2 {
                                Err(_) => {
                                    st.panics += 1;
                                    st.failures
                                        .push(format!("PANIC on re-ingest\n{}", inp.describe()));
                                }
                                Ok(t2) => match check_response(&t2) {
                                    Err(why) => {
                                        st.non_canonical += 1;
                                        st.failures
                                            .push(format!("{why} (re-ingest)\n{}", inp.describe()));
                                    }
                                    Ok(Kind::Error) if t2.contains("\"input\":\"state\"") => {
                                        st.reingest_failures += 1;
                                        st.failures.push(format!(
                                            "returned state not accepted back: {}\n{}",
                                            &t2[..t2.len().min(200)],
                                            inp.describe()
                                        ));
                                    }
                                    Ok(_) => {}
                                },
                            }
                        }
                    },
                }
            }
            st
        })
        .expect("spawn fuzz thread");
    let st = handle.join().unwrap_or_else(|_| {
        let mut s = Stats::default();
        s.panics += 1;
        s.failures
            .push("fuzz thread died (panic outside catch_unwind or stack overflow)".into());
        s
    });
    done2.store(1, Ordering::SeqCst);
    st
}
