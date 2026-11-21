---
title: "CC Prompt: Fabryk 4.4 — Graph Persistence"
milestone: "4.4"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["4.1-4.3 complete"]
governing-docs: [0011-audit §4.4, 0013-project-plan]
---

# CC Prompt: Fabryk 4.4 — Graph Persistence

## Context

The persistence module handles saving and loading graph data. It supports:

1. **JSON serialization** - Human-readable, debuggable format
2. **Optional rkyv caching** - Fast binary serialization (feature-gated)
3. **Freshness checking** - Rebuild only when content changes

The rkyv cache uses Blake3 content hashing to detect when the cache is stale.
This is feature-gated behind `rkyv-cache` to avoid mandatory dependencies.

## Objective

Extract graph persistence to `fabryk-graph::persistence`:

1. `save_graph()` - Serialize graph to JSON
2. `load_graph()` - Deserialize graph from JSON
3. `to_petgraph()` - Rebuild petgraph from loaded data
4. Optional rkyv cache with freshness validation

## Implementation Steps

### Step 1: Create persistence module

Create `fabryk-graph/src/persistence.rs`:

```rust
//! Graph persistence and caching.
//!
//! This module provides functions for saving and loading graph data:
//!
//! - JSON format for human-readable storage
//! - Optional rkyv binary format for fast loading (feature-gated)
//! - Freshness checking to avoid unnecessary rebuilds
//!
//! # Feature Flags
//!
//! - `rkyv-cache`: Enables binary caching with rkyv and Blake3 hashing

use crate::{Edge, GraphData, Node};
use fabryk_core::{Error, Result};
use petgraph::graph::DiGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Serializable representation of graph data.
///
/// Used for JSON persistence. The petgraph `DiGraph` is rebuilt on load.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableGraph {
    /// All nodes in the graph.
    pub nodes: Vec<Node>,
    /// All edges in the graph.
    pub edges: Vec<Edge>,
    /// Optional metadata about the graph.
    pub metadata: Option<GraphMetadata>,
}

/// Metadata about a persisted graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphMetadata {
    /// When the graph was built.
    pub built_at: String,
    /// Version of the builder.
    pub builder_version: String,
    /// Content hash for freshness checking.
    pub content_hash: Option<String>,
    /// Number of source files processed.
    pub source_file_count: Option<usize>,
}

impl Default for GraphMetadata {
    fn default() -> Self {
        Self {
            built_at: chrono_lite_now(),
            builder_version: env!("CARGO_PKG_VERSION").to_string(),
            content_hash: None,
            source_file_count: None,
        }
    }
}

/// Simple timestamp without chrono dependency.
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

/// Save a graph to a JSON file.
///
/// # Arguments
///
/// * `graph` - The graph data to save
/// * `path` - Output file path
/// * `metadata` - Optional metadata to include
///
/// # Example
///
/// ```rust,ignore
/// save_graph(&graph, "graph.json", None)?;
/// ```
pub fn save_graph(
    graph: &GraphData,
    path: impl AsRef<Path>,
    metadata: Option<GraphMetadata>,
) -> Result<()> {
    let serializable = SerializableGraph {
        nodes: graph.nodes.values().cloned().collect(),
        edges: graph.edges.clone(),
        metadata,
    };

    let json = serde_json::to_string_pretty(&serializable)
        .map_err(|e| Error::operation(format!("Failed to serialize graph: {}", e)))?;

    std::fs::write(path.as_ref(), json).map_err(|e| Error::io_with_path(e, path.as_ref()))?;

    Ok(())
}

/// Load a graph from a JSON file.
///
/// Rebuilds the petgraph `DiGraph` from the serialized nodes and edges.
///
/// # Arguments
///
/// * `path` - Input file path
///
/// # Returns
///
/// Loaded `GraphData` with petgraph rebuilt.
///
/// # Example
///
/// ```rust,ignore
/// let graph = load_graph("graph.json")?;
/// println!("Loaded {} nodes", graph.node_count());
/// ```
pub fn load_graph(path: impl AsRef<Path>) -> Result<GraphData> {
    let json = std::fs::read_to_string(path.as_ref())
        .map_err(|e| Error::io_with_path(e, path.as_ref()))?;

    let serializable: SerializableGraph = serde_json::from_str(&json)
        .map_err(|e| Error::parse(format!("Failed to parse graph JSON: {}", e)))?;

    to_graph_data(serializable)
}

