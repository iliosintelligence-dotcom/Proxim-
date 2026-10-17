//! Engine spec 7.5 (engine_spec 2, M3 slice 4): the full score S = V x Q x R x P, the pacing
//! draw and slots above one.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proxim_types::{Class, Outcome, ReasonCode, Signpost};

fn visit_once(sim: &mut Sim, t0: i64) -> (Vec<String>, Vec<(String, ReasonCode)>) {
    let outs = sim.run(&enter_cycle(t0, !sim.state.mem.det.is_empty()));
    let fired = outs
        .iter()
        .flat_map(|o| o.decisions.iter())
        .filter(|d| d.outcome == Outcome::Fire)
        .map(|d| d.signpost.as_str().to_string())
        .collect();
    (fired, outs.iter().flat_map(drop_reasons).collect())
}

#[test]
fn quality_breaks_a_rank_tie() {
    let mut low = circle("a-low", 200);
    low.quality_milli = Some(600);
    let mut high = circle("b-high", 200);
    high.quality_milli = Some(900);
    let mut sim = Sim::new(vec![low, high]);
    let (fired, drops) = visit_once(&mut sim, T0);
    assert_eq!(
        fired,
        ["b-high"],
        "the id would pick a-low; quality picks b-high"
    );
    assert!(drops.contains(&("a-low".into(), ReasonCode::LostSlot)));
}

#[test]
fn money_cannot_outrun_quality_beyond_the_rank_cap() {
    // V is capped at 3x (E-D-RANK): a 3000 rank at the lowest quality (500) scores 1500 units,
    // a 1600 rank at full quality scores 1600 units, so the better advert still wins.
    let mut rich = circle("a-rich", 200);
    rich.rank_milli = Some(1_000_000);
    rich.quality_milli = Some(500);
    let mut good = circle("b-good", 200);
    good.rank_milli = Some(1600);
    let mut sim = Sim::new(vec![rich, good]);
    assert_eq!(visit_once(&mut sim, T0).0, ["b-good"]);
}

fn paced(p: u16) -> Signpost {
    let mut sp = circle("c", 200);
    sp.p_serve_milli = Some(p);
    sp
}

#[test]
fn p_serve_zero_never_passes_and_1000_always_does() {
    let mut sim = Sim::new(vec![paced(0)]);
    let (fired, drops) = visit_once(&mut sim, T0);
    assert!(fired.is_empty());
    assert!(
        drops.contains(&("c".into(), ReasonCode::PacedOut)),
        "{drops:?}"
    );
    let mut sim = Sim::new(vec![paced(1000)]);
    assert_eq!(visit_once(&mut sim, T0).0, ["c"]);
}

#[test]
fn half_pacing_serves_about_half_of_installs_and_never_rerolls() {
    let mut served = 0;
    for seed in 0..400u64 {
        let mut sim = Sim::new(vec![paced(500)]);
        sim.seed = seed;
        let first = !visit_once(&mut sim, T0).0.is_empty();
        served += usize::from(first);
        // The same install trying again inside the same 15-minute bucket gets the same answer.
        if !first {
            let mut again = Sim::new(vec![paced(500)]);
            again.seed = seed;
            assert!(
                visit_once(&mut again, T0).0.is_empty(),
                "seed {seed} rerolled"
            );
        }
    }
    assert!((150..250).contains(&served), "{served} of 400");
}

#[test]
fn health_facility_is_never_paced() {
    let mut route_sp = route("clinic", 2500, 30);
    route_sp.class = Class::HealthFacility;
    route_sp.p_serve_milli = Some(0);
    let mut sim = Sim::new(vec![route_sp]);
    sim.consent = granted_health();
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 40, 0));
    assert_eq!(fire_indices(&outs).len(), 1, "PLAN 6.5: p fixed at 1");
}
