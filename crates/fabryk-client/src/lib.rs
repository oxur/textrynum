//! # fabryk-client
//!
//! Rust client library for Fabryk knowledge fabric.
//!
//! This crate provides a high-level client for interacting with Fabryk:
//! - Async client for Fabryk API
//! - Query, store, and retrieve knowledge items
//! - Partition and tag management
//! - Authentication handling
//! - Connection pooling and retry logic

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod client;
pub mod config;

pub use error::{Error, Result};
pub use client::FabrykClient;
