---
title: "CC Prompt: Fabryk 4.5 — Graph Builder"
milestone: "4.5"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["4.1-4.4 complete"]
governing-docs: [0011-audit §4.4, 0012-amendment §2f-iii, 0013-project-plan]
---

# CC Prompt: Fabryk 4.5 — Graph Builder

## Context

The `GraphBuilder` is the central orchestration point that:

1. Discovers content files using glob patterns
2. Calls `GraphExtractor` methods to extract nodes and edges
3. Builds the `GraphData` structure
4. Supports manual edges from JSON (Amendment §2f-iii)

This is a critical milestone as it wires together the extractor trait with
the graph data structure. The builder is generic over `E: GraphExtractor`.

## Objective

Extract `GraphBuilder<E: GraphExtractor>` to `fabryk-graph::builder`:

1. File discovery using content glob patterns
2. Node extraction via `GraphExtractor::extract_node()`
3. Edge extraction via `GraphExtractor::extract_edges()`
4. Manual edges support via `with_manual_edges()` (Amendment §2f-iii)
5. Progress reporting and error handling options

## Implementation Steps

### Step 1: Create builder module

Create `fabryk-graph/src/builder.rs`:

```rust
//! GraphBuilder for constructing knowledge graphs.
//!
//! The builder orchestrates content discovery and graph construction:
//!
//! 1. Discover content files using glob patterns
//! 2. Parse frontmatter and content
//! 3. Call GraphExtractor methods to extract nodes/edges
//! 4. Build the final GraphData structure
//!
//! # Example
//!
//! ```rust,ignore
//! let extractor = MyDomainExtractor::new();
//! let graph = GraphBuilder::new(extractor)
//!     .with_content_path("/data/concepts")
//!     .with_manual_edges("/data/graphs/manual_edges.json")
//!     .build()
//!     .await?;
//! ```

use crate::{Edge, EdgeOrigin, GraphData, GraphExtractor, Node};
use fabryk_content::markdown::extract_frontmatter;
use fabryk_core::{Error, Result};
use petgraph::graph::DiGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Options for handling errors during graph building.
#[derive(Clone, Debug, Default)]
pub enum ErrorHandling {
    /// Stop on first error.
    #[default]
    FailFast,
    /// Continue and collect errors.
    Collect,
    /// Log and skip problematic files.
    Skip,
}

/// Progress callback for build operations.
pub type ProgressCallback = Box<dyn Fn(BuildProgress) + Send + Sync>;

/// Progress information during graph building.
#[derive(Clone, Debug)]
pub struct BuildProgress {
    /// Current file being processed.
    pub current_file: Option<PathBuf>,
    /// Number of files processed.
    pub files_processed: usize,
    /// Total files to process.
    pub total_files: usize,
    /// Number of nodes created.
    pub nodes_created: usize,
    /// Number of edges created.
    pub edges_created: usize,
    /// Errors encountered (if ErrorHandling::Collect).
    pub errors: Vec<String>,
}

impl BuildProgress {
    fn new(total_files: usize) -> Self {
        Self {
            current_file: None,
            files_processed: 0,
            total_files,
            nodes_created: 0,
            edges_created: 0,
            errors: Vec::new(),
        }
    }
}

/// Result of a graph build operation.
#[derive(Debug)]
pub struct BuildResult {
    /// The constructed graph.
    pub graph: GraphData,
    /// Files that were processed.
    pub files_processed: usize,
    /// Files that were skipped due to errors.
    pub files_skipped: usize,
    /// Errors encountered (if not fail-fast).
    pub errors: Vec<BuildError>,
    /// Manual edges loaded.
    pub manual_edges_loaded: usize,
}

/// An error that occurred during building.
#[derive(Debug, Clone)]
pub struct BuildError {
    /// Path to the problematic file.
    pub file: PathBuf,
    /// Error message.
    pub message: String,
}

/// Manual edge definition loaded from JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManualEdge {
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Relationship type name (maps to Relationship enum).
    pub relationship: String,
    /// Optional weight override.
    pub weight: Option<f32>,
}

/// Builder for constructing knowledge graphs.
///
/// Generic over `E: GraphExtractor` to support any domain.
pub struct GraphBuilder<E: GraphExtractor> {
    extractor: E,
    content_path: Option<PathBuf>,
    manual_edges_path: Option<PathBuf>,
    error_handling: ErrorHandling,
    progress_callback: Option<ProgressCallback>,
}

