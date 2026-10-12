//! Speed and heading filters, path tiers, CLOSING and time to anchor (engine-spec 5).
//!
//! `no_std` + `alloc`, integers only (ADR-001). [`path_update`] is the one stateful step and it
//! touches only `Mem`; the corridor and ETA helpers are pure functions of their arguments.
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod corridor;
pub mod eta;
pub mod filter;
pub mod params;

pub use corridor::{
    P0_CONFIDENCE_MILLI, P1Evidence, Recent, RecentProjection, closing, copy_for_tier, heading_dot,
    p1_confidence, project_recent,
};
pub use eta::{d_rem_mm, fire_distance_mm, t_hat_ms};
pub use filter::{PathSummary, floor_mmps, min_speed_mmps, p0_conf_milli, path_update, v_eff_for};