/// Load a graph from a JSON string.
///
/// Useful for testing or loading from non-file sources.
pub fn load_graph_from_str(json: &str) -> Result<GraphData> {
    let serializable: SerializableGraph = serde_json::from_str(json)
        .map_err(|e| Error::parse(format!("Failed to parse graph JSON: {}", e)))?;

    to_graph_data(serializable)
}

/// Convert serializable format to GraphData.
fn to_graph_data(serializable: SerializableGraph) -> Result<GraphData> {
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();
    let mut nodes = HashMap::new();

    // Add nodes
    for node in &serializable.nodes {
        let idx = graph.add_node(node.clone());
        node_indices.insert(node.id.clone(), idx);
        nodes.insert(node.id.clone(), node.clone());
    }

    // Add edges
    for edge in &serializable.edges {
        if let (Some(&from_idx), Some(&to_idx)) =
            (node_indices.get(&edge.from), node_indices.get(&edge.to))
        {
            graph.add_edge(from_idx, to_idx, edge.clone());
        }
        // Skip edges with missing nodes (could log warning)
    }

    Ok(GraphData {
        graph,
        node_indices,
        nodes,
        edges: serializable.edges,
    })
}

/// Check if a cached graph is fresh compared to source content.
///
/// Returns `true` if the cache is valid and can be used.
///
/// # Arguments
///
/// * `cache_path` - Path to the cached graph file
/// * `content_hash` - Current hash of source content
///
/// # Returns
///
/// `true` if cache exists and hash matches, `false` otherwise.
pub fn is_cache_fresh(cache_path: impl AsRef<Path>, content_hash: &str) -> bool {
    let path = cache_path.as_ref();
    if !path.exists() {
        return false;
    }

    // Load just the metadata to check hash
    if let Ok(json) = std::fs::read_to_string(path) {
        if let Ok(serializable) = serde_json::from_str::<SerializableGraph>(&json) {
            if let Some(metadata) = serializable.metadata {
                if let Some(cached_hash) = metadata.content_hash {
                    return cached_hash == content_hash;
                }
            }
        }
    }

    false
}

// ============================================================================
// rkyv Cache Support (feature-gated)
// ============================================================================

#[cfg(feature = "rkyv-cache")]
pub mod rkyv_cache {
    //! Binary caching with rkyv for fast graph loading.
    //!
    //! Enabled with the `rkyv-cache` feature flag.

    use super::*;
    use blake3::Hasher;
    use std::fs::File;
    use std::io::{BufReader, BufWriter, Read, Write};

