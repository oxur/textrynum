//! # Fabryk
//!
//! Knowledge fabric for AI workflows.
//!
//! Fabryk is a knowledge management system designed for AI workflows, providing:
//! - Partitioned knowledge storage with access control
//! - Semantic and keyword search
//! - Tag-based organization
//! - HTTP API and client libraries
//! - MCP (Model Context Protocol) integration
//!
//! ## Architecture
//!
//! Fabryk is composed of several crates:
//!
//! - **fabryk-core**: Core types, traits, and errors
//! - **fabryk-acl**: Access control implementation
//! - **fabryk-storage**: Storage backend implementations
//! - **fabryk-query**: Query engine (keyword, semantic, hybrid)
//! - **fabryk-api**: HTTP API server
//! - **fabryk-client**: Rust client library
//! - **fabryk-mcp**: Model Context Protocol server
//! - **fabryk-cli**: Command-line administration tool
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use fabryk::client::FabrykClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a client
//! let client = FabrykClient::new()?;
//!
//! // TODO: Add usage examples when implementation is complete
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! This crate re-exports the main Fabryk components. For CLI and MCP server binaries,
//! see the `fabryk-cli` and `fabryk-mcp` crates respectively.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

// Re-export core types and traits
pub use fabryk_core as core;

// Re-export ACL functionality
pub use fabryk_acl as acl;

// Re-export storage backends
pub use fabryk_storage as storage;

// Re-export query engine
pub use fabryk_query as query;

// Re-export API server
pub use fabryk_api as api;

// Re-export client
pub use fabryk_client as client;

// Convenience re-exports of commonly used types
pub mod prelude {
    //! Prelude module with commonly used types and traits

    pub use crate::core::{Error, Result};
    pub use crate::client::FabrykClient;
}
