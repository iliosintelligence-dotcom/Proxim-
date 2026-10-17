//! OI-ROT-1 (Addendum 02 13): the Rust port of `select_variant` and the pending window reproduces
//! the frozen reference vectors `core/testdata/golden/rotation/golden_v2.json` and the supplement
//! `golden_v2_newcount.json`, both made by `tools/rotation-ref/refkernel.py`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::panic
)]

use proxim_decide::rotation::{self, Family, Ledger, Pending};
use proxim_decide::sha256::{hex, sha256};
use serde_json::{Value, json};
use std::path::PathBuf;

/// SHA-256 of golden_v2.json as committed. The file is never regenerated (`golden-regen-diff`).
const GOLDEN_V2_SHA256: &str = "3a57baae90196f27299c85e419848ed187213cbbf4482f509459de99b216e6b5";

fn dir() -> PathBuf {
    std::env::var_os("PROXIM_CORE_DIR")
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            PathBuf::from,
        )
        .join("testdata/golden/rotation")
}

fn load(name: &str) -> (Vec<u8>, Value) {
    let bytes = std::fs::read(dir().join(name)).unwrap();
    let v = serde_json::from_slice(&bytes).unwrap();
    (bytes, v)
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn digests(v: &Value) -> Vec<[u8; 32]> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| unhex(x.as_str().unwrap()).try_into().unwrap())
        .collect()
}

fn family(t: &Value) -> Family {
    rotation::derive(
        &unhex(t["signpost_id_hex"].as_str().unwrap()),
        &digests(&t["digests_hex"]),
    )
    .unwrap()
}

fn snapshot(l: &Ledger) -> Value {
    let rows: Vec<Value> = l.rows.iter().map(|(v, (c, d))| json!([v, c, d])).collect();
    json!({"rows": rows, "recent": l.recent, "last_day": l.last_day})
}

fn u8_of(v: &Value) -> u8 {
    u8::try_from(v.as_u64().unwrap()).unwrap()
}

fn u32_of(v: &Value) -> u32 {
    u32::try_from(v.as_u64().unwrap()).unwrap()
}

/// Replay one select trace; returns the number of ops.
fn replay_select(t: &Value) -> usize {
    let fam = family(t);
    let mut led = Ledger {
        lru: usize::try_from(t["lru"].as_u64().unwrap()).unwrap(),
        ..Ledger::default()
    };
    if let Some(pl) = t.get("preload").and_then(Value::as_object) {
        for r in pl["rows"].as_array().unwrap() {
            led.rows
                .insert(u32_of(&r[0]), (u32_of(&r[1]), r[2].as_i64().unwrap()));
        }
        led.recent = pl["recent"]
            .as_array()
            .unwrap()
            .iter()
            .map(u32_of)
            .collect();
        led.last_day = pl["last_day"].as_i64();
    }
    let name = t["name"].as_str().unwrap();
    let mut pend = None;
    let mut last = None;
    let ops = t["ops"].as_array().unwrap();
    for op in ops {
        match op["op"].as_str().unwrap() {
            "select" => {
                let day = op["day"].as_i64().unwrap();
                let lane = op.get("lane").unwrap_or(&t["lane"]);
                let lanes = op.get("lanes").unwrap_or(&t["lanes"]);
                let got = rotation::select(
                    &led,
                    day,
                    &fam,
                    u8_of(&op["avail_mask"]),
                    u8_of(&t["dow_mask"]),
                    u32_of(lane),
                    u32_of(lanes),
                );
                let want = op["expect_idx"]
                    .as_u64()
                    .map(|i| (usize::try_from(i).unwrap(), u32_of(&op["expect_new_count"])));
                assert_eq!(got.map(|s| (s.idx, s.new_count)), want, "{name} day {day}");
                pend = got.map(|s| (s, day));
            }
            "commit" => {
                let (s, day) = pend.unwrap();
                rotation::commit(&mut led, fam.vids[s.idx], day, s.new_count);
                last = Some((s.idx, day));
            }
            "dismiss" => {
                let (i, commit_day) = last.unwrap();
                let day = op["day"].as_i64().unwrap_or(commit_day);
                rotation::dismiss(&mut led, fam.vids[i], day);
            }
            other => panic!("{name}: unknown op {other}"),
        }
    }
    assert_eq!(snapshot(&led), t["final"], "{name}: final ledger");
    ops.len()
}

