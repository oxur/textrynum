//! Fabryk MCP — umbrella crate.
//!
//! This crate re-exports all Fabryk MCP components for convenience.
//! Use feature flags to enable backend-specific functionality.

// Re-export everything from core for backward compatibility.
// All infrastructure symbols (ToolRegistry, FabrykMcpServer, etc.)
// remain available at `fabryk_mcp::`.
pub use fabryk_mcp_core::*;

pub use fabryk_mcp_auth as auth;
pub use fabryk_mcp_content as content;
pub use fabryk_mcp_fts as fts;
pub use fabryk_mcp_graph as graph;
pub use fabryk_mcp_semantic as semantic;
