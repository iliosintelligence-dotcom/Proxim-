//! ADR-001 verification item: no float type, float literal or `libm` call appears in `proxim-types`
//! or in `proxim-geo` outside `trig.rs` and `h3.rs` (engine-spec 2.1). Comments and string
//! literals are ignored. This is the fallback for the clippy `disallowed-types` gap on `f64`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Source with comments, string literals and char literals blanked out (newlines kept).
fn code_only(src: &str) -> String {
    let b: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == 'r' && matches!(next, Some('"' | '#')) {
            // raw string r"..." or r#"..."#
            let mut j = i + 1;
            let mut hashes = 0;
            while b.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if b.get(j) == Some(&'"') {
                j += 1;
                loop {
                    if j >= b.len() {
                        break;
                    }
                    if b[j] == '"' && (0..hashes).all(|h| b.get(j + 1 + h) == Some(&'#')) {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                i = j;
                out.push(' ');
            } else {
                out.push(c);
                i += 1;
            }
        } else if c == '"' {
            i += 1;
            while i < b.len() && b[i] != '"' {
                if b[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            out.push(' ');
        } else if c == '\'' && b.get(i + 2) == Some(&'\'') {
            i += 3; // 'x'
            out.push(' ');
        } else if c == '\'' && next == Some('\\') {
            i += 2;
            while i < b.len() && b[i] != '\'' {
                i += 1;
            }
            i += 1;
            out.push(' ');
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn violations(code: &str) -> Vec<String> {
    let mut v = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphanumeric() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let tok: String = chars[start..i].iter().collect();
            let numeric = tok.chars().next().is_some_and(|c| c.is_ascii_digit());
            if tok == "f32"
                || tok == "f64"
                || (numeric && (tok.ends_with("f32") || tok.ends_with("f64")))
            {
                v.push(format!("float type/suffix `{tok}`"));
            }
            let after_dot = start > 0 && chars[start - 1] == '.'; // tuple field `x.0.1`
            if numeric
                && !after_dot
                && chars.get(i) == Some(&'.')
                && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())
            {
                v.push(format!("float literal starting `{tok}.`"));
            }
            if tok == "libm" {
                v.push("libm".to_string());
            }
        } else {
            i += 1;
        }
    }
    v
}

#[test]
fn floats_exist_only_in_trig_and_h3() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    rust_files(&root.join("proxim-types/src"), &mut files);
    rust_files(&root.join("proxim-geo/src"), &mut files);
    assert!(
        files.len() >= 12,
        "scanner found only {} files",
        files.len()
    );
    let mut allowed_seen = 0;
    let mut bad = Vec::new();
    for f in &files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let in_geo = f.to_string_lossy().contains("proxim-geo");
        let exempt = in_geo && (name == "trig.rs" || name == "h3.rs");
        let found = violations(&code_only(&fs::read_to_string(f).unwrap()));
        if exempt {
            allowed_seen += usize::from(!found.is_empty());
        } else {
            for x in found {
                bad.push(format!("{}: {x}", f.display()));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "floats outside the two float modules:\n{}",
        bad.join("\n")
    );
    assert_eq!(
        allowed_seen, 2,
        "the scanner must see floats in trig.rs and h3.rs, or it checks nothing"
    );
}

#[test]
fn scanner_catches_what_it_should() {
    assert!(!violations(&code_only("let x: f64 = 1.5;")).is_empty());
    assert!(!violations(&code_only("let x = 2.0;")).is_empty());
    assert!(!violations(&code_only("let x = 3f32;")).is_empty());
    assert!(!violations(&code_only("libm::sin(1)")).is_empty());
    assert!(
        violations(&code_only(
            "// f64 and 1.5 in a comment\nlet s = \"f64 1.5 libm\";"
        ))
        .is_empty()
    );
    assert!(
        violations(&code_only(
            "let r = r#\"f64 \"quoted\" 2.5\"#; let t = x.0.1;"
        ))
        .is_empty()
    );
    assert!(violations(&code_only("let c = '\"'; let n = 12; let t = a.0;")).is_empty());
    assert!(violations(&code_only("let m = u64::MAX; let i = is_f64;")).is_empty());
}
