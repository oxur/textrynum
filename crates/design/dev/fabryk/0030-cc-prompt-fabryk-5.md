---
title: "CC Prompt: Fabryk 5.5 — Graph MCP Tools"
milestone: "5.5"
phase: 5
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["5.1-5.4 complete", "Phase 4 fabryk-graph complete"]
governing-docs: [0011-audit §4.9, 0013-project-plan]
---

# CC Prompt: Fabryk 5.5 — Graph MCP Tools

## Context

This milestone extracts the graph query MCP tools to `fabryk-mcp-graph`. These
tools use the `fabryk-graph` infrastructure created in Phase 4.

The graph tools provide:
- Related concepts queries
- Path finding between concepts
- Prerequisites analysis
- Neighborhood exploration
- Graph statistics and validation

## Objective

Create `fabryk-mcp-graph` crate with:

1. Tools that delegate to `fabryk-graph` algorithms
2. Generic response types using fabryk-graph query types
3. Tools parameterized over generic node/edge types

## Implementation Steps

### Step 1: Create fabryk-mcp-graph crate

```bash
cd ~/lab/oxur/ecl/crates
mkdir -p fabryk-mcp-graph/src
```

Create `fabryk-mcp-graph/Cargo.toml`:

```toml
[package]
name = "fabryk-mcp-graph"
version = "0.1.0"
edition = "2021"
description = "Graph MCP tools for Fabryk domains"
license = "Apache-2.0"

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-graph = { path = "../fabryk-graph" }
fabryk-mcp = { path = "../fabryk-mcp" }

async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
mcp-core = "0.2"
tokio = { version = "1.0", features = ["sync"] }

[dev-dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
```

### Step 2: Create graph tools

Create `fabryk-mcp-graph/src/tools.rs`:

