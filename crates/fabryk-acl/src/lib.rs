//! # fabryk-acl
//!
//! Access control implementation for Fabryk knowledge fabric.
//!
//! This crate implements the ACL system for Fabryk:
//! - Identity management
//! - Permission checking (read, write, admin)
//! - Partition-level access control
//! - ACL policy enforcement
//! - Permission inheritance and groups

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod policy;
pub mod enforcement;
pub mod store;

pub use error::{Error, Result};
