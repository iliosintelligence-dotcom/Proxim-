//! E3: `decide` is a pure function of its arguments. No clock, no randomness, no hidden state,
//! no dependence on input order, on the seed (M1) or on `explain` (state only).
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

mod common;

use common::*;
use proptest::prelude::*;
use proxim_decide::decide;
use proxim_types::canon::to_canonical_json;
use proxim_types::{Output, Policy, Signpost, State};

/// Run a whole trace; returns canonical output texts and the final state text.
fn run_trace(
    signposts: &[Signpost],
    policy: &Policy,
    seed: u64,
    fixes: &[proxim_types::Fix],
) -> (Vec<String>, String) {
    let mut state = State::empty();
    let mut outs = Vec::new();
    for f in fixes {
        let (o, s) = decide(
            &state,
            f,
            signposts,
            policy,
            &granted_health(),
            &clock(f.t_ms),
            seed,
        )
        .unwrap();
        outs.push(to_canonical_json(&o).unwrap());
        state = s;
    }
    (outs, to_canonical_json(&state).unwrap())
}

#[test]
fn decide_is_a_pure_function_of_its_arguments() {
    // Same arguments, same result, call after call, from clones, and the inputs are untouched.
    let signposts = world();
    let policy = explain();
    let consent = granted_health();
    let mut fixes = tour(T0);
    fixes.extend(random_walk(7, 60, T0 + 8 * 3_600_000));
    let mut state = State::empty();
    let mut any_decision = false;
    for f in &fixes {
        let (state_b, signposts_b, policy_b, consent_b) = (
            state.clone(),
            signposts.clone(),
            policy.clone(),
            consent.clone(),
        );
        let clk = clock(f.t_ms);
        let first = decide(&state, f, &signposts, &policy, &consent, &clk, 3).unwrap();
        let second = decide(&state, f, &signposts, &policy, &consent, &clk, 3).unwrap();
        let third = decide(&state_b, f, &signposts_b, &policy_b, &consent_b, &clk, 3).unwrap();
        assert_eq!(first, second);
        assert_eq!(first, third);
        assert_eq!(state, state_b, "decide must not mutate its input state");
        assert_eq!(signposts, signposts_b);
        any_decision |= !first.0.decisions.is_empty();
        state = first.1;
    }
    assert!(any_decision, "the walk must exercise the decision path");
}

#[test]
fn replay_from_text_gives_identical_bytes() {
    // The canonical text of a state is a complete description: restarting from it is invisible.
    let signposts = world();
    let policy = explain();
    let mut fixes = tour(T0);
    fixes.extend(random_walk(11, 40, T0 + 8 * 3_600_000));
    let (full_outs, full_state) = run_trace(&signposts, &policy, 0, &fixes);
    let mut state = State::empty();
    let mut outs = Vec::new();
    for f in &fixes {
        let text = to_canonical_json(&state).unwrap();
        let reparsed: State = proxim_types::strict::from_canonical_json(text.as_bytes()).unwrap();
        let (o, s) = decide(
            &reparsed,
            f,
            &signposts,
            &policy,
            &granted_health(),
            &clock(f.t_ms),
            0,
        )
        .unwrap();
        outs.push(to_canonical_json(&o).unwrap());
        state = s;
    }
    assert_eq!(outs, full_outs);
    assert_eq!(to_canonical_json(&state).unwrap(), full_state);
}

fn permutation(n: usize, seed: u64) -> Vec<usize> {
    let mut rng = Splitmix(seed);
    let mut v: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = rng.range(0, i as i64) as usize;
        v.swap(i, j);
    }
    v
}

#[test]
fn decide_ignores_input_order() {
    let signposts = world();
    let policy = explain();
    let mut fixes = tour(T0);
    fixes.extend(random_walk(21, 80, T0 + 8 * 3_600_000));
    let reference = run_trace(&signposts, &policy, 0, &fixes);
    assert!(
        reference.0.iter().any(|o| o.contains("\"fire\"")),
        "the reference trace must fire at least once or the test proves nothing"
    );
    for seed in 0..12 {
        let perm = permutation(signposts.len(), seed);
        let shuffled: Vec<Signpost> = perm.iter().map(|&i| signposts[i].clone()).collect();
        assert_eq!(
            run_trace(&shuffled, &policy, 0, &fixes),
            reference,
            "perm {perm:?}"
        );
    }
    // Reversed as well, and with a duplicate-id superseded revision appended at either end.
    let reversed: Vec<Signpost> = signposts.iter().rev().cloned().collect();
    assert_eq!(run_trace(&reversed, &policy, 0, &fixes), reference);
}