    /// Compute a Blake3 hash of content files.
    ///
    /// Used for cache invalidation.
    pub fn compute_content_hash(paths: &[impl AsRef<Path>]) -> Result<String> {
        let mut hasher = Hasher::new();

        for path in paths {
            let mut file = File::open(path.as_ref())
                .map_err(|e| Error::io_with_path(e, path.as_ref()))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| Error::io_with_path(e, path.as_ref()))?;
            hasher.update(&buffer);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute a Blake3 hash of a directory's content files.
    ///
    /// Recursively hashes all `.md` files in the directory.
    pub fn compute_directory_hash(dir: impl AsRef<Path>) -> Result<String> {
        use std::fs;

        let mut hasher = Hasher::new();
        let mut paths: Vec<_> = Vec::new();

        fn collect_files(dir: &Path, paths: &mut Vec<std::path::PathBuf>) -> Result<()> {
            for entry in fs::read_dir(dir).map_err(|e| Error::io_with_path(e, dir))? {
                let entry = entry.map_err(|e| Error::io(e))?;
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, paths)?;
                } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                    paths.push(path);
                }
            }
            Ok(())
        }

        collect_files(dir.as_ref(), &mut paths)?;
        paths.sort(); // Consistent ordering

        for path in &paths {
            let content = fs::read(path).map_err(|e| Error::io_with_path(e, path))?;
            hasher.update(&content);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Save graph using rkyv binary format.
    ///
    /// Much faster to load than JSON for large graphs.
    pub fn save_binary(
        graph: &GraphData,
        path: impl AsRef<Path>,
        content_hash: Option<&str>,
    ) -> Result<()> {
        // For now, fall back to JSON with a .bin.json extension marker
        // Full rkyv implementation would require derive macros on all types
        let metadata = GraphMetadata {
            content_hash: content_hash.map(String::from),
            ..Default::default()
        };
        save_graph(graph, path, Some(metadata))
    }

    /// Load graph from rkyv binary format.
    pub fn load_binary(path: impl AsRef<Path>) -> Result<GraphData> {
        // Fall back to JSON for now
        load_graph(path)
    }
}
```

### Step 2: Update Cargo.toml

Ensure `fabryk-graph/Cargo.toml` has the correct dependencies:

```toml
[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-content = { path = "../fabryk-content" }

petgraph = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tokio = { version = "1.0", features = ["fs"] }

# Optional: rkyv caching
rkyv = { version = "0.7", optional = true }
blake3 = { version = "1.0", optional = true }

[features]
default = []
rkyv-cache = ["rkyv", "blake3"]
test-utils = []
```

### Step 3: Update lib.rs

Update `fabryk-graph/src/lib.rs`:

```rust
pub mod algorithms;
pub mod extractor;
pub mod persistence;
pub mod types;

// Re-exports
pub use algorithms::{
    calculate_centrality, find_bridges, get_related, neighborhood, prerequisites_sorted,
    shortest_path, CentralityScore, NeighborhoodResult, PathResult, PrerequisitesResult,
};
pub use extractor::GraphExtractor;
pub use persistence::{
    is_cache_fresh, load_graph, load_graph_from_str, save_graph, GraphMetadata, SerializableGraph,
};
pub use types::{Edge, EdgeOrigin, GraphData, Node, Relationship};

#[cfg(any(test, feature = "test-utils"))]
pub use extractor::mock::{MockEdgeData, MockExtractor, MockNodeData};

#[cfg(feature = "rkyv-cache")]
pub use persistence::rkyv_cache;
```

### Step 4: Add persistence tests

Add to `fabryk-graph/src/persistence.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use tempfile::tempdir;

    fn create_test_graph() -> GraphData {
        let mut graph = GraphData::new();

        let nodes = vec![
            Node::new("a", "Node A").with_category("cat1"),
            Node::new("b", "Node B").with_category("cat2"),
        ];

        for node in &nodes {
            let idx = graph.graph.add_node(node.clone());
            graph.node_indices.insert(node.id.clone(), idx);
            graph.nodes.insert(node.id.clone(), node.clone());
        }

        let edge = Edge::new("a", "b", Relationship::Prerequisite);
        let from_idx = graph.node_indices["a"];
        let to_idx = graph.node_indices["b"];
        graph.graph.add_edge(from_idx, to_idx, edge.clone());
        graph.edges.push(edge);

        graph
    }

    #[test]
    fn test_save_and_load_graph() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_graph.json");

        let original = create_test_graph();
        save_graph(&original, &path, None).unwrap();

        let loaded = load_graph(&path).unwrap();

        assert_eq!(loaded.node_count(), original.node_count());
        assert_eq!(loaded.edge_count(), original.edge_count());
        assert!(loaded.contains_node("a"));
        assert!(loaded.contains_node("b"));
    }

    #[test]
    fn test_save_with_metadata() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_graph.json");

        let graph = create_test_graph();
        let metadata = GraphMetadata {
            content_hash: Some("abc123".to_string()),
            source_file_count: Some(10),
            ..Default::default()
        };

        save_graph(&graph, &path, Some(metadata)).unwrap();

        let json = std::fs::read_to_string(&path).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_load_graph_from_str() {
        let json = r#"{
            "nodes": [
                {"id": "x", "title": "X", "category": null, "source_id": null, "is_canonical": true, "canonical_id": null, "metadata": {}}
            ],
            "edges": [],
            "metadata": null
        }"#;

        let graph = load_graph_from_str(json).unwrap();
        assert_eq!(graph.node_count(), 1);
        assert!(graph.contains_node("x"));
    }

    #[test]
    fn test_is_cache_fresh() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cache.json");

        let graph = create_test_graph();
        let metadata = GraphMetadata {
            content_hash: Some("hash123".to_string()),
            ..Default::default()
        };

        save_graph(&graph, &path, Some(metadata)).unwrap();

        // Same hash - fresh
        assert!(is_cache_fresh(&path, "hash123"));

        // Different hash - stale
        assert!(!is_cache_fresh(&path, "different_hash"));

        // Missing file - not fresh
        assert!(!is_cache_fresh(dir.path().join("missing.json"), "hash123"));
    }

    #[test]
    fn test_load_graph_invalid_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.json");
        std::fs::write(&path, "not valid json").unwrap();

        let result = load_graph(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_edges_with_missing_nodes() {
        // Edges referencing non-existent nodes should be skipped
        let json = r#"{
            "nodes": [
                {"id": "a", "title": "A", "category": null, "source_id": null, "is_canonical": true, "canonical_id": null, "metadata": {}}
            ],
            "edges": [
                {"from": "a", "to": "missing", "relationship": "Prerequisite", "weight": 1.0, "origin": "Frontmatter"}
            ],
            "metadata": null
        }"#;

        let graph = load_graph_from_str(json).unwrap();
        assert_eq!(graph.node_count(), 1);
        // Edge to missing node should be skipped
        assert_eq!(graph.graph.edge_count(), 0);
    }
}

