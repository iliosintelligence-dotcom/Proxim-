//! Golden vector generator for `decide` (engine-spec 9).
//!
//! `cargo run -p proxim-decide --example gen_golden` compares what the current code would write
//! with the files on disk and exits non-zero on any byte difference (a dry run).
//! `PROXIM_BLESS=1 cargo run -p proxim-decide --example gen_golden` rewrites the vectors and
//! `MANIFEST.json`; the diff must be reviewed and a byte change bumps `engine_spec`.
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::unwrap_used)]

#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/golden_scenarios.rs"]
mod golden_scenarios;
#[path = "support/manifest.rs"]
mod manifest;

use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/golden")
}

fn main() {
    let bless = std::env::var("PROXIM_BLESS").as_deref() == Ok("1");
    let dir = golden_dir();
    let mut diffs = 0;
    for sc in golden_scenarios::scenarios() {
        let vector = proxim_decide::golden::build(&sc).unwrap();
        let text = proxim_decide::golden::render(&vector).unwrap();
        let path = dir.join("decide").join(format!("{}.json", sc.id));
        let on_disk = std::fs::read_to_string(&path).ok();
        if on_disk.as_deref() == Some(text.as_str()) {
            println!("same    {}", path.display());
            continue;
        }
        diffs += 1;
        if bless {
            std::fs::write(&path, &text).unwrap();
            println!("written {} ({} bytes)", path.display(), text.len());
        } else {
            println!(
                "DIFFERS {} (run with PROXIM_BLESS=1 to write)",
                path.display()
            );
        }
    }
    let manifest = manifest::render(&dir).unwrap();
    let mpath = dir.join("MANIFEST.json");
    if std::fs::read_to_string(&mpath).ok().as_deref() != Some(manifest.as_str()) {
        diffs += 1;
        if bless {
            std::fs::write(&mpath, &manifest).unwrap();
            println!("written {}", mpath.display());
        } else {
            println!("DIFFERS {}", mpath.display());
        }
    }
    if diffs > 0 && !bless {
        std::process::exit(1);
    }
}
