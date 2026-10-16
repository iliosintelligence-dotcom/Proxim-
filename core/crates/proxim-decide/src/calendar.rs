//! Local calendar buckets for caps (engine-spec 7). Integer only.

use crate::params::DAY_MS;

/// `(day_idx, week_idx)` of `now_ms` shifted by `tz_offset_min`.
///
/// `day_idx = floor_div(now + tz x 60000, 86_400_000)`; `week_idx = floor_div(day_idx + 3, 7)`
/// (1970-01-01 was a Thursday, so weeks start on Monday). Evaluated in `i128`: no overflow for any
/// `i64` input.
#[must_use]
pub(crate) fn day_week(now_ms: i64, tz_offset_min: i16) -> (i64, i64) {
    let local = i128::from(now_ms) + i128::from(tz_offset_min) * 60_000;
    let day = local.div_euclid(i128::from(DAY_MS));
    let week = (day + 3).div_euclid(7);
    // |day| <= (2^63 + 2^21) / 86.4e6 < 2^63, so both fit; the fallback is unreachable.
    (
        i64::try_from(day).unwrap_or(i64::MAX),
        i64::try_from(week).unwrap_or(i64::MAX),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_thursday_week_zero_and_monday_starts_week_one() {
        assert_eq!(day_week(0, 0), (0, 0)); // Thu 1970-01-01
        assert_eq!(day_week(3 * DAY_MS, 0), (3, 0)); // Sun 1970-01-04 still week 0
        assert_eq!(day_week(4 * DAY_MS, 0), (4, 1)); // Mon 1970-01-05 starts week 1
        assert_eq!(day_week(10 * DAY_MS, 0), (10, 1)); // Sun 1970-01-11
        assert_eq!(day_week(11 * DAY_MS, 0), (11, 2)); // Mon 1970-01-12
    }

    #[test]
    fn negative_times_floor_toward_minus_infinity() {
        assert_eq!(day_week(-1, 0), (-1, 0)); // Wed 1969-12-31
        assert_eq!(day_week(-DAY_MS, 0), (-1, 0));
        assert_eq!(day_week(-DAY_MS - 1, 0), (-2, 0)); // Tue 1969-12-30
        assert_eq!(day_week(-3 * DAY_MS, 0), (-3, 0)); // Mon 1969-12-29
        assert_eq!(day_week(-3 * DAY_MS - 1, 0), (-4, -1)); // Sun 1969-12-28
    }

    #[test]
    fn timezone_moves_the_day_boundary() {
        // 23:30 UTC on day 0 is already day 1 at UTC+1, still day 0 at UTC-1.
        let t = DAY_MS - 30 * 60_000;
        assert_eq!(day_week(t, 0).0, 0);
        assert_eq!(day_week(t, 60).0, 1);
        assert_eq!(day_week(t, -60).0, 0);
        // 00:30 UTC on day 1 is day 0 at UTC-1.
        assert_eq!(day_week(DAY_MS + 30 * 60_000, -60).0, 0);
    }

    #[test]
    fn extremes_do_not_overflow() {
        let (d, w) = day_week(i64::MAX, 840);
        assert!(d > 0 && w > 0);
        let (d, w) = day_week(i64::MIN, -840);
        assert!(d < 0 && w < 0);
    }
}
