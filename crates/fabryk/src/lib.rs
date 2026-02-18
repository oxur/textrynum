//! Fabryk knowledge fabric — umbrella crate.
//!
//! This crate re-exports all Fabryk components for convenience.
//! Use feature flags to enable specific functionality.

#![doc = include_str!("../README.md")]

pub use fabryk_content as content;
pub use fabryk_core as core;

#[cfg(feature = "fts")]
pub use fabryk_fts as fts;

#[cfg(feature = "graph")]
pub use fabryk_graph as graph;

#[cfg(feature = "vector")]
pub use fabryk_vector as vector;

#[cfg(feature = "mcp")]
pub use fabryk_mcp as mcp;

#[cfg(feature = "mcp")]
pub use fabryk_mcp_content as mcp_content;

#[cfg(feature = "mcp")]
pub use fabryk_mcp_fts as mcp_fts;

#[cfg(feature = "mcp")]
pub use fabryk_mcp_graph as mcp_graph;

#[cfg(feature = "cli")]
pub use fabryk_cli as cli;
