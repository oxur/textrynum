//! # fabryk-core
//!
//! Core types, traits, and errors for Fabryk knowledge fabric.
//!
//! This crate provides the foundational abstractions used across all Fabryk components:
//! - Knowledge item types and metadata
//! - Partition and tag definitions
//! - Core traits for storage, query, and ACL
//! - Common error types
//! - Identity and permission types

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod identity;
pub mod item;
pub mod partition;
pub mod tag;
pub mod traits;

pub use error::{Error, Result};
