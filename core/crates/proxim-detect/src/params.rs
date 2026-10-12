//! Provisional detector numbers of engine-spec 6 as named constants (see `proxim-path::params`
//! for why they are not in `proxim-types::defaults`).

/// ENTER: consecutive `Inside` fixes needed.
pub const ENTER_MIN_FIXES: u32 = 3;
/// ENTER: the first to the last `Inside` fix must span at least this long, ms.
pub const ENTER_MIN_SPAN_MS: i64 = 3000;
/// ENTER: the span needed when `vf` is below [`ENTER_SLOW_MMPS`] or unknown, ms.
pub const ENTER_MIN_SPAN_SLOW_MS: i64 = 10_000;
/// ENTER: speed below which the longer span applies.
pub const ENTER_SLOW_MMPS: u32 = 1500;
/// ENTER: a gap above this between candidate fixes restarts the count, ms.
pub const ENTER_MAX_GAP_MS: i64 = 10_000;
/// Circles need `acc_mm` at most this (6.1).
pub const CIRCLE_MAX_ACC_MM: u32 = 50_000;
/// EXIT: `Outside` fixes in a row that confirm an exit.
pub const EXIT_FIXES: u32 = 5;
/// EXIT: `Outside` fixes that confirm an exit when they span [`EXIT_SPAN_MS`].
pub const EXIT_FEW_FIXES: u32 = 2;
/// See [`EXIT_FEW_FIXES`].
pub const EXIT_SPAN_MS: i64 = 10_000;
/// EXIT: cool-down after an exit, ms.
pub const COOL_MS: i64 = 1_800_000;
/// `in` keeps its outside run across gaps up to this, ms; a longer gap restarts the run.
pub const IN_SURVIVE_GAP_MS: i64 = 120_000;
/// A gap above this in `in` ends it as `cool`, ms.
pub const IN_MAX_GAP_MS: i64 = 900_000;

/// APPROACH: a fix older than this is `FIX_STALE`, ms.
pub const MAX_FIX_AGE_MS: i64 = 10_000;
/// APPROACH: accuracy gate, mm of `acc_eff`.
pub const MAX_ACC_EFF_MM: i64 = 65_000;
/// APPROACH: `t_comp`, ms.
pub const T_COMP_MS: i64 = 3000;
/// APPROACH: two confirming fixes must be this far apart, ms.
pub const CONFIRM_SPACING_MS: i64 = 1000;
/// APPROACH: a gap above this clears the confirmation, ms.
pub const CONFIRM_MAX_GAP_MS: i64 = 5000;
/// APPROACH: `L_min = max(0.4 lead_s, 10 s)`; the 0.4 as ms per lead second, the floor in ms.
pub const L_MIN_MS_PER_LEAD_S: i64 = 400;
/// See [`L_MIN_MS_PER_LEAD_S`].
pub const L_MIN_FLOOR_MS: i64 = 10_000;
/// `d_pass = max(60 m, 5 s x v_eff)`: the floor in mm and the seconds.
pub const D_PASS_MIN_MM: i64 = 60_000;
/// See [`D_PASS_MIN_MM`].
pub const D_PASS_S: i64 = 5;
/// `d_lo` default for vehicles, mm; foot and cycle use [`D_LO_SLOW_MM`] and `3 acc_eff`.
pub const D_LO_VEHICLE_MM: i64 = 150_000;
/// See [`D_LO_VEHICLE_MM`].
pub const D_LO_SLOW_MM: i64 = 100_000;
/// Speed band hysteresis once armed, mmps.
pub const BAND_HYSTERESIS_MMPS: u32 = 1000;
/// Re-arm distance factor `1.2` as a ratio (numerator, denominator).
pub const REARM_NUM: i128 = 12;
/// See [`REARM_NUM`].
pub const REARM_DEN: i128 = 10;
/// Default reach margin beyond the geometry extent, mm (spec 1.2: "+ 100 m").
pub const REACH_MARGIN_MM: i64 = 100_000;
/// Lowest accepted circle radius, m (also in `defaults::MIN_CIRCLE_RADIUS_M`).
pub const BUFFER_MIN_M: u32 = 30;
/// Highest accepted corridor buffer, m.
pub const BUFFER_MAX_M: u32 = 500;
/// Accepted `bearing_tol_ddeg` range.
pub const BEARING_TOL_MIN_DDEG: u16 = 150;
/// See [`BEARING_TOL_MIN_DDEG`].
pub const BEARING_TOL_MAX_DDEG: u16 = 450;
