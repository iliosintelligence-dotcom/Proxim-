//! Local time windows for dayparts and quiet hours (engine spec 7.6). Integer only.

use proxim_types::Window;

const DAY_MS: i64 = 86_400_000;
pub const MINUTES_PER_DAY: u16 = 1440;
/// Most windows in one list (a daypart schedule or the quiet hours).
pub const MAX_WINDOWS: usize = 16;

/// Local minute of the day and weekday (0 = Monday) at `now_ms` with offset `tz_offset_min`.
#[must_use]
pub fn local(now_ms: i64, tz_offset_min: i16) -> (u16, u8) {
    let t = now_ms.saturating_add(i64::from(tz_offset_min) * 60_000);
    let day = t.div_euclid(DAY_MS);
    let minute = u16::try_from(t.rem_euclid(DAY_MS) / 60_000).unwrap_or(0);
    // 1970-01-01 was a Thursday (weekday 3 with Monday = 0).
    let dow = u8::try_from((day + 3).rem_euclid(7)).unwrap_or(0);
    (minute, dow)
}

const fn on(days: u8, dow: u8) -> bool {
    days & 0x7f == 0 || days & (1 << dow) != 0
}

/// Whether `minute` on weekday `dow` falls inside `w`. A window past midnight belongs to the day it
/// starts on.
#[must_use]
pub fn contains(w: &Window, minute: u16, dow: u8) -> bool {
    let prev = (dow + 6) % 7;
    if w.start_min == w.end_min {
        on(w.days, dow)
    } else if w.start_min < w.end_min {
        on(w.days, dow) && w.start_min <= minute && minute < w.end_min
    } else {
        (on(w.days, dow) && minute >= w.start_min) || (on(w.days, prev) && minute < w.end_min)
    }
}

/// Minutes within a day, bit 7 of `days` clear, and at most [`MAX_WINDOWS`] windows.
#[must_use]
pub fn valid(ws: &[Window]) -> bool {
    ws.len() <= MAX_WINDOWS
        && ws.iter().all(|w| {
            w.start_min <= MINUTES_PER_DAY && w.end_min <= MINUTES_PER_DAY && w.days & 0x80 == 0
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(days: u8, start_min: u16, end_min: u16) -> Window {
        Window {
            days,
            start_min,
            end_min,
        }
    }

    #[test]
    fn local_time_and_weekday() {
        // 2026-10-12 was a Monday; 00:00 UTC.
        let mon = 1_791_763_200_000;
        assert_eq!(local(mon, 0), (0, 0));
        assert_eq!(local(mon + 8 * 3_600_000 + 30 * 60_000, 0), (510, 0));
        assert_eq!(local(mon, -60), (1380, 6), "UTC-1: still Sunday 23:00");
        assert_eq!(
            local(mon + 23 * 3_600_000, 120),
            (60, 1),
            "UTC+2: Tuesday 01:00"
        );
    }

    #[test]
    fn windows_by_day_and_across_midnight() {
        let lunch = w(0b001_1111, 690, 840); // weekdays 11:30-14:00
        assert!(contains(&lunch, 700, 0) && !contains(&lunch, 700, 5));
        assert!(!contains(&lunch, 840, 0), "end is exclusive");
        let night = w(0, 1260, 480); // every day 21:00-08:00
        assert!(contains(&night, 1300, 3) && contains(&night, 100, 3) && !contains(&night, 600, 3));
        let fri_night = w(1 << 4, 1320, 120); // Friday 22:00 to Saturday 02:00
        assert!(contains(&fri_night, 1330, 4) && contains(&fri_night, 60, 5));
        assert!(
            !contains(&fri_night, 60, 4),
            "Friday 01:00 belongs to Thursday's window"
        );
        assert!(
            contains(&w(1, 0, 0), 900, 0) && !contains(&w(1, 0, 0), 900, 1),
            "whole Monday"
        );
        assert!(valid(&[lunch, night]) && !valid(&[w(0, 0, 1441)]) && !valid(&[w(0x80, 0, 1)]));
    }
}