impl<E: GraphExtractor> GraphBuilder<E> {
    /// Creates a new builder with the given extractor.
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            content_path: None,
            manual_edges_path: None,
            error_handling: ErrorHandling::default(),
            progress_callback: None,
        }
    }

    /// Sets the content directory path.
    pub fn with_content_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.content_path = Some(path.into());
        self
    }

    /// Adds manual edges from a JSON file.
    ///
    /// Manual edges supplement extracted edges, useful for human-curated
    /// corrections or cross-source links that can't be derived from content.
    ///
    /// # JSON Format
    ///
    /// ```json
    /// [
    ///   {"from": "concept-a", "to": "concept-b", "relationship": "prerequisite"},
    ///   {"from": "concept-c", "to": "concept-d", "relationship": "relates_to", "weight": 0.8}
    /// ]
    /// ```
    pub fn with_manual_edges(mut self, path: impl Into<PathBuf>) -> Self {
        self.manual_edges_path = Some(path.into());
        self
    }

    /// Sets the error handling strategy.
    pub fn with_error_handling(mut self, handling: ErrorHandling) -> Self {
        self.error_handling = handling;
        self
    }

    /// Sets a progress callback.
    pub fn with_progress(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Builds the graph.
    ///
    /// This is the main entry point that:
    /// 1. Discovers content files
    /// 2. Extracts nodes and edges via the extractor
    /// 3. Loads manual edges
    /// 4. Constructs the final GraphData
    pub async fn build(self) -> Result<BuildResult> {
        let content_path = self.content_path.ok_or_else(|| {
            Error::config("Content path not set. Use with_content_path() first.")
        })?;

        // Discover files
        let glob_pattern = self.extractor.content_glob();
        let files = discover_files(&content_path, glob_pattern).await?;

        let mut progress = BuildProgress::new(files.len());
        let mut errors: Vec<BuildError> = Vec::new();

        // Build graph structure
        let mut petgraph: DiGraph<Node, Edge> = DiGraph::new();
        let mut node_indices = HashMap::new();
        let mut nodes = HashMap::new();
        let mut all_edges: Vec<Edge> = Vec::new();

        // Temporary storage for edge data (processed after all nodes exist)
        let mut pending_edges: Vec<(String, E::EdgeData)> = Vec::new();

        // Process each file
        for file_path in &files {
            progress.current_file = Some(file_path.clone());

            match self.process_file(&content_path, file_path).await {
                Ok((node_data, edge_data)) => {
                    let node = self.extractor.to_graph_node(&node_data);
                    let idx = petgraph.add_node(node.clone());
                    node_indices.insert(node.id.clone(), idx);
                    nodes.insert(node.id.clone(), node.clone());
                    progress.nodes_created += 1;

                    if let Some(edges) = edge_data {
                        pending_edges.push((node.id.clone(), edges));
                    }
                }
                Err(e) => {
                    let build_error = BuildError {
                        file: file_path.clone(),
                        message: e.to_string(),
                    };

                    match self.error_handling {
                        ErrorHandling::FailFast => return Err(e),
                        ErrorHandling::Collect | ErrorHandling::Skip => {
                            progress.errors.push(e.to_string());
                            errors.push(build_error);
                        }
                    }
                }
            }

            progress.files_processed += 1;
            if let Some(ref callback) = self.progress_callback {
                callback(progress.clone());
            }
        }

        // Process edges (now that all nodes exist)
        for (from_id, edge_data) in pending_edges {
            let edges = self.extractor.to_graph_edges(&from_id, &edge_data);
            for edge in edges {
                // Only add edge if both nodes exist
                if let (Some(&from_idx), Some(&to_idx)) =
                    (node_indices.get(&edge.from), node_indices.get(&edge.to))
                {
                    petgraph.add_edge(from_idx, to_idx, edge.clone());
                    all_edges.push(edge);
                    progress.edges_created += 1;
                }
                // Skip edges with missing nodes (could log warning)
            }
        }

        // Load manual edges
        let manual_edges_loaded = if let Some(ref manual_path) = self.manual_edges_path {
            self.load_manual_edges(manual_path, &node_indices, &mut petgraph, &mut all_edges)?
        } else {
            0
        };

        let graph = GraphData {
            graph: petgraph,
            node_indices,
            nodes,
            edges: all_edges,
        };

        Ok(BuildResult {
            graph,
            files_processed: progress.files_processed,
            files_skipped: errors.len(),
            errors,
            manual_edges_loaded,
        })
    }

    /// Process a single file to extract node and edge data.
    async fn process_file(
        &self,
        base_path: &Path,
        file_path: &Path,
    ) -> Result<(E::NodeData, Option<E::EdgeData>)> {
        // Read file content
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| Error::io_with_path(e, file_path))?;

        // Extract frontmatter
        let fm_result = extract_frontmatter(&content)?;
        let frontmatter = fm_result.frontmatter;
        let body = fm_result.content;

        // Extract node data
        let node_data = self
            .extractor
            .extract_node(base_path, file_path, &frontmatter, body)?;

        // Extract edge data
        let edge_data = self.extractor.extract_edges(&frontmatter, body)?;

        Ok((node_data, edge_data))
    }

    /// Load manual edges from JSON.
    fn load_manual_edges(
        &self,
        path: &Path,
        node_indices: &HashMap<String, petgraph::graph::NodeIndex>,
        graph: &mut DiGraph<Node, Edge>,
        all_edges: &mut Vec<Edge>,
    ) -> Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        let json = std::fs::read_to_string(path).map_err(|e| Error::io_with_path(e, path))?;

        let manual_edges: Vec<ManualEdge> = serde_json::from_str(&json)
            .map_err(|e| Error::parse(format!("Failed to parse manual edges: {}", e)))?;

        let mut loaded = 0;
        for manual in manual_edges {
            if let (Some(&from_idx), Some(&to_idx)) = (
                node_indices.get(&manual.from),
                node_indices.get(&manual.to),
            ) {
                let relationship = parse_relationship(&manual.relationship);
                let weight = manual.weight.unwrap_or_else(|| relationship.default_weight());

                let edge = Edge {
                    from: manual.from.clone(),
                    to: manual.to.clone(),
                    relationship,
                    weight,
                    origin: EdgeOrigin::Manual,
                };

                graph.add_edge(from_idx, to_idx, edge.clone());
                all_edges.push(edge);
                loaded += 1;
            }
        }

        Ok(loaded)
    }
}

