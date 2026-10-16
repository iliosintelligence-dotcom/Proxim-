//! Wire types for the Proxim decision core (engine-spec section 3, ADR-001).
//!
//! `no_std` + `alloc`: no clocks, files, threads or hash maps exist here. No float
//! appears in any type (spec 1.1). Every field name carries its unit.
#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod canon;
pub mod defaults;
pub mod error;
pub mod model;
pub mod reason;
pub mod strict;
pub mod taxonomy;

pub use error::EngineError;
pub use model::*;
pub use reason::ReasonCode;

/// Engine specification version carried in golden headers (spec 9).
pub const ENGINE_SPEC: u32 = 1;
