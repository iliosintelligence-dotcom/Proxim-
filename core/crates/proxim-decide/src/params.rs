//! Provisional numbers of engine-spec 7 that `proxim-types::defaults` does not carry yet.
//! Named constants, never inlined in logic. Moving them into `defaults` is a lead decision
//! (same precedent as errata E-P2/E-P14).

/// F06 `FIX_FROM_FUTURE`: a fix may be at most this far ahead of `now`, ms.
pub const FIX_FUTURE_SLACK_MS: i64 = 5000;
/// F07 `FIX_STALE`: a fix older than this relative to `now` is rejected, ms.
pub const FIX_MAX_AGE_MS: i64 = 30_000;
/// F09: implied speed above this is a jump, mmps.
pub const JUMP_MAX_SPEED_MMPS: i128 = 83_000;
/// F09: and the distance must exceed both accuracies plus this, mm.
pub const JUMP_MARGIN_MM: i64 = 100_000;
/// F09: only a previous fix at most this old is compared, ms.
pub const JUMP_WINDOW_MS: i64 = 60_000;
/// F09: the third consecutive reject resets the trace and detectors and accepts the fix.
pub const JUMP_RESET_AT: u8 = 3;
/// Detector entries absent from the input and unseen for longer than this are pruned, ms.
pub const DET_PRUNE_MS: i64 = 900_000;
/// F20 default `max_event_age_s` for ENTER.
pub const EVENT_AGE_ENTER_S: u32 = 30;
/// F20 default `max_event_age_s` for APPROACH.
pub const EVENT_AGE_APPROACH_S: u32 = 10;
/// F21 default `max_event_dist_m`.
pub const EVENT_DIST_M: u32 = 5000;
/// Default `rank_milli` (1000 = 1.0).
pub const RANK_DEFAULT_MILLI: u32 = 1000;
/// Largest `rank_milli` the engine honours (E-D-RANK): PLAN 5.6 caps the paid value factor at
/// 3x the category floor so money cannot buy unlimited relevance; the device enforces it too.
pub const RANK_MAX_MILLI: u32 = 3000;
/// Ceiling of one zone's standoff, m (keeps the test inside the 150 km frame).
pub const STANDOFF_MAX_M: u32 = 100_000;
/// Largest accepted `tz_offset_min` magnitude (UTC-12 .. UTC+14 fits).
pub const TZ_MAX_MIN: i16 = 840;
/// Milliseconds per local day.
pub const DAY_MS: i64 = 86_400_000;
/// Platform ceiling for the per-app slot count in M1.
/// Most slots a partner may ask for in one call (engine spec 7.5; 1 in engine_spec 1).
pub const SLOTS_CEILING: u8 = 3;

/// Q of the score: the lowest quality prior counted, and the neutral value (engine spec 7.5).
pub const QUALITY_MIN_MILLI: u16 = 500;
pub const QUALITY_MAX_MILLI: u16 = 1000;
/// Timing fit `g` of P: 1000 at the sender's lead, falling linearly to this floor one lead away.
pub const TIMING_FLOOR_MILLI: u64 = 500;
/// Pacing (PLAN 5.6 step 7): one draw per signpost revision per 15-minute bucket.
pub const PACE_BUCKET_MS: i64 = 900_000;
pub const P_SERVE_MAX_MILLI: u16 = 1000;
pub const PACE_DOMAIN: &[u8] = b"PROXIM-PACE-V1\x00";

/// Driving-safe format (PLAN 5.6): above this filtered speed only the glanceable is shown.
pub const DRIVING_MMPS: u32 = 5000;
