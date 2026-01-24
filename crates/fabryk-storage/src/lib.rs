//! # fabryk-storage
//!
//! Storage backend implementations for Fabryk knowledge fabric.
//!
//! This crate provides multiple storage backends:
//! - Filesystem-primary hybrid storage (markdown + metadata cache)
//! - PostgreSQL/SQLite backends
//! - In-memory storage (for testing)
//! - Storage abstraction traits
//! - Migration and versioning support

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod filesystem;
pub mod database;
pub mod memory;
pub mod traits;

pub use error::{Error, Result};