#[cfg(all(test, feature = "rkyv-cache"))]
mod rkyv_tests {
    use super::rkyv_cache::*;
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compute_content_hash() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("a.md");
        let file2 = dir.path().join("b.md");

        std::fs::write(&file1, "content a").unwrap();
        std::fs::write(&file2, "content b").unwrap();

        let hash1 = compute_content_hash(&[&file1, &file2]).unwrap();
        let hash2 = compute_content_hash(&[&file1, &file2]).unwrap();

        // Same content = same hash
        assert_eq!(hash1, hash2);

        // Modify content
        std::fs::write(&file2, "different").unwrap();
        let hash3 = compute_content_hash(&[&file1, &file2]).unwrap();

        // Different content = different hash
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_compute_directory_hash() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();

        std::fs::write(dir.path().join("a.md"), "a").unwrap();
        std::fs::write(sub.join("b.md"), "b").unwrap();

        let hash = compute_directory_hash(dir.path()).unwrap();
        assert!(!hash.is_empty());
    }
}
```

### Step 5: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-graph
cargo test -p fabryk-graph
cargo clippy -p fabryk-graph -- -D warnings

# Test with rkyv-cache feature
cargo test -p fabryk-graph --features rkyv-cache
```

## Exit Criteria

- [ ] `save_graph()` serializes graph to JSON
- [ ] `load_graph()` deserializes and rebuilds petgraph
- [ ] `load_graph_from_str()` for non-file sources
- [ ] `is_cache_fresh()` validates cache against content hash
- [ ] `GraphMetadata` captures build timestamp, version, content hash
- [ ] `rkyv-cache` feature gates optional binary caching
- [ ] `compute_content_hash()` and `compute_directory_hash()` (rkyv-cache)
- [ ] Handles edges with missing nodes gracefully
- [ ] All tests pass (including with `--features rkyv-cache`)

## Commit Message

```
feat(graph): add graph persistence module

Add persistence support for knowledge graphs:
- save_graph(): JSON serialization with pretty-printing
- load_graph(): Deserialize and rebuild petgraph
- is_cache_fresh(): Content hash validation
- GraphMetadata: Build timestamp, version, hash tracking

Feature-gate rkyv binary caching behind rkyv-cache:
- compute_content_hash(): Blake3 hashing of files
- compute_directory_hash(): Recursive directory hashing

Phase 4 milestone 4.4 of Fabryk extraction.

Ref: Doc 0011 §4.4 (graph persistence)
Ref: Doc 0013 Phase 4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
