//! The two quantized trig tables (spec 2.2) against their frozen golden JSON files, byte for byte.
//!
//! `core/testdata/golden/frame_scale_table.json` and `sincos_table.json` hold the full tables
//! (`rows`), every 100th row (`entries`, the spec 9 sample) and the SHA-256 of all rows rendered as
//! `idx,v1,v2\n` (`sha256`). Regenerate after a reviewed change with
//! `PROXIM_BLESS=1 cargo test -p proxim-geo --test trig_tables`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::sha256_hex;
use proxim_geo::trig::{SCALE_LAT_Q_MAX, SCALE_LEN, SINCOS_LEN, scale_for_lat_q, unit_from_course};
use proxim_types::canon::value_to_canonical;
use proxim_types::strict::{JsonLimits, from_canonical_json_with};
use serde_json::{Value, json};
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/golden")
}

type Row = (usize, i64, i64);

fn scale_rows() -> Vec<Row> {
    (0..SCALE_LEN)
        .map(|idx| {
            let lat_q = i32::try_from(idx).unwrap() - SCALE_LAT_Q_MAX;
            let (kn, ke) = scale_for_lat_q(lat_q).unwrap();
            (idx, kn, ke)
        })
        .collect()
}

fn sincos_rows() -> Vec<Row> {
    (0..SINCOS_LEN)
        .map(|idx| {
            let [e, n] = unit_from_course(u16::try_from(idx).unwrap()).unwrap();
            (idx, i64::from(e), i64::from(n))
        })
        .collect()
}

fn digest(rows: &[Row]) -> String {
    let mut text = String::new();
    for (i, a, b) in rows {
        text.push_str(&format!("{i},{a},{b}\n"));
    }
    sha256_hex(text.as_bytes())
}

fn as_json(rows: &[Row]) -> Vec<Value> {
    rows.iter().map(|(i, a, b)| json!([i, a, b])).collect()
}

fn document(id: &str, extra: Value, columns: [&str; 3], rows: &[Row]) -> String {
    let sample: Vec<Value> = rows
        .iter()
        .filter(|r| r.0 % 100 == 0)
        .map(|(i, a, b)| json!([i, a, b]))
        .collect();
    let mut doc = json!({
        "format": "proxim-golden/1",
        "kind": "trig",
        "id": id,
        "engine_spec": 1,
        "synthetic": true,
        "columns": columns,
        "entries": sample,
        "rows": as_json(rows),
        "sha256": digest(rows),
    });
    if let (Some(obj), Some(more)) = (doc.as_object_mut(), extra.as_object()) {
        for (k, v) in more {
            obj.insert(k.clone(), v.clone());
        }
    }
    let mut text = value_to_canonical(&doc).unwrap();
    text.push('\n');
    text
}

fn scale_doc() -> String {
    document(
        "frame_scale_table",
        json!({"idx0_lat_q": -SCALE_LAT_Q_MAX, "lat_q_step_deg_e2": 1}),
        ["idx", "kn_q32", "ke_q32"],
        &scale_rows(),
    )
}

fn sincos_doc() -> String {
    document(
        "sincos_table",
        json!({"course_step_ddeg": 1}),
        ["idx", "east_q30", "north_q30"],
        &sincos_rows(),
    )
}

fn check_or_bless(file: &str, want: &str) {
    let path = golden_dir().join(file);
    if std::env::var("PROXIM_BLESS").as_deref() == Ok("1") {
        std::fs::write(&path, want).unwrap();
    }
    let got = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e} (run with PROXIM_BLESS=1 to create it)",
            path.display()
        )
    });
    assert!(
        got == want,
        "{file} differs from the compiled table (len {} vs {}); if the change is intended, re-bless \
         with PROXIM_BLESS=1 and bump engine_spec",
        got.len(),
        want.len()
    );
}

#[test]
fn sha256_known_answers() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(
        sha256_hex(&vec![b'a'; 1_000_000]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn tables_match_golden_json() {
    check_or_bless("frame_scale_table.json", &scale_doc());
    check_or_bless("sincos_table.json", &sincos_doc());
}

#[test]
fn golden_files_parse_strictly_and_every_row_agrees() {
    for (file, n) in [
        ("frame_scale_table.json", SCALE_LEN),
        ("sincos_table.json", SINCOS_LEN),
    ] {
        let bytes = std::fs::read(golden_dir().join(file)).unwrap();
        assert_eq!(bytes.last(), Some(&b'\n'), "{file}: trailing newline");
        let body = &bytes[..bytes.len() - 1];
        let limits = JsonLimits {
            max_array_len: 20_000,
            ..JsonLimits::default()
        };
        let v: Value = from_canonical_json_with(body, &limits).unwrap(); // canonical bytes, no null, no float
        assert_eq!(v["format"], "proxim-golden/1");
        assert_eq!(v["kind"], "trig");
        assert_eq!(v["engine_spec"], 1);
        assert_eq!(v["synthetic"], true);
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), n, "{file}");
        let entries = v["entries"].as_array().unwrap();
        assert_eq!(entries.len(), n.div_ceil(100));
        for (k, e) in entries.iter().enumerate() {
            assert_eq!(e, &rows[k * 100], "{file} entry {k}");
        }
        for (idx, row) in rows.iter().enumerate() {
            let row = row.as_array().unwrap();
            assert_eq!(row[0].as_u64().unwrap(), idx as u64);
            let (a, b) = (row[1].as_i64().unwrap(), row[2].as_i64().unwrap());
            if file.starts_with("frame") {
                let lat_q = i32::try_from(idx).unwrap() - SCALE_LAT_Q_MAX;
                assert_eq!(scale_for_lat_q(lat_q).unwrap(), (a, b), "scale row {idx}");
            } else {
                let [e, nn] = unit_from_course(u16::try_from(idx).unwrap()).unwrap();
                assert_eq!((i64::from(e), i64::from(nn)), (a, b), "sincos row {idx}");
            }
        }
        // The stored digest is the digest of the stored rows (recomputed here from the JSON).
        let parsed: Vec<Row> = rows
            .iter()
            .map(|r| {
                (
                    r[0].as_u64().unwrap() as usize,
                    r[1].as_i64().unwrap(),
                    r[2].as_i64().unwrap(),
                )
            })
            .collect();
        assert_eq!(v["sha256"].as_str().unwrap(), digest(&parsed));
    }
}

#[test]
fn digests_are_pinned_in_the_test_too() {
    // Belt and braces: a re-bless that silently changes a table still has to change this literal in
    // the same reviewed diff. (Values produced by the independent Python check in the spec errata.)
    assert_eq!(digest(&scale_rows()), PINNED_SCALE_SHA256);
    assert_eq!(digest(&sincos_rows()), PINNED_SINCOS_SHA256);
}

const PINNED_SCALE_SHA256: &str =
    "539ff5182206bb12baf90cc21bddeb4f0306746e3e9209d7ecf62ccef947771a";
const PINNED_SINCOS_SHA256: &str =
    "79bcdda582a121931729d908ee2782154ad3a67a7ad1cca9ef27f7f5d24b73e1";
