//! Per-signpost caps with local calendar buckets (engine-spec 7, F22/F23, commit).
//! Sorted by `id` (BTreeMap), integer only.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::{Caps, Id, SignpostCaps};

/// The caps of one call: loaded (and week-pruned) from `State.caps`, updated by commits.
pub(crate) struct CapsBook {
    map: BTreeMap<String, SignpostCaps>,
    day_idx: i64,
    week_idx: i64,
}

impl CapsBook {
    /// Entries from earlier weeks are pruned (spec 3.3). An entry from a LATER index than now
    /// (only possible when the timezone moves west; the clock cannot, see `hwm_ms`) is kept and
    /// counts as a different bucket, as the spec's "a new index resets the count" says.
    pub fn load(caps: &Caps, day_idx: i64, week_idx: i64) -> Self {
        let map = caps
            .signposts
            .iter()
            .filter(|e| e.week_idx >= week_idx)
            .map(|e| (String::from(e.id.as_str()), e.clone()))
            .collect();
        Self {
            map,
            day_idx,
            week_idx,
        }
    }

    /// Fires of `id` in the current local day.
    pub fn day_count(&self, id: &str) -> u32 {
        self.map
            .get(id)
            .filter(|e| e.day_idx == self.day_idx)
            .map_or(0, |e| e.day_n)
    }

    /// Fires of `id` in the current local week.
    pub fn week_count(&self, id: &str) -> u32 {
        self.map
            .get(id)
            .filter(|e| e.week_idx == self.week_idx)
            .map_or(0, |e| e.week_n)
    }

    /// Commit one fire: a new index resets that count to 1.
    pub fn record_fire(&mut self, id: &Id) {
        let day = self.day_count(id.as_str()).saturating_add(1);
        let week = self.week_count(id.as_str()).saturating_add(1);
        self.map.insert(
            String::from(id.as_str()),
            SignpostCaps {
                id: id.clone(),
                day_idx: self.day_idx,
                day_n: day,
                week_idx: self.week_idx,
                week_n: week,
            },
        );
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Entries in ascending `id` order.
    pub fn into_vec(self) -> Vec<SignpostCaps> {
        self.map.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Id {
        Id::new(s).unwrap()
    }

    fn entry(s: &str, day: i64, dn: u32, week: i64, wn: u32) -> SignpostCaps {
        SignpostCaps {
            id: id(s),
            day_idx: day,
            day_n: dn,
            week_idx: week,
            week_n: wn,
        }
    }

    #[test]
    fn counts_reset_with_a_new_index_and_accumulate_within_one() {
        let caps = Caps {
            hwm_ms: 0,
            last_fire_ms: None,
            signposts: alloc::vec![entry("a", 10, 2, 1, 4)],
        };
        let mut b = CapsBook::load(&caps, 10, 1);
        assert_eq!((b.day_count("a"), b.week_count("a")), (2, 4));
        assert_eq!((b.day_count("zz"), b.week_count("zz")), (0, 0));
        b.record_fire(&id("a"));
        assert_eq!((b.day_count("a"), b.week_count("a")), (3, 5));
        // New day, same week: day count restarts, week count carries.
        let mut b = CapsBook::load(&caps, 11, 1);
        assert_eq!((b.day_count("a"), b.week_count("a")), (0, 4));
        b.record_fire(&id("a"));
        assert_eq!((b.day_count("a"), b.week_count("a")), (1, 5));
        // New week: the entry is pruned entirely.
        let b = CapsBook::load(&caps, 17, 2);
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn a_later_bucket_counts_as_a_different_bucket() {
        let caps = Caps {
            hwm_ms: 0,
            last_fire_ms: None,
            signposts: alloc::vec![entry("a", 12, 2, 2, 5)],
        };
        let b = CapsBook::load(&caps, 11, 1);
        assert_eq!(b.len(), 1, "kept: not an earlier week");
        assert_eq!((b.day_count("a"), b.week_count("a")), (0, 0));
    }

    #[test]
    fn output_is_sorted_by_id() {
        let mut b = CapsBook::load(&Caps::default(), 0, 0);
        for s in ["m", "B", "a", "z"] {
            b.record_fire(&id(s));
        }
        let ids: Vec<_> = b
            .into_vec()
            .into_iter()
            .map(|e| String::from(e.id.as_str()))
            .collect();
        assert_eq!(ids, ["B", "a", "m", "z"]);
    }
}