/// Parse a relationship string to Relationship enum.
fn parse_relationship(s: &str) -> crate::Relationship {
    use crate::Relationship;
    match s.to_lowercase().as_str() {
        "prerequisite" | "prereq" => Relationship::Prerequisite,
        "leads_to" | "leadsto" => Relationship::LeadsTo,
        "relates_to" | "relatesto" | "related" => Relationship::RelatesTo,
        "extends" => Relationship::Extends,
        "introduces" => Relationship::Introduces,
        "covers" => Relationship::Covers,
        "variant_of" | "variantof" => Relationship::VariantOf,
        other => Relationship::Custom(other.to_string()),
    }
}

/// Discover content files matching a glob pattern.
async fn discover_files(base_path: &Path, glob_pattern: &str) -> Result<Vec<PathBuf>> {
    use fabryk_core::util::files::{find_all_files, FindOptions};

    let options = FindOptions {
        pattern: Some(glob_pattern.to_string()),
        ..Default::default()
    };

    let files = find_all_files(base_path, options).await?;
    let paths: Vec<PathBuf> = files.into_iter().map(|f| f.path).collect();

    Ok(paths)
}
```

### Step 2: Update lib.rs

Update `fabryk-graph/src/lib.rs`:

```rust
pub mod algorithms;
pub mod builder;
pub mod extractor;
pub mod persistence;
pub mod types;

// Re-exports
pub use algorithms::{
    calculate_centrality, find_bridges, get_related, neighborhood, prerequisites_sorted,
    shortest_path, CentralityScore, NeighborhoodResult, PathResult, PrerequisitesResult,
};
pub use builder::{BuildError, BuildProgress, BuildResult, ErrorHandling, GraphBuilder, ManualEdge};
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

### Step 3: Add builder tests

