//! Fabryk Core — shared types, traits, errors, and utilities.
//!
//! This crate provides the foundational types used across all Fabryk crates.
//! It has no internal Fabryk dependencies (dependency level 0).
//!
//! # Modules
//!
//! - [`error`]: Error types and Result alias
//! - [`util`]: File, path, and ID utilities

#![doc = include_str!("../README.md")]

pub mod error;
pub mod util;

// Re-export key types at crate root for convenience
pub use error::{Error, Result};

// Convenience re-exports from util
pub use util::ids::{id_from_path, normalize_id};
pub use util::resolver::PathResolver;

// Modules to be added during extraction:
// pub mod traits;
// pub mod state;
// pub mod resources;
