//! Utility modules for file operations, path handling, ID computation,
//! and common helpers.
//!
//! # Modules
//!
//! - [`files`]: Async file discovery and reading utilities
//! - [`ids`]: ID normalization and computation
//! - [`paths`]: Generic path utilities (binary location, tilde expansion)
//! - [`resolver`]: Configurable domain path resolution

pub mod files;
pub mod ids;
pub mod paths;
pub mod resolver;
