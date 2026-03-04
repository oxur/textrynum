//! Built-in MCP tools.
//!
//! Tools provided by the core `fabryk-mcp` crate, available to all
//! Fabryk-based MCP servers.

pub mod diagnostics;
pub mod health;

pub use diagnostics::DiagnosticTools;
pub use health::{HealthResponse, HealthTools, handle_health};