fn replay_pending(case: &Value) {
    let fam = family(case);
    let name = case["name"].as_str().unwrap();
    let mut led = Ledger::default();
    let mut day = case["start_day"].as_i64().unwrap();
    let mut p: Option<Pending> = None;
    let mut commits = 0u32;
    let mut out = vec![];
    for ev in case["events"].as_array().unwrap() {
        let now = ev[1].as_i64().unwrap();
        match ev[0].as_str().unwrap() {
            "emit" => {
                let idx = rotation::emit(&mut p, now, || {
                    rotation::select(
                        &led,
                        day,
                        &fam,
                        0b11_1111,
                        case["dow_mask"]
                            .as_u64()
                            .map_or(0x1f, |m| u8::try_from(m).unwrap()),
                        0,
                        1,
                    )
                })
                .unwrap();
                out.push(json!(["emit", now, idx]));
            }
            "tick" => {
                if let Some(s) = rotation::tick(&mut p, now) {
                    rotation::commit(&mut led, fam.vids[s.idx], day, s.new_count);
                    commits += 1;
                }
                out.push(json!(["tick", now, commits]));
            }
            "mark" => {
                let (m, c) = rotation::mark_shown(&mut p, now, ev[2].as_i64().unwrap());
                if let Some(s) = c {
                    rotation::commit(&mut led, fam.vids[s.idx], day, s.new_count);
                }
                if matches!(
                    m,
                    rotation::Mark::Billed
                        | rotation::Mark::CommittedNotViewable
                        | rotation::Mark::BilledLate
                ) {
                    commits += 1;
                }
                out.push(json!(["mark", now, m.name()]));
            }
            "day" => day = now,
            other => panic!("{name}: unknown event {other}"),
        }
    }
    assert_eq!(Value::Array(out), case["expect_log"], "{name}: log");
    assert_eq!(snapshot(&led), case["expect_ledger"], "{name}: ledger");
}

#[test]
fn golden_v2_is_the_frozen_file() {
    let (bytes, _) = load("golden_v2.json");
    assert_eq!(
        hex(&sha256(&bytes)),
        GOLDEN_V2_SHA256,
        "golden_v2.json must never be regenerated"
    );
}

#[test]
fn constants_decay_sched_digests_and_derive() {
    let (_, g) = load("golden_v2.json");
    let c = &g["constants"];
    for (k, v) in [
        ("ONE", i64::from(rotation::ONE)),
        ("SAT", i64::from(rotation::SAT)),
        ("DISMISS", i64::from(rotation::DISMISS)),
        ("PURGE_BELOW", i64::from(rotation::PURGE_BELOW)),
        ("MAX_AGE", rotation::MAX_AGE),
        ("SLACK", i64::from(rotation::SLACK)),
        ("K_MAX", rotation::K_MAX as i64),
        ("RECENT_MAX", rotation::RECENT_MAX as i64),
        ("W_EMIT_MS", rotation::W_EMIT_MS),
        ("LATE_MS", rotation::LATE_MS),
        ("LRU_DEFAULT", rotation::LRU_DEFAULT as i64),
    ] {
        assert_eq!(c[k].as_i64(), Some(v), "{k}");
    }
    assert_eq!(
        unhex(c["ROT_PREFIX_HEX"].as_str().unwrap()),
        rotation::ROT_DOMAIN
    );
    assert_eq!(
        unhex(c["CREATIVE_PREFIX_HEX"].as_str().unwrap()),
        b"PROXIM-CREATIVE-V2\x00"
    );
    assert_eq!(
        unhex(c["ENV_PREFIX_HEX"].as_str().unwrap()),
        b"PROXIM-ENV-V2\x00"
    );
    assert_eq!(g["decay"], json!(rotation::DECAY.to_vec()));
    let mut n = 0;
    for r in g["sched_index"].as_array().unwrap() {
        assert_eq!(
            rotation::sched_index(r[0].as_i64().unwrap(), u8_of(&r[1])),
            r[2].as_i64().unwrap(),
            "{r}"
        );
        n += 1;
    }
    assert_eq!(n, 70);
    for r in g["digest_vectors"].as_array().unwrap() {
        let mut m = b"PROXIM-CREATIVE-V2\x00".to_vec();
        m.extend(unhex(r["canon_hex"].as_str().unwrap()));
        let d = sha256(&m);
        assert_eq!(hex(&d), r["digest_hex"].as_str().unwrap());
        assert_eq!(
            u32::from_be_bytes([d[0], d[1], d[2], d[3]]),
            u32_of(&r["vid"])
        );
    }
    for r in g["derive"].as_array().unwrap() {
        let f = family(r);
        assert_eq!(
            (json!(f.vids), json!(f.pos), f.phi),
            (r["vids"].clone(), r["pos"].clone(), u32_of(&r["phi"]))
        );
    }
}

