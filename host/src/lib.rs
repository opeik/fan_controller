#![cfg_attr(not(test), no_std)]
#![feature(error_in_core)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::wildcard_imports
)]

pub use uom::si::f64 as units;
pub mod decode;
pub mod fan_curve;
