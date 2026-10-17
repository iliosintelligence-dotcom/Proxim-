//! Contract tests between packages and against the golden vectors (spec 9): both bindings return
//! identical bytes, and every `decide` golden vector replays byte for byte under an independent
//! SHA-256 and canonical writer.
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

use proxim_conformance::fuzz::{make_case, rich_world};
use proxim_conformance::rng::Rng;
use proxim_conformance::sha::sha256_hex;
use proxim_conformance::*;
use serde_json::{Value, json};
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    core_dir().join("testdata/golden")
}

fn all_json_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            all_json_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "json") {
            out.push(p);
        }
    }
}

#[test]
fn ffi_and_wasm_bindings_return_identical_bytes() {
    let mut n = 0;
    for seed in 0..1500_u64 {
        let inp = make_case(seed * 7919 + 1);
        let a = proxim_ffi::decide_json(
            &inp.args[0],
            &inp.args[1],
            &inp.args[2],
            &inp.args[3],
            &inp.args[4],
            &inp.args[5],
            inp.seed,
        );
        let b = proxim_wasm::decide_json(
            &inp.args[0],
            &inp.args[1],
            &inp.args[2],
            &inp.args[3],
            &inp.args[4],
            &inp.args[5],
            inp.seed,
        );
        assert_eq!(a, b, "bindings differ for case {seed}");
        n += 1;
    }
    // and across a real trace with threaded state
    let mut rng = Rng::new(9);
    let sps = mixed_signposts(&mut rng);
    let trace = scripted_trace(&mut rng, &sps, 8);
    let (mut sa, mut sb) = (empty_state(), empty_state());
    for (f, now) in &trace {
        let args = |state: &str| {
            (
                state.to_string(),
                render(f),
                render(&Value::Array(sps.clone())),
                render(&json!({"explain": true})),
                render(&consent_granted()),
                render(&clock(*now, 0)),
            )
        };
        let (s1, f1, g1, p1, c1, k1) = args(&sa);
        let a = proxim_ffi::decide_json(&s1, &f1, &g1, &p1, &c1, &k1, 0);
        let (s2, f2, g2, p2, c2, k2) = args(&sb);
        let b = proxim_wasm::decide_json(&s2, &f2, &g2, &p2, &c2, &k2, 0);
        assert_eq!(a, b);
        let r = parse_response(a);
        if r.is_ok() {
            sa = render(&r.state);
            sb = sa.clone();
        }
        n += 1;
    }
    assert!(n > 1500);
    assert_eq!(proxim_ffi::engine_spec(), proxim_wasm::engine_spec());
}