Add to `fabryk-graph/src/builder.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::mock::MockExtractor;
    use crate::Relationship;
    use tempfile::tempdir;

    async fn setup_test_files() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        // Create test files
        let file_a = r#"---
title: "Concept A"
category: "basics"
prerequisites:
  - concept-b
---

# Concept A

Content here.
"#;

        let file_b = r#"---
title: "Concept B"
category: "fundamentals"
---

# Concept B

Foundation content.
"#;

        std::fs::write(content_dir.join("concept-a.md"), file_a).unwrap();
        std::fs::write(content_dir.join("concept-b.md"), file_b).unwrap();

        (dir, content_dir)
    }

    #[tokio::test]
    async fn test_builder_basic() {
        let (_dir, content_dir) = setup_test_files().await;

        let extractor = MockExtractor;
        let result = GraphBuilder::new(extractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();

        assert_eq!(result.files_processed, 2);
        assert_eq!(result.graph.node_count(), 2);
        assert!(result.graph.contains_node("concept-a"));
        assert!(result.graph.contains_node("concept-b"));
    }

    #[tokio::test]
    async fn test_builder_extracts_edges() {
        let (_dir, content_dir) = setup_test_files().await;

        let extractor = MockExtractor;
        let result = GraphBuilder::new(extractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();

        // concept-a has prerequisite concept-b
        assert!(result.graph.edge_count() >= 1);
    }

    #[tokio::test]
    async fn test_builder_manual_edges() {
        let (_dir, content_dir) = setup_test_files().await;
        let manual_edges_path = content_dir.parent().unwrap().join("manual_edges.json");

        let manual_edges = r#"[
            {"from": "concept-a", "to": "concept-b", "relationship": "relates_to", "weight": 0.9}
        ]"#;
        std::fs::write(&manual_edges_path, manual_edges).unwrap();

        let extractor = MockExtractor;
        let result = GraphBuilder::new(extractor)
            .with_content_path(&content_dir)
            .with_manual_edges(&manual_edges_path)
            .build()
            .await
            .unwrap();

        assert_eq!(result.manual_edges_loaded, 1);
    }

    #[tokio::test]
    async fn test_builder_error_handling_collect() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        // Create a valid file and an invalid one
        std::fs::write(
            content_dir.join("valid.md"),
            "---\ntitle: Valid\n---\nContent",
        )
        .unwrap();
        std::fs::write(content_dir.join("invalid.md"), "not yaml frontmatter").unwrap();

        let extractor = MockExtractor;
        let result = GraphBuilder::new(extractor)
            .with_content_path(&content_dir)
            .with_error_handling(ErrorHandling::Collect)
            .build()
            .await
            .unwrap();

        assert_eq!(result.files_processed, 2);
        assert!(result.files_skipped > 0 || result.graph.node_count() < 2);
    }

    #[tokio::test]
    async fn test_builder_missing_content_path() {
        let extractor = MockExtractor;
        let result = GraphBuilder::new(extractor).build().await;

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_relationship() {
        assert_eq!(parse_relationship("prerequisite"), Relationship::Prerequisite);
        assert_eq!(parse_relationship("prereq"), Relationship::Prerequisite);
        assert_eq!(parse_relationship("leads_to"), Relationship::LeadsTo);
        assert_eq!(parse_relationship("relates_to"), Relationship::RelatesTo);
        assert_eq!(parse_relationship("related"), Relationship::RelatesTo);
        assert_eq!(
            parse_relationship("custom_rel"),
            Relationship::Custom("custom_rel".to_string())
        );
    }

    #[tokio::test]
    async fn test_builder_with_progress() {
        let (_dir, content_dir) = setup_test_files().await;

        let progress_updates = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let updates_clone = progress_updates.clone();

        let callback: ProgressCallback = Box::new(move |progress| {
            updates_clone.lock().unwrap().push(progress.files_processed);
        });

        let extractor = MockExtractor;
        let _result = GraphBuilder::new(extractor)
            .with_content_path(&content_dir)
            .with_progress(callback)
            .build()
            .await
            .unwrap();

        let updates = progress_updates.lock().unwrap();
        assert!(!updates.is_empty());
    }
}
```

### Step 4: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-graph
cargo test -p fabryk-graph
cargo clippy -p fabryk-graph -- -D warnings
```

## Exit Criteria

- [ ] `GraphBuilder<E: GraphExtractor>` generic struct defined
- [ ] `with_content_path()` sets content directory
- [ ] `with_manual_edges()` loads supplementary JSON edges (Amendment §2f-iii)
- [ ] `with_error_handling()` configures fail-fast vs collect vs skip
- [ ] `with_progress()` enables progress callbacks
- [ ] `build()` async method orchestrates full graph construction
- [ ] Manual edges marked with `EdgeOrigin::Manual`
- [ ] `BuildResult` contains graph, stats, and error info
- [ ] All tests pass

## Commit Message

```
feat(graph): add GraphBuilder for graph construction

Add GraphBuilder<E: GraphExtractor> that orchestrates:
- Content file discovery via glob patterns
- Node extraction via extractor trait
- Edge extraction and connection
- Manual edges from JSON (Amendment §2f-iii)
- Configurable error handling (fail-fast, collect, skip)
- Progress reporting via callbacks

BuildResult provides stats on processed files, skipped files,
errors, and manual edges loaded.

Phase 4 milestone 4.5 of Fabryk extraction.

Ref: Doc 0011 §4.4 (graph builder)
Ref: Doc 0012 §2f-iii (manual edges support)
Ref: Doc 0013 Phase 4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
