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
//! - `graph_get_node` — detailed information about a single node
//! - `graph_get_node_edges` — edges connected to a node
//! - `graph_dependents` — nodes that depend on a given node
//! - `graph_status` — whether graph is loaded with basic stats
//! - `graph_concept_sources` — find sources that introduce or cover a concept
//! - `graph_concept_variants` — find source-specific variants of a canonical concept
//! - `graph_source_coverage` — find concepts that a source introduces or covers
//! - `graph_learning_path` — step-numbered learning path to a target concept
//! - `graph_bridge_categories` — find nodes connecting two specific categories
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
pub use tools::{
    BridgeCategoriesArgs, DependentsArgs, GetNodeArgs, GetNodeEdgesArgs, GraphNodeFilter,
    GraphTools, LearningPathArgs, NeighborhoodArgs, PathArgs, PrerequisitesArgs, RelatedArgs,
};