#[test]
fn decide_ignores_input_order_with_revisions_and_duplicates() {
    // Two revisions of one id plus an ambiguous pair: the outcome must not depend on the order.
    let mut a1 = circle("a", 200);
    a1.rev = 1;
    let mut a2 = circle("a", 200);
    a2.rev = 2;
    let b1 = circle("b", 200);
    let b1_twin = circle("b", 300);
    let set = vec![a1, a2, b1, b1_twin];
    let fixes: Vec<_> = (0..8).map(|i| still(T0 + i * 4000, 0, 0)).collect();
    let reference = run_trace(&set, &explain(), 0, &fixes);
    for seed in 0..24 {
        let perm = permutation(set.len(), seed);
        let shuffled: Vec<Signpost> = perm.iter().map(|&i| set[i].clone()).collect();
        assert_eq!(run_trace(&shuffled, &explain(), 0, &fixes), reference);
    }
}

/// engine_spec 2 (7.5): the seed is the install's pacing salt, read only by the pacing draw of a
/// signpost with `p_serve_milli < 1000`. Without pacing it has no effect at all.
#[test]
fn seed_has_no_effect_without_pacing() {
    let signposts = world();
    let policy = explain();
    let mut fixes = tour(T0);
    fixes.extend(random_walk(33, 50, T0 + 8 * 3_600_000));
    let reference = run_trace(&signposts, &policy, 0, &fixes);
    for seed in [1, 2, 0xDEAD_BEEF, u64::MAX, 1 << 63] {
        assert_eq!(
            run_trace(&signposts, &policy, seed, &fixes),
            reference,
            "seed {seed}"
        );
    }
}

#[test]
fn explain_changes_only_the_output_never_the_state() {
    let signposts = world();
    let mut fixes = tour(T0);
    fixes.extend(random_walk(44, 80, T0 + 8 * 3_600_000));
    let quiet = Policy::default();
    let loud = explain();
    let mut sq = State::empty();
    let mut sl = State::empty();
    let mut quiet_fires = 0;
    for f in &fixes {
        let c = clock(f.t_ms);
        let (oq, nq) = decide(&sq, f, &signposts, &quiet, &granted_health(), &c, 0).unwrap();
        let (ol, nl) = decide(&sl, f, &signposts, &loud, &granted_health(), &c, 0).unwrap();
        assert_eq!(nq, nl, "state must not depend on explain");
        // Without explain: fires only, no notes. With explain the same fires come first.
        assert!(oq.notes.is_none());
        assert!(
            oq.decisions
                .iter()
                .all(|d| d.outcome == proxim_types::Outcome::Fire)
        );
        let loud_fires: Vec<_> = ol
            .decisions
            .iter()
            .filter(|d| d.outcome == proxim_types::Outcome::Fire)
            .cloned()
            .collect();
        assert_eq!(oq.decisions, loud_fires);
        quiet_fires += oq.decisions.len();
        sq = nq;
        sl = nl;
    }
    assert!(quiet_fires > 0);
}

fn fire_count(outs: &[Output]) -> usize {
    outs.iter().map(|o| fires(o).len()).sum()
}

#[test]
fn a_known_trace_fires_the_same_decisions_every_run() {
    // A pinned behavioural anchor on top of the byte-level goldens: drive the approach route.
    let mut sim = Sim::new(vec![route("r-1", 2500, 30)]);
    let outs = sim.run(&drive_north(T0, 1_400_000, 15_000, 60, 0));
    assert_eq!(fire_count(&outs), 1);
    assert_eq!(fire_indices(&outs), vec![32]);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, failure_persistence: None, rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha, rng_seed: proptest::test_runner::RngSeed::Fixed(0x5eed202b), ..ProptestConfig::default() })]

    #[test]
    fn prop_pure_over_random_walks(seed in any::<u64>(), n in 20usize..90) {
        let signposts = world();
        let mut fixes = tour(T0);
        fixes.extend(random_walk(seed, n, T0 + 8 * 3_600_000));
        let a = run_trace(&signposts, &explain(), 5, &fixes);
        let b = run_trace(&signposts, &explain(), 5, &fixes);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn prop_order_and_seed_invariance(seed in any::<u64>(), perm_seed in any::<u64>(), call_seed in any::<u64>()) {
        let signposts = world();
        let mut fixes = tour(T0);
        fixes.extend(random_walk(seed, 40, T0 + 8 * 3_600_000));
        let reference = run_trace(&signposts, &explain(), 0, &fixes);
        let perm = permutation(signposts.len(), perm_seed);
        let shuffled: Vec<Signpost> = perm.iter().map(|&i| signposts[i].clone()).collect();
        prop_assert_eq!(run_trace(&shuffled, &explain(), call_seed, &fixes), reference);
    }
}
