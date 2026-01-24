//! # fabryk-cli
//!
//! Admin CLI for Fabryk knowledge fabric.
//!
//! This crate provides command-line tools for Fabryk administration:
//! - Knowledge item management (create, read, update, delete)
//! - Partition administration
//! - ACL policy management
//! - Import/export operations
//! - Health checks and diagnostics

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod commands;
pub mod config;

pub use error::{Error, Result};
