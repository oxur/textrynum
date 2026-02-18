//! Built-in MCP tools.
//!
//! Tools provided by the core `fabryk-mcp` crate, available to all
//! Fabryk-based MCP servers.

pub mod health;

pub use health::{handle_health, HealthResponse, HealthTools};
