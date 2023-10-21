#![no_std]
#![feature(error_in_core)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

pub mod fan;
pub mod mcp9808;

pub use self::{fan::Fan, mcp9808::Mcp9808};
