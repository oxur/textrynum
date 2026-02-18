//! Graph query MCP tools for Fabryk.
//!
//! This crate provides MCP tools that delegate to `fabryk-graph` algorithms.
//!
//! # Tools
//!
//! - `graph_related` — find related nodes
//! - `graph_path` — shortest path between nodes
//! - `graph_prerequisites` — learning order prerequisites
//! - `graph_neighborhood` — N-hop neighborhood exploration
//! - `graph_info` — graph statistics
//! - `graph_validate` — structure validation
//! - `graph_centrality` — most important nodes
//! - `graph_bridges` — gateway nodes
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_graph::load_graph;
//! use fabryk_mcp_graph::GraphTools;
//!
//! let graph = load_graph("graph.json")?;
//! let graph_tools = GraphTools::new(graph);
//!
//! // Register with composite registry
//! let registry = CompositeRegistry::new().add(graph_tools);
//! ```

pub mod tools;

// Re-exports
pub use tools::{GraphTools, NeighborhoodArgs, PathArgs, PrerequisitesArgs, RelatedArgs};
