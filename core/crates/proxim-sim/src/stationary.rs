//! The stationary-outside statistical check (ADDENDUM-01 M1): a phone standing 25 m outside a
//! circle for 10 minutes under open-sky noise. Target [MODEL, provisional]: 0% false enters over
//! 1,000 seeds and at most 4 detector edge transitions per 10 minutes. Also reports the naive
//! flapping (the reported centre crossing the edge) that the detector suppresses.

use crate::runner::ScenarioResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StationaryStats {
    pub seeds: u64,
    /// Runs in which the signpost fired at least once (every one is a false enter: the truth
    /// never enters).
    pub false_enter_runs: u64,
    pub max_transitions: u32,
    pub max_naive_transitions: u32,
    /// Sum of naive transitions over all runs (mean = sum / seeds).
    pub naive_transitions_sum: u64,
}

impl StationaryStats {
    /// Summarise the expectation at `index` (scenario order) of a scenario result.
    #[must_use]
    pub fn from_result(r: &ScenarioResult, index: usize) -> Self {
        let mut s = Self {
            seeds: r.runs,
            false_enter_runs: 0,
            max_transitions: 0,
            max_naive_transitions: 0,
            naive_transitions_sum: 0,
        };
        for v in &r.verdicts {
            if let Some(sp) = v.signposts.get(index) {
                if !sp.fires.is_empty() {
                    s.false_enter_runs += 1;
                }
                s.max_transitions = s.max_transitions.max(sp.transitions);
                s.max_naive_transitions = s.max_naive_transitions.max(sp.naive_transitions);
                s.naive_transitions_sum += u64::from(sp.naive_transitions);
            }
        }
        s
    }
}
