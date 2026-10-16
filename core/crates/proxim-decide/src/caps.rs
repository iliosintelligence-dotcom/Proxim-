//! Per-signpost caps with local calendar buckets (engine-spec 7, F22/F23, commit).
//! Sorted by `id` (BTreeMap), integer only.

use crate::calendar::day_week;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use proxim_types::{Caps, Id, SignpostCaps};

/// The caps of one call: loaded (and week-pruned) from `State.caps`, updated by commits.
///
/// Each entry is judged in the timezone it was counted under (`tz_offset_min`, errata E-D-TZ): its
/// "current" day and week are `day_week(now, entry_tz)`. A call that carries a different offset
/// therefore cannot move an entry into a fresh bucket. Legacy entries without an offset are judged
/// under the call's offset.
pub(crate) struct CapsBook {
    map: BTreeMap<String, SignpostCaps>,
    now_ms: i64,
    tz_offset_min: i16,
    day_idx: i64,
    week_idx: i64,
}

impl CapsBook {
    /// Entries whose week (in their own zone) is over are pruned (spec 3.3).
    pub fn load(caps: &Caps, now_ms: i64, tz_offset_min: i16) -> Self {
        let (day_idx, week_idx) = day_week(now_ms, tz_offset_min);
        let mut book = Self {
            map: BTreeMap::new(),
            now_ms,
            tz_offset_min,
            day_idx,
            week_idx,
        };
        for e in &caps.signposts {
            if e.week_idx >= book.current(e).1 {
                book.map.insert(String::from(e.id.as_str()), e.clone());
            }
        }
        book
    }

    /// `(day_idx, week_idx)` of now in the entry's own zone.
    fn current(&self, e: &SignpostCaps) -> (i64, i64) {
        match e.tz_offset_min {
            Some(z) => day_week(self.now_ms, z),
            None => (self.day_idx, self.week_idx),
        }
    }

    /// Fires of `id` in its current day.
    pub fn day_count(&self, id: &str) -> u32 {
        self.map
            .get(id)
            .filter(|e| e.day_idx == self.current(e).0)
            .map_or(0, |e| e.day_n)
    }

    /// Fires of `id` in its current week.
    pub fn week_count(&self, id: &str) -> u32 {
        self.map
            .get(id)
            .filter(|e| e.week_idx == self.current(e).1)
            .map_or(0, |e| e.week_n)
    }

    /// Commit one fire. While the entry's week is still current in its own zone the entry keeps that
    /// zone (and its day/week indices in it); otherwise a new entry starts under the call's zone.
    pub fn record_fire(&mut self, id: &Id) {
        let day = self.day_count(id.as_str()).saturating_add(1);
        let week = self.week_count(id.as_str()).saturating_add(1);
        let (tz, day_idx, week_idx) = match self.map.get(id.as_str()) {
            Some(e) if e.tz_offset_min.is_some() && week > 1 => {
                let (d, w) = self.current(e);
                (e.tz_offset_min, d, w)
            }
            _ => (Some(self.tz_offset_min), self.day_idx, self.week_idx),
        };
        self.map.insert(
            String::from(id.as_str()),
            SignpostCaps {
                id: id.clone(),
                day_idx,
                day_n: day,
                week_idx,
                week_n: week,
                tz_offset_min: tz,
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
    use crate::params::DAY_MS;

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
            tz_offset_min: None,
        }
    }

    #[test]
    fn counts_reset_with_a_new_index_and_accumulate_within_one() {
        let caps = Caps {
            hwm_ms: 0,
            last_fire_ms: None,
            // day 8 is Friday of week 1 (calendar: weeks start Monday, day 4 = Mon of week 1)
            signposts: alloc::vec![entry("a", 8, 2, 1, 4)],
        };
        let mut b = CapsBook::load(&caps, 8 * DAY_MS, 0);
        assert_eq!((b.day_count("a"), b.week_count("a")), (2, 4));
        assert_eq!((b.day_count("zz"), b.week_count("zz")), (0, 0));
        b.record_fire(&id("a"));
        assert_eq!((b.day_count("a"), b.week_count("a")), (3, 5));
        // New day, same week (day 9 = Saturday of week 1): day count restarts, week count carries.
        let mut b = CapsBook::load(&caps, 9 * DAY_MS, 0);
        assert_eq!((b.day_count("a"), b.week_count("a")), (0, 4));
        b.record_fire(&id("a"));
        assert_eq!((b.day_count("a"), b.week_count("a")), (1, 5));
        // New week (day 11 = Monday of week 2): the entry is pruned entirely.
        let b = CapsBook::load(&caps, 11 * DAY_MS, 0);
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn a_later_bucket_counts_as_a_different_bucket() {
        let caps = Caps {
            hwm_ms: 0,
            last_fire_ms: None,
            signposts: alloc::vec![entry("a", 12, 2, 2, 5)],
        };
        // now = day 10 (week 1); the legacy entry sits in a later bucket (day 12, week 2)
        let b = CapsBook::load(&caps, 10 * DAY_MS, 0);
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
