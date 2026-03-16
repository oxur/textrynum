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
//! - `graph-rkyv-cache`: Enables binary caching with rkyv and Blake3 hashing

use crate::{Edge, GraphData, Node};
use fabryk_core::{Error, Result};
use petgraph::graph::DiGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// Serializable types
// ============================================================================

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
    /// When the graph was built (unix timestamp).
    #[serde(default)]
    pub built_at: String,
    /// Version of the builder.
    #[serde(default)]
    pub builder_version: String,
    /// Content hash for freshness checking.
    pub content_hash: Option<String>,
    /// Number of source files processed.
    pub source_file_count: Option<usize>,
}

impl Default for GraphMetadata {
    fn default() -> Self {
        Self {
            built_at: timestamp_now(),
            builder_version: env!("CARGO_PKG_VERSION").to_string(),
            content_hash: None,
            source_file_count: None,
        }
    }
}

/// Simple unix timestamp.
fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

// ============================================================================
// Save / Load
// ============================================================================

/// Save a graph to a JSON file.
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
        .map_err(|e| Error::operation(format!("Failed to serialize graph: {e}")))?;

    std::fs::write(path.as_ref(), json).map_err(|e| Error::io_with_path(e, path.as_ref()))?;

    Ok(())
}

/// Load a graph from a JSON file.
///
/// Rebuilds the petgraph `DiGraph` from the serialized nodes and edges.
pub fn load_graph(path: impl AsRef<Path>) -> Result<GraphData> {
    let json = std::fs::read_to_string(path.as_ref())
        .map_err(|e| Error::io_with_path(e, path.as_ref()))?;

    load_graph_from_str(&json)
}

/// Load a graph from a JSON string.
///
/// Useful for testing or loading from non-file sources.
pub fn load_graph_from_str(json: &str) -> Result<GraphData> {
    let serializable: SerializableGraph = serde_json::from_str(json)
        .map_err(|e| Error::parse(format!("Failed to parse graph JSON: {e}")))?;

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

    // Add edges (skip edges referencing missing nodes)
    let mut valid_edges = Vec::new();
    for edge in &serializable.edges {
        if let (Some(&from_idx), Some(&to_idx)) =
            (node_indices.get(&edge.from), node_indices.get(&edge.to))
        {
            graph.add_edge(from_idx, to_idx, edge.clone());
            valid_edges.push(edge.clone());
        }
    }

    Ok(GraphData {
        graph,
        node_indices,
        nodes,
        edges: valid_edges,
    })
}

/// Check if a cached graph is fresh compared to source content.
///
/// Returns `true` if the cache file exists and its content hash matches.
pub fn is_cache_fresh(cache_path: impl AsRef<Path>, content_hash: &str) -> bool {
    let path = cache_path.as_ref();
    if !path.exists() {
        return false;
    }

    if let Ok(json) = std::fs::read_to_string(path)
        && let Ok(serializable) = serde_json::from_str::<SerializableGraph>(&json)
        && let Some(metadata) = serializable.metadata
        && let Some(cached_hash) = metadata.content_hash
    {
        return cached_hash == content_hash;
    }

    false
}

// ============================================================================
// rkyv Cache Support (feature-gated)
// ============================================================================

#[cfg(feature = "graph-rkyv-cache")]
pub mod rkyv_cache {
    //! Binary caching with Blake3 content hashing.
    //!
    //! Enabled with the `graph-rkyv-cache` feature flag.

    use super::*;

    /// Compute a Blake3 hash of content files.
    pub fn compute_content_hash(paths: &[impl AsRef<Path>]) -> Result<String> {
        let mut hasher = blake3::Hasher::new();

        for path in paths {
            let content =
                std::fs::read(path.as_ref()).map_err(|e| Error::io_with_path(e, path.as_ref()))?;
            hasher.update(&content);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute a Blake3 hash of a directory's markdown files.
    pub fn compute_directory_hash(dir: impl AsRef<Path>) -> Result<String> {
        let mut hasher = blake3::Hasher::new();
        let mut paths: Vec<std::path::PathBuf> = Vec::new();

        fn collect_files(dir: &Path, paths: &mut Vec<std::path::PathBuf>) -> Result<()> {
            for entry in std::fs::read_dir(dir).map_err(|e| Error::io_with_path(e, dir))? {
                let entry = entry.map_err(Error::io)?;
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, paths)?;
                } else if path.extension().is_some_and(|e| e == "md") {
                    paths.push(path);
                }
            }
            Ok(())
        }

        collect_files(dir.as_ref(), &mut paths)?;
        paths.sort();

        for path in &paths {
            let content = std::fs::read(path).map_err(|e| Error::io_with_path(e, path))?;
            hasher.update(&content);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use tempfile::tempdir;

    fn create_test_graph() -> GraphData {
        let mut graph = GraphData::new();

        graph.add_node(Node::new("a", "Node A").with_category("cat1"));
        graph.add_node(Node::new("b", "Node B").with_category("cat2"));

        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();

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
    fn test_load_round_trip_preserves_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("round_trip.json");

        let mut original = GraphData::new();
        original.add_node(
            Node::new("a", "A")
                .with_category("cat")
                .with_source("src")
                .with_metadata("key", "value"),
        );
        original.add_node(Node::new("b", "B").as_variant_of("canonical-b"));

        original
            .add_edge(
                Edge::new("a", "b", Relationship::Custom("test-rel".to_string()))
                    .with_weight(0.42)
                    .with_origin(EdgeOrigin::Manual),
            )
            .unwrap();

        save_graph(&original, &path, None).unwrap();
        let loaded = load_graph(&path).unwrap();

        let node_a = loaded.get_node("a").unwrap();
        assert_eq!(node_a.category, Some("cat".to_string()));
        assert_eq!(node_a.source_id, Some("src".to_string()));

        let node_b = loaded.get_node("b").unwrap();
        assert!(!node_b.is_canonical);
        assert_eq!(node_b.canonical_id, Some("canonical-b".to_string()));

        assert_eq!(loaded.edges.len(), 1);
        assert_eq!(loaded.edges[0].weight, 0.42);
        assert_eq!(loaded.edges[0].origin, EdgeOrigin::Manual);
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

        assert!(is_cache_fresh(&path, "hash123"));
        assert!(!is_cache_fresh(&path, "different_hash"));
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
        assert_eq!(graph.graph.edge_count(), 0);
    }

    #[test]
    fn test_metadata_default() {
        let meta = GraphMetadata::default();
        assert!(!meta.built_at.is_empty());
        assert!(!meta.builder_version.is_empty());
        assert!(meta.content_hash.is_none());
        assert!(meta.source_file_count.is_none());
    }

    #[test]
    fn test_serializable_graph_round_trip() {
        let sg = SerializableGraph {
            nodes: vec![Node::new("test", "Test")],
            edges: vec![],
            metadata: Some(GraphMetadata::default()),
        };

        let json = serde_json::to_string(&sg).unwrap();
        let parsed: SerializableGraph = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.nodes.len(), 1);
        assert!(parsed.metadata.is_some());
    }
}

#[cfg(all(test, feature = "graph-rkyv-cache"))]
mod rkyv_tests {
    use super::rkyv_cache::*;
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
        assert_eq!(hash1, hash2);

        std::fs::write(&file2, "different").unwrap();
        let hash3 = compute_content_hash(&[&file1, &file2]).unwrap();
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
