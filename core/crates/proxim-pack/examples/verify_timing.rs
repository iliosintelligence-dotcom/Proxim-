//! ES256 verify and pack-open timing (PLAN M2 "ECDSA verify time"). Real-device numbers come from
//! the iPhone and Android runs (M8/M10); this gives the host and WebAssembly baseline.
//! `cargo run --release -p proxim-pack --features testkit --example verify_timing`
#![allow(
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    clippy::disallowed_types // a timing tool outside the engine needs a clock
)]

#[path = "../tests/common/mod.rs"]
mod common;

use common::*;
use proxim_pack::{RevocationList, open_pack};
use std::time::Instant;

fn main() {
    let a = approve();
    let revs: Vec<Rev> = (0..500)
        .map(|i| {
            sign_rev(
                &signpost(&format!("sp-{i:04}"), 1, &[]),
                &["Coffee ahead", "Fresh pastries"],
                &a,
            )
        })
        .collect();
    let rows: Vec<_> = revs.iter().map(all_shipped).collect();
    let ks = default_keyset();
    let pack = build(&header(1), &rows);
    let m = manifest_for(1, &pack, &ks);
    let none = RevocationList::default();
    let _ = open_pack(&pack, &m, 0, &ks, &none, NOW).unwrap();
    let runs = 5;
    let t = Instant::now();
    for _ in 0..runs {
        let p = open_pack(&pack, &m, 0, &ks, &none, NOW).unwrap();
        assert_eq!(p.entries.len(), 500);
    }
    let per_pack = t.elapsed().as_secs_f64() * 1000.0 / f64::from(runs);
    println!(
        "pack of 500 entries, {} bytes: open {:.1} ms ({:.0} us per entry)",
        pack.len(),
        per_pack,
        per_pack * 1000.0 / 500.0
    );
}