```rust
//! MCP tools for graph queries.

use fabryk_core::Result;
use fabryk_graph::{
    calculate_centrality, find_bridges, neighborhood, prerequisites_sorted, shortest_path,
    validate_graph, GraphData, NodeSummary, Relationship,
};
use fabryk_mcp::{ToolRegistry, ToolResult};
use mcp_core::ToolInfo;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Arguments for graph_related tool.
#[derive(Debug, Deserialize)]
pub struct RelatedArgs {
    pub id: String,
    pub relationship: Option<String>,
    pub limit: Option<usize>,
}

/// Arguments for graph_path tool.
#[derive(Debug, Deserialize)]
pub struct PathArgs {
    pub from: String,
    pub to: String,
}

/// Arguments for graph_prerequisites tool.
#[derive(Debug, Deserialize)]
pub struct PrerequisitesArgs {
    pub id: String,
}

/// Arguments for graph_neighborhood tool.
#[derive(Debug, Deserialize)]
pub struct NeighborhoodArgs {
    pub id: String,
    pub radius: Option<usize>,
    pub relationship: Option<String>,
}

/// MCP tools for graph queries.
///
/// Holds a shared reference to the graph data.
pub struct GraphTools {
    graph: Arc<RwLock<GraphData>>,
}

impl GraphTools {
    /// Create new graph tools.
    pub fn new(graph: GraphData) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
        }
    }

    /// Create graph tools with shared graph data.
    pub fn with_shared(graph: Arc<RwLock<GraphData>>) -> Self {
        Self { graph }
    }

    /// Update the graph data.
    pub async fn update_graph(&self, graph: GraphData) {
        let mut lock = self.graph.write().await;
        *lock = graph;
    }

    fn parse_relationship(s: Option<&str>) -> Option<Vec<Relationship>> {
        s.map(|rel| {
            vec![match rel.to_lowercase().as_str() {
                "prerequisite" => Relationship::Prerequisite,
                "leads_to" => Relationship::LeadsTo,
                "relates_to" => Relationship::RelatesTo,
                "extends" => Relationship::Extends,
                other => Relationship::Custom(other.to_string()),
            }]
        })
    }
}

impl ToolRegistry for GraphTools {
    fn tools(&self) -> Vec<ToolInfo> {
        vec![
            ToolInfo {
                name: "graph_related".to_string(),
                description: "Find concepts related to a given concept".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Concept ID"
                        },
                        "relationship": {
                            "type": "string",
                            "description": "Filter by relationship type"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results"
                        }
                    },
                    "required": ["id"]
                }),
            },
            ToolInfo {
                name: "graph_path".to_string(),
                description: "Find the shortest path between two concepts".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "from": {
                            "type": "string",
                            "description": "Starting concept ID"
                        },
                        "to": {
                            "type": "string",
                            "description": "Target concept ID"
                        }
                    },
                    "required": ["from", "to"]
                }),
            },
            ToolInfo {
                name: "graph_prerequisites".to_string(),
                description: "Get prerequisites for a concept in learning order".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Concept ID"
                        }
                    },
                    "required": ["id"]
                }),
            },
            ToolInfo {
                name: "graph_neighborhood".to_string(),
                description: "Explore the neighborhood around a concept".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Center concept ID"
                        },
                        "radius": {
                            "type": "integer",
                            "description": "Hops from center (default 1)"
                        },
                        "relationship": {
                            "type": "string",
                            "description": "Filter by relationship type"
                        }
                    },
                    "required": ["id"]
                }),
            },
            ToolInfo {
                name: "graph_info".to_string(),
                description: "Get graph statistics and overview".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolInfo {
                name: "graph_validate".to_string(),
                description: "Validate graph structure and report issues".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolInfo {
                name: "graph_centrality".to_string(),
                description: "Get most central/important concepts".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of results (default 10)"
                        }
                    }
                }),
            },
            ToolInfo {
                name: "graph_bridges".to_string(),
                description: "Find bridge concepts that connect different areas".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of results (default 10)"
                        }
                    }
                }),
            },
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let graph = Arc::clone(&self.graph);

        match name {
            "graph_related" => Some(Box::pin(async move {
                let args: RelatedArgs = serde_json::from_value(args)?;
                let graph = graph.read().await;

                let result = neighborhood(
                    &graph,
                    &args.id,
                    1,
                    Self::parse_relationship(args.relationship.as_deref()).as_deref(),
                )?;

                let mut nodes: Vec<NodeSummary> = result
                    .nodes
                    .iter()
                    .map(NodeSummary::from)
                    .collect();

                if let Some(limit) = args.limit {
                    nodes.truncate(limit);
                }

                Ok(json!({
                    "source": NodeSummary::from(&result.center),
                    "related": nodes,
                    "count": nodes.len()
                }))
            })),

            "graph_path" => Some(Box::pin(async move {
                let args: PathArgs = serde_json::from_value(args)?;
                let graph = graph.read().await;

                let result = shortest_path(&graph, &args.from, &args.to)?;

                if result.found {
                    let path: Vec<NodeSummary> = result
                        .path
                        .iter()
                        .map(NodeSummary::from)
                        .collect();

                    Ok(json!({
                        "found": true,
                        "path": path,
                        "length": path.len(),
                        "total_weight": result.total_weight
                    }))
                } else {
                    Ok(json!({
                        "found": false,
                        "message": format!("No path found from {} to {}", args.from, args.to)
                    }))
                }
            })),

            "graph_prerequisites" => Some(Box::pin(async move {
                let args: PrerequisitesArgs = serde_json::from_value(args)?;
                let graph = graph.read().await;

                let result = prerequisites_sorted(&graph, &args.id)?;

                let prereqs: Vec<NodeSummary> = result
                    .ordered
                    .iter()
                    .map(NodeSummary::from)
                    .collect();

                Ok(json!({
                    "target": NodeSummary::from(&result.target),
                    "prerequisites": prereqs,
                    "count": prereqs.len(),
                    "has_cycles": result.has_cycles
                }))
            })),

            "graph_neighborhood" => Some(Box::pin(async move {
                let args: NeighborhoodArgs = serde_json::from_value(args)?;
                let graph = graph.read().await;

                let radius = args.radius.unwrap_or(1);
                let result = neighborhood(
                    &graph,
                    &args.id,
                    radius,
                    Self::parse_relationship(args.relationship.as_deref()).as_deref(),
                )?;

                let nodes: Vec<_> = result
                    .nodes
                    .iter()
                    .map(|n| {
                        json!({
                            "node": NodeSummary::from(n),
                            "distance": result.distances.get(&n.id)
                        })
                    })
                    .collect();

                Ok(json!({
                    "center": NodeSummary::from(&result.center),
                    "radius": radius,
                    "nodes": nodes,
                    "edge_count": result.edges.len()
                }))
            })),

            "graph_info" => Some(Box::pin(async move {
                let graph = graph.read().await;
                let stats = fabryk_graph::compute_stats(&graph);

                Ok(json!({
                    "node_count": stats.node_count,
                    "edge_count": stats.edge_count,
                    "categories": stats.category_distribution,
                    "relationships": stats.relationship_distribution,
                    "orphan_count": stats.orphan_count,
                    "avg_degree": stats.avg_degree
                }))
            })),

            "graph_validate" => Some(Box::pin(async move {
                let graph = graph.read().await;
                let result = validate_graph(&graph);

                Ok(json!({
                    "valid": result.valid,
                    "error_count": result.errors.len(),
                    "warning_count": result.warnings.len(),
                    "errors": result.errors,
                    "warnings": result.warnings
                }))
            })),

            "graph_centrality" => Some(Box::pin(async move {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(10);

                let graph = graph.read().await;
                let scores = calculate_centrality(&graph);

                let top: Vec<_> = scores
                    .into_iter()
                    .take(limit)
                    .map(|s| {
                        json!({
                            "id": s.node_id,
                            "degree": s.degree,
                            "in_degree": s.in_degree,
                            "out_degree": s.out_degree
                        })
                    })
                    .collect();

                Ok(json!({ "top_central": top }))
            })),

            "graph_bridges" => Some(Box::pin(async move {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(10);

                let graph = graph.read().await;
                let bridges = find_bridges(&graph, limit);

                let summaries: Vec<NodeSummary> = bridges
                    .iter()
                    .map(NodeSummary::from)
                    .collect();

                Ok(json!({ "bridges": summaries }))
            })),

            _ => None,
        }
    }
}
```

