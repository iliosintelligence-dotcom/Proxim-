//! Provisional numbers of engine-spec 5 as named constants. They are data, not logic: the spec marks
//! them [P] and wants them in `proxim-types::defaults`; that crate is closed for this stage, so they
//! live here until the lead moves them (see the errata in docs/engine-spec.md).

/// Speed filter time constant (5.1), ms.
pub const SPEED_TAU_MS: i64 = 8000;
/// Heading filter time constant (5.2), ms.
pub const HEADING_TAU_MS: i64 = 4000;
/// `dt` is clamped to `0..=DT_CLAMP_MS` before it enters a filter weight.
pub const DT_CLAMP_MS: i64 = 60_000;
/// A gap above this reseeds both filters (5.1, 5.2).
pub const RESEED_GAP_MS: i64 = 5000;
/// `vf` and the heading are hidden when not refreshed within this long.
pub const STALE_HIDE_MS: i64 = 10_000;
/// Doppler speed is trusted when its accuracy is absent or at most this (5.1).
pub const SPEED_ACC_MAX_MMPS: u32 = 5000;
/// Position-derived speed uses a trace entry aged within this range (5.1), ms.
pub const SPEED_DISP_MIN_AGE_MS: i64 = 3000;
/// See [`SPEED_DISP_MIN_AGE_MS`].
pub const SPEED_DISP_MAX_AGE_MS: i64 = 10_000;
/// Position-derived heading uses a trace entry aged within this range (5.2), ms.
pub const HEADING_DISP_MIN_AGE_MS: i64 = 3000;
/// See [`HEADING_DISP_MIN_AGE_MS`].
pub const HEADING_DISP_MAX_AGE_MS: i64 = 15_000;
/// The OS course is trusted when its accuracy is absent or at most this (5.2), 0.1 degree.
pub const COURSE_ACC_MAX_DDEG: u16 = 450;
/// The OS course is used only when `vf` is at least this (5.2).
pub const HEADING_MIN_SPEED_MMPS: u32 = 800;
/// Displacement heading needs at least `max(this, 3 acc_eff)` mm (5.2).
pub const HEADING_MIN_DISP_MM: i64 = 15_000;

/// Speed floors for `v_eff` (5.1).
pub const FLOOR_VEHICLE_MMPS: u32 = 4000;
/// See [`FLOOR_VEHICLE_MMPS`].
pub const FLOOR_CYCLE_MMPS: u32 = 2000;
/// See [`FLOOR_VEHICLE_MMPS`].
pub const FLOOR_FOOT_MMPS: u32 = 700;
/// Minimum filtered speed for tier P1 (5.3).
pub const MIN_VEHICLE_MMPS: u32 = 2500;
/// See [`MIN_VEHICLE_MMPS`].
pub const MIN_CYCLE_MMPS: u32 = 1500;
/// See [`MIN_VEHICLE_MMPS`].
pub const MIN_FOOT_MMPS: u32 = 500;
/// Default upper edge of the foot speed band (6.2).
pub const MAX_FOOT_MMPS: u32 = 2500;

/// P1 needs `acc_eff` at most this on the last three fixes (5.3), mm.
pub const P1_ACC_EFF_MAX_MM: i64 = 65_000;
/// P1 needs this many fixes within [`P1_WINDOW_MS`].
pub const P1_MIN_FIXES: u8 = 3;
/// See [`P1_MIN_FIXES`].
pub const P1_WINDOW_MS: i64 = 60_000;
/// Heading gate of P1: within 45 degrees of the route tangent (5.3), 0.1 degree.
pub const P1_HEADING_TOL_DDEG: u16 = 450;
/// Bonus heading tolerance: within 15 degrees (5.3), 0.1 degree.
pub const P1_BONUS_HEADING_TOL_DDEG: u16 = 150;
/// P1 base confidence, permille (5.3).
pub const P1_CONF_BASE: u16 = 600;
/// Bonus for five or more fixes in the window, and for a tight heading.
pub const P1_CONF_BONUS: u16 = 100;
/// Penalty when the worst `acc_eff` of the last three fixes exceeds [`P1_ACC_PENALTY1_MM`].
pub const P1_CONF_PENALTY1: u16 = 100;
/// Penalty when it exceeds [`P1_ACC_PENALTY2_MM`] (replaces, does not add to, the first).
pub const P1_CONF_PENALTY2: u16 = 200;
/// See [`P1_CONF_PENALTY1`].
pub const P1_ACC_PENALTY1_MM: i64 = 30_000;
/// See [`P1_CONF_PENALTY2`].
pub const P1_ACC_PENALTY2_MM: i64 = 50_000;
/// Confidence cap for the foot mode (5.3).
pub const P1_CONF_FOOT_CAP: u16 = 600;
/// P0 (presence) confidence, permille (5.3).
pub const P0_CONF: u16 = 500;
/// Fixes needed for the five-fix bonus.
pub const P1_BONUS_FIXES: u8 = 5;

/// CLOSING: the net advance over three fixes must reach `max(this, tol)` mm (5.3).
pub const CLOSING_MIN_ADVANCE_MM: i64 = 5000;
/// Along-route window behind the previous fix, mm (6.2).
pub const WINDOW_BACK_MM: i64 = 100_000;
/// Along-route window ahead of the previous fix, fixed part, mm (6.2).
pub const WINDOW_AHEAD_MM: i64 = 500_000;
/// Along-route window ahead, speed part: `v_eff` times this many seconds (6.2).
pub const WINDOW_AHEAD_S: i64 = 30;
