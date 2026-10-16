//! Shared builders for the proxim-path tests.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use proxim_geo::{Frame, frame_for};
use proxim_types::{Enu, Fix, Position};

/// Synthetic origin at latitude 45, longitude 7 (invariant I2: not a real place of a person).
pub const ORIGIN: Position = Position {
    lat_e7: 450_000_000,
    lon_e7: 70_000_000,
};

pub fn frame() -> Frame {
    frame_for(ORIGIN).unwrap()
}

/// Position of an ENU offset (mm) from the origin.
pub fn pos(e_mm: i32, n_mm: i32) -> Position {
    frame().unproject(Enu { e_mm, n_mm }).unwrap()
}

/// A fix at an ENU offset with Doppler speed and course.
pub fn fix_at(
    t_ms: i64,
    e_mm: i32,
    n_mm: i32,
    acc_mm: u32,
    speed: Option<u32>,
    course: Option<u16>,
) -> Fix {
    let p = pos(e_mm, n_mm);
    Fix {
        t_ms,
        lat_e7: p.lat_e7,
        lon_e7: p.lon_e7,
        acc_mm,
        speed_mmps: speed,
        speed_acc_mmps: None,
        course_ddeg: course,
        course_acc_ddeg: None,
        is_mock: false,
    }
}
