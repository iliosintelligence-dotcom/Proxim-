//! Writes `core/testdata/sim/phone-simulator-results.md` from a fresh run of the phone simulator.
//! Dry run by default (exit 1 if the file would change); `PROXIM_BLESS=1` writes it.
#![allow(clippy::print_stderr, clippy::expect_used)]

use proxim_sim::phone::{Routing, run};
use proxim_sim::{report, town};
use std::path::PathBuf;

fn main() {
    let world = town::world().expect("built-in town");
    let runs = [Routing::Broad, Routing::BestFit].map(|r| run(&world, r).expect("run"));
    let text = report::markdown(&world, &runs);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/sim/phone-simulator-results.md");
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    if old == text {
        eprintln!("{}: up to date", path.display());
    } else if std::env::var("PROXIM_BLESS").as_deref() == Ok("1") {
        std::fs::write(&path, &text).expect("write report");
        eprintln!("{}: written", path.display());
    } else {
        eprintln!("{}: would change (run with PROXIM_BLESS=1)", path.display());
        std::process::exit(1);
    }
}
