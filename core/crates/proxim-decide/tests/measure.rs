//! Measurement report (run with
//! `cargo test -p proxim-decide --test measure --release -- --ignored --nocapture`).
//! Prints the behaviour of `decide` on the fixture routes and its cost; the assertions are only
//! sanity checks. All figures are integers (metres are printed as `m.d`).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stderr,
    clippy::cast_possible_truncation
)]

mod common;

use common::*;
use proxim_decide::decide;
use proxim_types::{Mode, Outcome, Policy, ReasonCode, State};

fn d1(x: i64) -> String {
    let sign = if x < 0 { "-" } else { "" };
    let a = x.abs();
    format!("{sign}{}.{}", a / 1000, (a % 1000) / 100)
}

// Timing is a measurement of the harness only: the engine itself has no clock (the disallowed-types
// rule exists for the engine crates), so `Instant` is allowed in this report test.
#[test]
#[ignore = "report generator"]
#[allow(clippy::too_many_lines, clippy::disallowed_types)]
fn report() {
    eprintln!("== APPROACH through decide, route 6 km, 1 Hz, acc 5 m, lead 40 s, d_hi 3 km ==");
    eprintln!("v_mps  mode     D_fire_spec_m  fired_at_d_rem_m  t_hat_s  conf  fix_index");
    for (v, mode, spec) in [
        (3_000u32, Mode::Vehicle, "172"),
        (5_000, Mode::Cycle, "215"),
        (15_000, Mode::Vehicle, "645"),
        (20_000, Mode::Vehicle, "860"),
        (29_000, Mode::Vehicle, "1247"),
    ] {
        let mut sim = Sim::new(vec![with_mode(route("r", 6000, 30), mode)]);
        let n = (6_000_000 / i64::from(v)) as usize + 3;
        let outs = sim.run(&drive_north(T0, 0, v, n, 0));
        let idx = fire_indices(&outs);
        let d = fires(&outs[idx[0]])[0].clone();
        eprintln!(
            "{:>5}  {:7}  {:>13}  {:>16}  {:>7}  {:>4}  {:>9}   fires: {}",
            d1(i64::from(v)),
            format!("{mode:?}"),
            spec,
            d1(d.d_rem_mm.unwrap()),
            d1(d.t_hat_ms.unwrap()),
            d.path_conf_milli,
            idx[0],
            idx.len()
        );
    }

    eprintln!(
        "== ENTER circle r=150 m, acc 5 m (acc_eff 10 m): distance from the centre at the fire =="
    );
    for v in [1_500i32, 5_000, 15_000, 29_000] {
        let count = 500_000 / v + 40;
        let fixes: Vec<_> = (0..count)
            .map(|i| {
                fix_at(
                    T0 + i64::from(i) * 1000,
                    0,
                    -250_000 + v * i,
                    5000,
                    Some(v as u32),
                    Some(0),
                )
            })
            .collect();
        let mut sim = Sim::new(vec![circle("c", 150)]);
        let outs = sim.run(&fixes);
        let idx = fire_indices(&outs);
        eprintln!(
            "v {:>5} m/s: fire at {} m from the centre (fix {})",
            d1(i64::from(v)),
            idx.first().map_or("-".to_string(), |i| d1(i64::from(
                (-250_000 + v * *i as i32).abs()
            ))),
            idx.first().map_or(-1, |i| *i as i64)
        );
    }
    eprintln!(
        "== ENTER jitter: stationary device oscillating around the radius-150 edge never fires =="
    );
    for amp in [2_000i32, 5_000, 9_000, 10_000, 12_000] {
        let fixes: Vec<_> = (0..200)
            .map(|i| {
                still(
                    T0 + 4000 * i,
                    0,
                    150_000 + if i % 2 == 0 { -amp } else { amp },
                )
            })
            .collect();
        let mut sim = Sim::new(vec![circle("c", 150)]);
        let outs = sim.run(&fixes);
        eprintln!(
            "+-{} m around the edge: fires {}",
            d1(i64::from(amp)),
            fire_indices(&outs).len()
        );
    }

    eprintln!(
        "== parallel same-direction road at lateral offset (buffer 30 m, acc 5 m, 20 m/s) =="
    );
    for off in [0, 10_000, 25_000, 30_000, 31_000, 60_000, 200_000] {
        let mut sim = Sim::new(vec![route("r", 6000, 30)]);
        let outs = sim.run(&drive_north(T0, 0, 20_000, 310, off));
        eprintln!(
            "offset {:>5} m -> fires {}",
            d1(i64::from(off)),
            fire_indices(&outs).len()
        );
    }
    eprintln!(
        "== opposite carriageway (8 m, heading south) and a U-turn 1 km before the anchor =="
    );
    let mut sim = Sim::new(vec![route("r", 2500, 30)]);
    let outs = sim.run(&drive_south(T0, 2_400_000, 15_000, 150, 8_000));
    eprintln!("opposite carriageway: fires {}", fire_indices(&outs).len());
    let mut fixes = drive_north(T0, 700_000, 15_000, 21, 0);
    fixes.extend(drive_south(T0 + 21_000, 985_000, 15_000, 50, 0));
    let mut sim = Sim::new(vec![route("r", 2500, 30)]);
    eprintln!(
        "U-turn at 1 km: fires {}",
        fire_indices(&sim.run(&fixes)).len()
    );

    eprintln!("== late join after a GPS drop: first fix d_rem (m) and the outcome ==");
    for start in [2_000_000i64, 2_200_000, 2_240_000, 2_300_000, 2_340_000] {
        let mut sim = Sim::new(vec![route("r", 2500, 30)]);
        let outs = sim.run(&drive_north(T0, start, 15_000, 20, 0));
        let late = outs
            .iter()
            .any(|o| has_drop(o, "r", ReasonCode::TooLateForLead));
        eprintln!(
            "start d_rem {:>6} m -> fires {} too_late {}",
            d1(2_500_000 - start),
            fire_indices(&outs).len(),
            late
        );
    }

    eprintln!("== caps: five visits a day, six days ==");
    let mut sim = Sim::new(vec![circle("c", 200)]);
    let mut fired = Vec::new();
    for day in 0..6 {
        for k in 0..5 {
            let outs = sim.run(&enter_cycle(
                T0 + day * DAY + 1_950_000 * k,
                k > 0 || day > 0,
            ));
            fired.push(outs.iter().any(|o| !fires(o).is_empty()));
        }
    }
    eprintln!(
        "fires per day: {:?}",
        fired
            .chunks(5)
            .map(|c| c.iter().filter(|x| **x).count())
            .collect::<Vec<_>>()
    );

    eprintln!("== cost of one decide call (release build recommended) ==");
    for n in [1usize, 10, 100, 500, 1000, 4096] {
        let signposts: Vec<_> = (0..n)
            .map(|i| {
                let e = i32::try_from((i % 64) * 2_000).unwrap() * 1000 / 1000;
                if i % 3 == 0 {
                    let mut r = route(&format!("r-{i:04}"), 2500, 30);
                    r.trigger.origin = pos(e * 1000, 0);
                    r
                } else {
                    circle_at(
                        &format!("c-{i:04}"),
                        e * 1000,
                        ((i / 64) as i32) * 1_000_000,
                        200,
                    )
                }
            })
            .collect();
        let fix = still(T0, 0, 0);
        let state = State::empty();
        let t = std::time::Instant::now();
        let reps = 20;
        for _ in 0..reps {
            let _ = decide(
                &state,
                &fix,
                &signposts,
                &Policy::default(),
                &granted(),
                &clock(T0),
                0,
            )
            .unwrap();
        }
        let per = t.elapsed().as_micros() / reps;
        eprintln!("{n:>5} signposts: {per:>8} us per call");
    }
    let _ = Outcome::Fire;
}
