#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Workflows Library
//!
//! Restate workflow definitions for ECL.

pub mod simple;

// Re-export core types
pub use ecl_core::{Error, Result};
