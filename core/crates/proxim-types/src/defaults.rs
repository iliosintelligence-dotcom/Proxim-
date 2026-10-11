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
pub const MAX_TRACE: usize = 16;
pub const MAX_DETECTORS: usize = 4096;
pub const MAX_FRAME_MM: i64 = 150_000_000;
pub const MAX_ABS_LAT_E7: i32 = 800_000_000;