#[test]
fn every_golden_file_is_canonical_synthetic_and_listed_in_the_manifest() {
    let root = golden_dir();
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root.join("MANIFEST.json")).unwrap()).unwrap();
    assert_eq!(manifest["engine_spec"], proxim_types::ENGINE_SPEC);
    assert_eq!(manifest["format"], "proxim-golden-manifest/1");
    let listed: std::collections::BTreeMap<String, String> = manifest["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["path"].as_str().unwrap().to_string(),
                v["sha256"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let mut files = vec![];
    all_json_files(&root, &mut files);
    let mut seen = 0;
    for f in files {
        let rel = f.strip_prefix(&root).unwrap().to_str().unwrap().to_string();
        if rel == "MANIFEST.json" {
            continue;
        }
        let bytes = std::fs::read(&f).unwrap();
        let want = listed
            .get(&rel)
            .unwrap_or_else(|| panic!("{rel} is not in MANIFEST.json"));
        assert_eq!(&sha256_hex(&bytes), want, "{rel}: manifest hash differs");
        // OI-ROT-1 (Addendum 02 13): golden_v2.json is committed unchanged from the reference
        // kernel's output (indented, no header) and is never regenerated, so only its listing and
        // hash are checked here; `proxim-decide/tests/rotation_golden.rs` pins its exact bytes.
        if rel == "rotation/golden_v2.json" {
            seen += 1;
            continue;
        }
        let text = std::str::from_utf8(&bytes).unwrap();
        let body = text
            .strip_suffix('\n')
            .unwrap_or_else(|| panic!("{rel}: no trailing newline"));
        assert!(
            !body.ends_with('\n'),
            "{rel}: more than one trailing newline"
        );
        check_canonical(body).unwrap_or_else(|e| panic!("{rel}: not canonical JSON ({e})"));
        let v: Value = serde_json::from_str(body).unwrap();
        // the two trig table files carry the header fields at top level (see D-TRIG-HEADER below)
        let h = if v.get("header").is_some() {
            &v["header"]
        } else {
            &v
        };
        assert_eq!(h["format"], "proxim-golden/1", "{rel}");
        // The header records the spec a vector was made under (engine spec 9, E-SPEC2): decide
        // vectors must match the current spec; other kinds stay valid until their behaviour moves.
        let made_under = h["engine_spec"].as_u64().unwrap();
        if rel.starts_with("decide/") {
            assert_eq!(made_under, u64::from(proxim_types::ENGINE_SPEC), "{rel}");
        } else {
            assert!(
                (1..=u64::from(proxim_types::ENGINE_SPEC)).contains(&made_under),
                "{rel}"
            );
        }
        assert_eq!(
            h["synthetic"], true,
            "{rel}: golden data must be synthetic (I2)"
        );
        seen += 1;
    }
    for p in listed.keys() {
        assert!(root.join(p).exists(), "manifest lists missing file {p}");
    }
    assert_eq!(seen, listed.len(), "manifest and directory disagree");
    assert!(seen >= 20);
}

#[test]
fn every_decide_golden_vector_replays_byte_for_byte() {
    let dir = golden_dir().join("decide");
    let mut files = vec![];
    all_json_files(&dir, &mut files);
    assert!(files.len() >= 20, "{} decide vectors", files.len());
    let mut steps_total = 0;
    for f in files {
        let name = f.file_name().unwrap().to_str().unwrap().to_string();
        let v: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
        assert_eq!(v["header"]["kind"], "decide", "{name}");
        assert_eq!(
            v["header"]["id"].as_str().unwrap(),
            name.trim_end_matches(".json"),
            "{name}: id must match file name"
        );
        let seed = v["seed"].as_u64().unwrap();
        let mut state = empty_state();
        let mut trace = String::new();
        for (i, step) in v["steps"].as_array().unwrap().iter().enumerate() {
            let resp = proxim_ffi::decide_json(
                &state,
                &render(&step["fix"]),
                &render(&v["signposts"]),
                &render(&v["policy"]),
                &render(&v["consent"]),
                &render(&step["clock"]),
                seed,
            );
            let r = parse_response(resp);
            assert!(r.is_ok(), "{name} step {i}: {}", r.text);
            let out_text = render(&r.output);
            assert_eq!(
                out_text,
                render(&step["expect"]["output"]),
                "{name} step {i}: output bytes differ"
            );
            state = render(&r.state);
            let sha = sha256_hex(state.as_bytes());
            assert_eq!(
                sha,
                step["expect"]["state_sha256"].as_str().unwrap(),
                "{name} step {i}: state hash differs"
            );
            trace.push_str(&out_text);
            trace.push('\n');
            trace.push_str(&sha);
            trace.push('\n');
            steps_total += 1;
        }
        assert_eq!(
            sha256_hex(trace.as_bytes()),
            v["trace_sha256"].as_str().unwrap(),
            "{name}: trace hash differs"
        );
    }
    assert!(steps_total > 500, "{steps_total} golden steps replayed");
}

#[test]
fn golden_scenarios_cover_the_spec_s_must_not_fire_list() {
    // the vectors that exist must at least name the scenarios section 11 insists on
    let dir = golden_dir().join("decide");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    names.sort();
    for needed in [
        "consent_unknown",
        "u_turn",
        "opposite_carriageway",
        "late_join",
        "cap_exhaustion",
        "clock_goes_backwards",
        "standoff",
        "boundary_jitter",
        "parallel",
    ] {
        assert!(
            names.iter().any(|n| n.contains(needed)),
            "no golden vector covers `{needed}`: {names:?}"
        );
    }
}

#[test]
fn rich_world_is_stable_across_bindings_and_runs() {
    let w = rich_world();
    let a = proxim_ffi::decide_json(
        &render(&w[0]),
        &render(&w[1]),
        &render(&w[2]),
        &render(&w[3]),
        &render(&w[4]),
        &render(&w[5]),
        7,
    );
    let b = proxim_wasm::decide_json(
        &render(&w[0]),
        &render(&w[1]),
        &render(&w[2]),
        &render(&w[3]),
        &render(&w[4]),
        &render(&w[5]),
        7,
    );
    assert_eq!(a, b);
    assert_eq!(sha256_hex(a.as_bytes()).len(), 64);
}

#[test]
fn trig_table_files_use_the_nested_header_of_spec_9() {
    // Spec 9: `trig` tables are `{header, entries: every 100th row, sha256}`; the files put the
    // header fields at top level and add `rows`/`columns`.
    for name in ["frame_scale_table.json", "sincos_table.json"] {
        let v: Value =
            serde_json::from_slice(&std::fs::read(golden_dir().join(name)).unwrap()).unwrap();
        assert!(
            v.get("header").is_some_and(Value::is_object),
            "{name}: no `header` object"
        );
        assert_eq!(v["header"]["kind"], "trig");
    }
}

/// The `core/` directory. `PROXIM_CORE_DIR` overrides the build machine's path
/// when the tests run on a device (Android emulator) with the tree copied over.
fn core_dir() -> std::path::PathBuf {
    std::env::var_os("PROXIM_CORE_DIR").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    )
}
