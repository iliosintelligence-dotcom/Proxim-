//! `proxim-sim` command line (docs/simulator.md).
//!
//! ```text
//! proxim-sim run <file-or-dir>... [--seeds N] [--report FILE] [--junit FILE]
//! proxim-sim geo-cases [--count N] [--seed S] [--out FILE]
//! ```
//! `run` exits 1 if any scenario fails, 2 on a usage or load error. `PROXIM_SIM_SEEDS` sets the
//! seed count when `--seeds` is absent.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use proxim_sim::geocases::write_cases;
use proxim_sim::junit::junit_xml;
use proxim_sim::runner::{RealEngine, report_json, run_scenario};
use proxim_sim::scenario::{Scenario, list_dir};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  proxim-sim run <file-or-dir>... [--seeds N] [--report FILE] [--junit FILE]\n  proxim-sim geo-cases [--count N] [--seed S] [--out FILE]"
    );
    ExitCode::from(2)
}

fn num(v: Option<&String>) -> Option<u64> {
    v.and_then(|s| s.replace('_', "").parse().ok())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        return usage();
    };
    let mut paths = Vec::new();
    let mut opts = std::collections::BTreeMap::new();
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if let Some(k) = a.strip_prefix("--") {
            let Some(v) = it.next() else { return usage() };
            opts.insert(k.to_string(), v.clone());
        } else {
            paths.push(PathBuf::from(a));
        }
    }
    match cmd.as_str() {
        "run" => run(&paths, &opts),
        "geo-cases" => geo(&opts),
        _ => usage(),
    }
}

fn run(paths: &[PathBuf], opts: &std::collections::BTreeMap<String, String>) -> ExitCode {
    if paths.is_empty() {
        return usage();
    }
    let seeds = num(opts.get("seeds")).or_else(|| {
        std::env::var("PROXIM_SIM_SEEDS")
            .ok()
            .and_then(|s| s.parse().ok())
    });
    let mut files = Vec::new();
    for p in paths {
        if p.is_dir() {
            match list_dir(p) {
                Ok(v) => files.extend(v),
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            }
        } else {
            files.push(p.clone());
        }
    }
    let mut results = Vec::new();
    for f in &files {
        let sc = match Scenario::load(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}: {e}", f.display());
                return ExitCode::from(2);
            }
        };
        let runs = sc.runs(seeds);
        match run_scenario(&sc, runs, &mut RealEngine) {
            Ok(r) => {
                println!(
                    "{:<6} {:<36} runs {:>5}  {}",
                    if r.pass { "PASS" } else { "FAIL" },
                    r.id,
                    r.runs,
                    r.title
                );
                if !r.pass {
                    println!("{}", junit_failure_text(&r.summary));
                }
                results.push(r);
            }
            Err(e) => {
                eprintln!("{}: {e}", f.display());
                return ExitCode::from(2);
            }
        }
    }
    let failed = results.iter().filter(|r| !r.pass).count();
    println!("{} scenario(s), {} passed, {failed} failed", results.len(), results.len() - failed);
    if let Some(out) = opts.get("report") {
        match report_json(&results, seeds) {
            Ok(s) => {
                if let Err(e) = write(Path::new(out), s.as_bytes()) {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        }
    }
    if let Some(out) = opts.get("junit")
        && let Err(e) = write(Path::new(out), junit_xml(&results).as_bytes())
    {
        eprintln!("{e}");
        return ExitCode::from(2);
    }
    if failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn junit_failure_text(summary: &serde_json::Value) -> String {
    let mut s = String::new();
    if let Some(sps) = summary["signposts"].as_array() {
        for sp in sps {
            for f in sp["failures"].as_array().into_iter().flatten() {
                s.push_str("    ");
                s.push_str(f.as_str().unwrap_or(""));
                s.push('\n');
            }
        }
    }
    for f in summary["global_failures"].as_array().into_iter().flatten() {
        s.push_str("    ");
        s.push_str(f.as_str().unwrap_or(""));
        s.push('\n');
    }
    s
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn geo(opts: &std::collections::BTreeMap<String, String>) -> ExitCode {
    let count = num(opts.get("count")).unwrap_or(10_000);
    let seed = num(opts.get("seed")).unwrap_or(1);
    let res = if let Some(out) = opts.get("out") {
        match std::fs::File::create(out) {
            Ok(f) => {
                let mut w = std::io::BufWriter::new(f);
                let r = write_cases(count, seed, &mut w);
                r.and_then(|n| {
                    w.flush()
                        .map(|()| n)
                        .map_err(|e| proxim_sim::geom::SimError::new(e.to_string()))
                })
            }
            Err(e) => {
                eprintln!("{out}: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        let stdout = std::io::stdout();
        let mut w = std::io::BufWriter::new(stdout.lock());
        let r = write_cases(count, seed, &mut w);
        r.and_then(|n| {
            w.flush()
                .map(|()| n)
                .map_err(|e| proxim_sim::geom::SimError::new(e.to_string()))
        })
    };
    match res {
        Ok(n) => {
            eprintln!("geo-cases: wrote {n} cases (seed {seed})");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("geo-cases: {e}");
            ExitCode::from(1)
        }
    }
}
