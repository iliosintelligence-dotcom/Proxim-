//! `MANIFEST.json`: `{path, sha256}` of every JSON file under `core/testdata/golden` (engine-spec
//! 9), sorted by path, canonical JSON plus a trailing newline.
#![allow(clippy::unwrap_used)]

use proxim_decide::sha256::sha256_hex;
use proxim_types::canon::to_canonical_json;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Entry {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct Manifest {
    format: &'static str,
    engine_spec: u32,
    vectors: Vec<Entry>,
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    for p in names {
        if p.is_dir() {
            walk(root, &p, out);
        } else if p.extension().is_some_and(|e| e == "json")
            && p.file_name().is_some_and(|n| n != "MANIFEST.json")
        {
            out.push(p.strip_prefix(root).unwrap().to_path_buf());
        }
    }
}

/// The manifest text for the golden directory as it is on disk now.
pub fn render(golden: &Path) -> Result<String, proxim_types::EngineError> {
    let mut files = Vec::new();
    walk(golden, golden, &mut files);
    let mut vectors: Vec<Entry> = files
        .iter()
        .map(|rel| Entry {
            path: rel.to_string_lossy().replace('\\', "/"),
            sha256: sha256_hex(&std::fs::read(golden.join(rel)).unwrap()),
        })
        .collect();
    vectors.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let mut s = to_canonical_json(&Manifest {
        format: "proxim-golden-manifest/1",
        engine_spec: proxim_types::ENGINE_SPEC,
        vectors,
    })?;
    s.push('\n');
    Ok(s)
}
