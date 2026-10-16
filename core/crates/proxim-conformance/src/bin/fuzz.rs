//! Long-running fuzz driver: `cargo run --release -p proxim-conformance --bin fuzz -- <first_seed> <cases> [verbose]`.
//! Exits non-zero (and prints the offending inputs) on any panic, hang, non-canonical response or
//! state that cannot be re-ingested. Use it for long runs outside the CI budget.
#![allow(
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types
)]

use proxim_conformance::fuzz::{Inputs, make_case, run_cases};
use std::time::Duration;

fn verbose_case(seed: u64) -> Inputs {
    eprintln!("case seed {seed}");
    make_case(seed)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "one") {
        // replay one derived case seed (as printed by `verbose`) and show the inputs and the response
        let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
        let inp = make_case(seed);
        println!("{}", inp.describe());
        let t0 = std::time::Instant::now();
        let out = inp.call();
        println!(
            "response ({} bytes, {:?}): {}",
            out.len(),
            t0.elapsed(),
            &out[..out.len().min(300)]
        );
        return;
    }
    let first: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let n: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let verbose = args.get(3).is_some_and(|s| s == "verbose");
    let st = run_cases(
        first,
        n,
        1 << 20,
        Duration::from_secs(120),
        if verbose { verbose_case } else { make_case },
    );
    println!(
        "cases={} errors={} successes={} panics={} hangs={} non_canonical={} reingest_failures={} slowest={}ms",
        st.cases,
        st.errors,
        st.successes,
        st.panics,
        st.hangs,
        st.non_canonical,
        st.reingest_failures,
        st.slowest_ms
    );
    for f in st.failures.iter().take(5) {
        println!("FAILURE:\n{f}");
    }
    if !st.failures.is_empty() {
        std::process::exit(1);
    }
}
