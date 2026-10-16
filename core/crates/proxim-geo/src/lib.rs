//! Integer geometry (engine-spec 1-4). `no_std` + `alloc`. Floats exist only in [`trig`] and
//! [`h3`]; everything else is exact integer arithmetic with checked overflow.
//!
//! - [`frame_for`] / [`Frame::project`]: e7 degrees to integer ENU mm around a signpost origin.
//! - [`point_in_polygon`], [`signed_depth_polygon_mm`], [`signed_depth_circle_mm`], [`classify`]:
//!   the accuracy-aware Inside / Straddle / Outside decision.
//! - [`Polyline`], [`project_to_polyline`]: cross-track, along-route distance, corridor tests.
//! - [`h3`]: H3 cells via `h3o` for compact regional candidate sets.
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod frame;
pub mod h3;
pub mod line;
pub mod prim;
pub mod seg;
pub mod shape;
pub mod trig;

pub use frame::{Frame, frame_for};
pub use line::{Polyline, Projection, dot_q30, normalize_q30, project_to_polyline, rem_along_mm};
pub use prim::{dist_mm, div_round, isqrt, orient2d, rsr};
pub use seg::dist_point_segment_mm;
pub use shape::{
    Depth, PipResult, acc_eff_mm, classify, point_in_polygon, signed_depth_circle_mm,
    signed_depth_polygon_mm,
};
pub use trig::{cos_tol_q30, unit_from_course};