#[test]
fn every_select_trace_replays() {
    let (_, g) = load("golden_v2.json");
    let traces = g["select"].as_array().unwrap();
    let ops: usize = traces.iter().map(replay_select).sum();
    assert_eq!((traces.len(), ops), (20, 487));
}

#[test]
fn every_pending_case_replays() {
    let (_, g) = load("golden_v2.json");
    let cases = g["pending"].as_array().unwrap();
    cases.iter().for_each(replay_pending);
    assert_eq!(cases.len(), 5);
}

#[test]
fn new_count_supplement_replays() {
    let (_, g) = load("golden_v2_newcount.json");
    let traces = g["select"].as_array().unwrap();
    traces.iter().for_each(|t| {
        replay_select(t);
    });
    let pending = g["pending"].as_array().unwrap();
    pending.iter().for_each(replay_pending);
    assert_eq!((traces.len(), pending.len()), (7, 5));
}

#[test]
fn a_family_with_a_vid_collision_is_refused() {
    let mut b = [9u8; 32];
    b[..4].copy_from_slice(&[7; 4]);
    let d = [[7u8; 32], b];
    assert!(rotation::derive(b"sp-1", &d).is_err());
    assert!(rotation::derive(b"sp-1", &[]).is_err());
}

/// Hard property (Addendum 02 5.1, N-38): inside one install the pick is never the last shown
/// wording (|A| >= 2) and never one of the last 3 (|A| >= 4), whatever the open pattern.
#[test]
fn never_repeats_the_recent_wordings() {
    let (_, g) = load("golden_v2.json");
    let fam = family(&g["derive"][0]);
    let n = fam.vids.len();
    let mut x: u64 = 0x2026_1017;
    for avail in [0b11u8, 0b1111, 0b11_1111, 0b10_1101] {
        let a = avail.count_ones() as usize;
        let mut led = Ledger::default();
        let mut day = 0i64;
        for _ in 0..400 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            day += i64::try_from(x % 4).unwrap();
            let s = rotation::select(&led, day, &fam, avail, 0x1f, 0, 1).unwrap();
            let k = 3.min(a - 1);
            let recent = &led.recent[led.recent.len().saturating_sub(k)..];
            if led.last_day.is_some_and(|l| day - l <= rotation::MAX_AGE) {
                assert!(
                    !recent.contains(&fam.vids[s.idx]),
                    "avail {avail:b} day {day}"
                );
            }
            assert!(s.idx < n && avail >> s.idx & 1 == 1);
            rotation::commit(&mut led, fam.vids[s.idx], day, s.new_count);
        }
    }
}

/// Review finding (2026-10-17): a stored count near `u32::MAX` must not overflow `top + 1`.
#[test]
fn huge_stored_counts_saturate() {
    let fam = rotation::derive(b"sp", &[[1; 32], [2; 32]]).unwrap();
    let mut led = Ledger::default();
    led.rows.insert(fam.vids[1], (u32::MAX, 5));
    let s = rotation::select(&led, 5, &fam, 0b11, 0x7f, 0, 1).unwrap();
    assert_eq!((s.idx, s.new_count), (0, rotation::SAT));
}

/// The reference counts a set bit 7 of the weekday mask (`mask or 0x7F`); so does the port.
#[test]
fn weekday_mask_bit_7_matches_the_reference() {
    assert_eq!(rotation::sched_index(1, 0x80), 0);
    assert_eq!(rotation::sched_index(8, 0xff), 9);
}