### Step 3: Create lib.rs

Create `fabryk-mcp-graph/src/lib.rs`:

```rust
//! Graph MCP tools for Fabryk domains.
//!
//! This crate provides MCP tools that delegate to `fabryk-graph` algorithms.
//!
//! # Tools
//!
//! - `graph_related` - Find related concepts
//! - `graph_path` - Shortest path between concepts
//! - `graph_prerequisites` - Learning order prerequisites
//! - `graph_neighborhood` - N-hop neighborhood exploration
//! - `graph_info` - Graph statistics
//! - `graph_validate` - Structure validation
//! - `graph_centrality` - Most important concepts
//! - `graph_bridges` - Gateway concepts
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

pub use tools::{GraphTools, NeighborhoodArgs, PathArgs, PrerequisitesArgs, RelatedArgs};
```

### Step 4: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-mcp-graph
cargo test -p fabryk-mcp-graph
cargo clippy -p fabryk-mcp-graph -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-mcp-graph` crate created
- [ ] `GraphTools` implements ToolRegistry with 8 tools
- [ ] `graph_related` with optional relationship filter
- [ ] `graph_path` for shortest path queries
- [ ] `graph_prerequisites` for learning order
- [ ] `graph_neighborhood` for N-hop exploration
- [ ] `graph_info`, `graph_validate`, `graph_centrality`, `graph_bridges`
- [ ] All tools return proper JSON responses
- [ ] All tests pass

## Commit Message

```
feat(mcp): add fabryk-mcp-graph with graph query tools

Add MCP tools for graph queries:
- GraphTools implements ToolRegistry with 8 tools
- graph_related: Find related concepts with filtering
- graph_path: Shortest path between concepts
- graph_prerequisites: Learning order analysis
- graph_neighborhood: N-hop exploration
- graph_info/validate/centrality/bridges

Tools hold shared reference to GraphData via RwLock.

Phase 5 milestone 5.5 of Fabryk extraction.

Ref: Doc 0011 §4.9 (graph MCP tools)
Ref: Doc 0013 Phase 5

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
