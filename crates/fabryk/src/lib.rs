//! Fabryk knowledge fabric — umbrella crate.
//!
//! This crate re-exports all Fabryk core components for convenience.
//! Use feature flags to enable backend-specific functionality.

#![doc = include_str!("../README.md")]

pub use fabryk_acl as acl;
pub use fabryk_auth as auth;
pub use fabryk_content as content;
pub use fabryk_core as core;
pub use fabryk_fts as fts;
pub use fabryk_graph as graph;
pub use fabryk_vector as vector;
