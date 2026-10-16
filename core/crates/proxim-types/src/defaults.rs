//! Provisional defaults [P] (spec 1, 3.3). Data, never hard-coded in logic.

pub const ACC_FLOOR_MM: u32 = 10_000;
pub const KAPPA_PERMILLE: u32 = 1000;
pub const K_IN_PERMILLE: u32 = 1000;
pub const K_OUT_PERMILLE: u32 = 1000;
pub const SENSITIVITY_CEILING: u8 = 2;
pub const LEAD_S_MIN: u32 = 15;
pub const LEAD_S_MAX: u32 = 120;
pub const LEAD_S_DEFAULT: u32 = 40;
pub const CAP_SIGNPOST_DAY: u32 = 2;
pub const CAP_SIGNPOST_WEEK: u32 = 5;
pub const APP_MIN_INTERVAL_S: u32 = 1800;
pub const SLOTS: u8 = 1;
pub const MAX_ACC_MM: u32 = 100_000;
pub const STANDOFF_M: u32 = 1000;
pub const D_HI_M: u32 = 3000;
pub const D_HI_MAX_M: u32 = 5000;
pub const BEARING_TOL_DDEG: u16 = 300;
pub const MIN_CIRCLE_RADIUS_M: u32 = 150;
pub const MAX_SIGNPOSTS: usize = 4096;
pub const MAX_ROUTE_VERTICES: usize = 1024;
pub const MAX_POLYGON_VERTICES: usize = 256;
pub const MAX_STANDOFF_ZONES: usize = 256;
/// Most entries in `categories_accepted` and in `category_affinity` (spec 7.1).
pub const MAX_CATEGORY_RULES: usize = 256;
/// Neutral and largest category weight (spec 7.1).
pub const AFFINITY_NEUTRAL_MILLI: u16 = 1000;
pub const AFFINITY_MAX_MILLI: u16 = 3000;
pub const MAX_TRACE: usize = 16;
pub const MAX_DETECTORS: usize = 4096;
pub const MAX_FRAME_MM: i64 = 150_000_000;
pub const MAX_ABS_LAT_E7: i32 = 800_000_000;

// Strict JSON parser bounds (spec 3.2: bounded sizes and depth; every input is untrusted).
/// Largest accepted JSON text in bytes (4096 signposts of up to 1024 vertices each fit).
pub const MAX_JSON_BYTES: usize = 33_554_432;
/// Deepest container nesting accepted (the deepest real document is about 8).
pub const MAX_JSON_DEPTH: usize = 32;
/// Longest decoded string in bytes.
pub const MAX_JSON_STRING_BYTES: usize = 8192;
/// Most elements in one array.
pub const MAX_JSON_ARRAY_LEN: usize = 8192;
/// Most keys in one object.
pub const MAX_JSON_OBJECT_KEYS: usize = 256;

// ---- proxim-path (spec 5): filters, tiers, CLOSING, along window ----

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
/// Foot cap with strong evidence (5.3, errata E-P-FOOT): both bonuses earned and the worst
/// `acc_eff` of the last three fixes at most [`P1_FOOT_STRONG_ACC_MM`]. Equals the health floor
/// [`MIN_CONF_HEALTH`], so a walker can receive a health approach only on strong evidence.
pub const P1_CONF_FOOT_STRONG_CAP: u16 = 700;
/// See [`P1_CONF_FOOT_STRONG_CAP`].
pub const P1_FOOT_STRONG_ACC_MM: i64 = 20_000;
/// P0 (presence) confidence, permille (5.3).
pub const P0_CONF: u16 = 500;
/// Default `min_path_conf_milli` for a general signpost (spec 1).
pub const MIN_CONF_GENERAL: u16 = 500;
/// Health floor (spec 5.3: 'Health's floor 700'; PLAN 5.5). A health signpost's
/// `min_path_conf_milli` defaults to it and is never lowered below it (errata E-D-HEALTH).
pub const MIN_CONF_HEALTH: u16 = 700;
/// Fixes needed for the five-fix bonus.
pub const P1_BONUS_FIXES: u8 = 5;

/// CLOSING: the net advance over the time span must reach `max(this, acc_eff / 2)` mm (5.3,
/// errata E-P12).
pub const CLOSING_MIN_ADVANCE_MM: i64 = 5000;
/// CLOSING: the baseline fix must be at least this much older than the newest fix, ms. A walker at
/// 1.4 m/s gains only 2.8 m in 2 s, so a count of fixes cannot work for foot; a time span can.
pub const CLOSING_MIN_SPAN_MS: i64 = 4000;
/// CLOSING: a baseline older than this is no evidence of the current motion, ms (errata E-P12;
/// equals the P1 fix window).
pub const CLOSING_MAX_SPAN_MS: i64 = 60_000;
/// Along-route window behind the previous fix, mm (6.2).
pub const WINDOW_BACK_MM: i64 = 100_000;
/// Along-route window ahead of the previous fix, fixed part, mm (6.2).
pub const WINDOW_AHEAD_MM: i64 = 500_000;
/// Along-route window ahead, speed part: `v_eff` times this many seconds (6.2).
pub const WINDOW_AHEAD_S: i64 = 30;

// ---- proxim-detect (spec 6): ENTER/EXIT and APPROACH ----

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
