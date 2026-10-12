//! Measurement report (run with `cargo test -p proxim-detect --test measure -- --ignored --nocapture`).
//! Prints the behaviour of the detectors on synthetic traces; the assertions are only sanity checks.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stderr,
    clippy::cast_possible_truncation
)]

mod common;

use common::{Driver, ROUTE_M, circle, drive_north, fix_at, route, with_mode};
use proxim_types::{DetState, Mode};

const ANCHOR_MM: i64 = ROUTE_M * 1000;

/// Millimetres (or ms) as a decimal string with one fractional digit, integers only.
fn d1(x: i64) -> String {
    let sign = if x < 0 { "-" } else { "" };
    let a = x.abs();
    format!("{sign}{}.{}", a / 1000, (a % 1000) / 100)
}

#[test]
#[ignore = "report generator"]
fn report() {
    eprintln!("== APPROACH steady 1 Hz, lead 40 s, acc 5 m, buffer 30 m, d_hi 3 km ==");
    eprintln!(
        "v_mps mode      D_fire_m armed_at_m confirm_at_m fire_at_m t_hat_fire_s lead_to_anchor_s(true)"
    );
    for (v, mode) in [
        (3_000u32, Mode::Vehicle),
        (4_000, Mode::Vehicle),
        (5_000, Mode::Cycle),
        (10_000, Mode::Vehicle),
        (15_000, Mode::Vehicle),
        (25_000, Mode::Vehicle),
        (29_000, Mode::Vehicle),
        (35_000, Mode::Vehicle),
    ] {
        let mut d = Driver::new(&with_mode(route(30), mode));
        let n = (ANCHOR_MM / i64::from(v)) as usize + 3;
        d.feed_all(&drive_north(0, 0, v, n, 0));
        let armed = d
            .steps
            .iter()
            .position(|s| matches!(s.out.state, Some(DetState::Armed { .. })))
            .unwrap();
        let conf = d
            .steps
            .iter()
            .position(|s| {
                matches!(
                    s.out.state,
                    Some(DetState::Armed {
                        confirm_t: Some(_),
                        ..
                    })
                )
            })
            .unwrap();
        let (fi, ev) = d.events()[0];
        let rem = |i: usize| ANCHOR_MM - i64::from(v) * i as i64;
        let d_fire = proxim_path::fire_distance_mm(
            proxim_path::v_eff_for(mode, Some(v)),
            40,
            3000,
            if mode == Mode::Vehicle {
                150_000
            } else {
                100_000
            },
            3_000_000,
        )
        .unwrap();
        eprintln!(
            "{:>5} {:9} {:>8} {:>11} {:>12} {:>10} {:>12} {:>8}",
            d1(i64::from(v)),
            format!("{mode:?}"),
            d1(d_fire),
            d1(rem(armed)),
            d1(rem(conf)),
            d1(rem(fi)),
            d1(ev.t_hat_ms.unwrap()),
            d1(ev.d_rem_mm.unwrap() * 1000 / i64::from(v)),
        );
    }

    eprintln!("== lateral offset of a parallel road at 20 m/s (buffer 30 m): fires? ==");
    for off in [
        0, 10_000, 20_000, 25_000, 29_000, 30_000, 31_000, 35_000, 60_000,
    ] {
        let mut d = Driver::new(&route(30));
        d.feed_all(&drive_north(0, 0, 20_000, 320, off));
        eprintln!(
            "offset {:5.1} m -> events {}",
            f64::from(off) / 1000.0,
            d.events().len()
        );
    }

    eprintln!("== accuracy vs confidence at 29 m/s ==");
    for acc in [5_000u32, 10_000, 30_000, 40_000, 50_000, 65_000, 66_000] {
        let fixes: Vec<_> = (0..210_i64)
            .map(|i| fix_at(i * 1000, 0, (29_000 * i) as i32, acc, Some(29_000), Some(0)))
            .collect();
        let mut d = Driver::new(&route(30));
        d.feed_all(&fixes);
        eprintln!(
            "acc {:4.0} m -> events {} conf {:?}",
            f64::from(acc) / 1000.0,
            d.events().len(),
            d.events().first().map(|e| e.1.path_conf_milli)
        );
    }

    eprintln!("== ENTER (circle r=150 m, acc 5 m): distance from centre at the event ==");
    for v in [1_500i32, 5_000, 15_000, 29_000] {
        let count = 500_000 / v + 40;
        let fixes: Vec<_> = (0..count)
            .map(|i| {
                fix_at(
                    i64::from(i) * 1000,
                    0,
                    -250_000 + v * i,
                    5000,
                    Some(v as u32),
                    Some(0),
                )
            })
            .collect();
        let mut d = Driver::new(&circle(150));
        d.feed_all(&fixes);
        let ev = d.events();
        let first_inside = d
            .steps
            .iter()
            .position(|s| matches!(s.out.state, Some(DetState::Cand { .. })))
            .unwrap();
        let exit = d
            .steps
            .iter()
            .position(|s| matches!(s.out.state, Some(DetState::Cool { .. })));
        let dist = |i: usize| i64::from((-250_000 + v * i as i32).abs());
        eprintln!(
            "v {:>5} m/s: first inside {:>6} m, ENTER at {:>6} m ({:?} fixes later), exit confirmed at {:?} m",
            d1(i64::from(v)),
            d1(dist(first_inside)),
            ev.first().map_or(String::from("-"), |e| d1(dist(e.0))),
            ev.first().map(|e| e.0 - first_inside),
            exit.map(|i| d1(dist(i))),
        );
    }
    eprintln!("== sampling cadence at 29 m/s: fire distance ==");
    for step_ms in [500i64, 1000, 2000, 3000, 4000, 5000] {
        let n = 6_000_000 / (29 * step_ms) as usize + 3;
        let fixes: Vec<_> = (0..n as i64)
            .map(|i| {
                fix_at(
                    i * step_ms,
                    0,
                    (29 * step_ms * i) as i32,
                    5000,
                    Some(29_000),
                    Some(0),
                )
            })
            .collect();
        let mut d = Driver::new(&route(30));
        d.feed_all(&fixes);
        eprintln!(
            "every {:4} ms -> fire at {:?} m",
            step_ms,
            d.events().first().map(|e| e.1.d_rem_mm.unwrap() / 1000)
        );
    }
}
