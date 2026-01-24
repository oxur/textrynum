//! # fabryk-query
//!
//! Query engine for Fabryk knowledge fabric.
//!
//! This crate implements the query engine:
//! - Keyword search
//! - Semantic search (pgvector integration)
//! - Hybrid search (keyword + semantic)
//! - Tag-based filtering
//! - Partition-scoped queries
//! - Query parsing and optimization

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod engine;
pub mod semantic;
pub mod keyword;
pub mod hybrid;

pub use error::{Error, Result};
