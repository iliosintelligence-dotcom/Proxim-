//! Black-box conformance support for the Proxim engine.
//!
//! Everything here is written from `docs/engine-spec.md` and uses only public APIs
//! (`proxim_ffi::decide_json`, `proxim_geo`, `proxim_types`). Nothing in this crate calls
//! engine-internal code to compute an expected value: the canonical-JSON checker, the SHA-256 and
//! the reference trigonometry are independent re-implementations, so a bug shared with the engine
//! cannot hide here.
//!
//! This crate is test support: panics on malformed test input are intended.
#![forbid(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

pub mod canon;
pub mod fuzz;
pub mod rng;
pub mod sha;
pub mod world;

pub use canon::{check_canonical, render};
pub use world::*;
