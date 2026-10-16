//! Helpers shared by the proxim-decide integration tests.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

#[path = "../../examples/support/fixtures.rs"]
pub mod fixtures;

pub use fixtures::*;

use proxim_decide::decide;
use proxim_types::{
    Clock, Consent, Decision, Fix, Outcome, Output, Policy, ReasonCode, Signpost, State,
};

/// Threads `State` through successive `decide` calls.
pub struct Sim {
    pub state: State,
    pub signposts: Vec<Signpost>,
    pub policy: Policy,
    pub consent: Consent,
    pub seed: u64,
    pub tz: i16,
}

impl Sim {
    pub fn new(signposts: Vec<Signpost>) -> Self {
        Self {
            state: State::empty(),
            signposts,
            policy: explain(),
            consent: granted(),
            seed: 0,
            tz: 0,
        }
    }

    /// One call with `now = fix.t_ms`.
    pub fn step(&mut self, fix: &Fix) -> Output {
        self.step_at(fix, fix.t_ms)
    }

    pub fn step_at(&mut self, fix: &Fix, now_ms: i64) -> Output {
        let clock = Clock {
            now_ms,
            tz_offset_min: self.tz,
        };
        let (out, next) = decide(
            &self.state,
            fix,
            &self.signposts,
            &self.policy,
            &self.consent,
            &clock,
            self.seed,
        )
        .unwrap();
        self.state = next;
        out
    }

    pub fn run(&mut self, fixes: &[Fix]) -> Vec<Output> {
        fixes.iter().map(|f| self.step(f)).collect()
    }
}

pub fn fires(o: &Output) -> Vec<&Decision> {
    o.decisions
        .iter()
        .filter(|d| d.outcome == Outcome::Fire)
        .collect()
}

pub fn drop_reasons(o: &Output) -> Vec<(String, ReasonCode)> {
    o.decisions
        .iter()
        .filter(|d| d.outcome == Outcome::Drop)
        .map(|d| (d.signpost.to_string(), d.reason))
        .collect()
}

pub fn fire_indices(outs: &[Output]) -> Vec<usize> {
    outs.iter()
        .enumerate()
        .filter(|(_, o)| !fires(o).is_empty())
        .map(|(i, _)| i)
        .collect()
}

pub fn has_drop(o: &Output, id: &str, reason: ReasonCode) -> bool {
    o.decisions
        .iter()
        .any(|d| d.outcome == Outcome::Drop && d.signpost.as_str() == id && d.reason == reason)
}
