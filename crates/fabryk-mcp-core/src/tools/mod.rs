//! Built-in MCP tools.
//!
//! Tools provided by the core `fabryk-mcp` crate, available to all
//! Fabryk-based MCP servers.

pub mod diagnostics;
pub mod health;

pub use diagnostics::DiagnosticTools;
pub use health::{
    BackendInfo, BackendsInfo, FieldBoosts, HealthResponse, HealthTools, SearchConfigInfo,
    handle_health, handle_health_enriched,
};
