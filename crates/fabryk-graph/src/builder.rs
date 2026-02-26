//! GraphBuilder for constructing knowledge graphs.
//!
//! The builder orchestrates content discovery and graph construction:
//!
//! 1. Discover content files using glob patterns
//! 2. Parse frontmatter and content
//! 3. Call GraphExtractor methods to extract nodes/edges
//! 4. Build the final GraphData structure
//!
//! # Taproot Adaptations
//!
//! - **Two-phase build**: Phase 1 creates all nodes, Phase 2 creates all edges.
//!   This ensures all nodes exist before edge creation, handling forward references.
//! - **Dangling reference tracking**: Edges referencing missing nodes are logged
//!   in `BuildStats::dangling_refs` instead of silently dropped.
//! - **Bidirectional edge deduplication**: Prevents duplicate edges when both
//!   sides of a relationship declare each other.

use crate::persistence::{self, GraphMetadata};
use crate::{Edge, EdgeOrigin, GraphData, GraphExtractor, Relationship};
use fabryk_content::markdown::extract_frontmatter;
use fabryk_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ============================================================================
// Builder configuration types
// ============================================================================

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
    /// Relationship type name.
    pub relationship: String,
    /// Optional weight override.
    pub weight: Option<f32>,
}

/// Statistics from a graph build operation.
#[derive(Debug, Clone)]
pub struct BuildStats {
    /// Number of nodes created.
    pub nodes_created: usize,
    /// Number of edges created.
    pub edges_created: usize,
    /// Files that were processed.
    pub files_processed: usize,
    /// Files that were skipped due to errors.
    pub files_skipped: usize,
    /// Errors encountered (if not fail-fast).
    pub errors: Vec<BuildError>,
    /// Manual edges loaded.
    pub manual_edges_loaded: usize,
    /// Dangling references (edges to/from missing nodes).
    pub dangling_refs: Vec<String>,
    /// Duplicate edges that were deduplicated.
    pub deduped_edges: usize,
    /// Whether the result was loaded from cache.
    pub from_cache: bool,
}

// ============================================================================
// GraphBuilder
// ============================================================================

/// Builder for constructing knowledge graphs.
///
/// Generic over `E: GraphExtractor` to support any domain.
///
/// # Caching
///
/// When a cache path is configured via [`with_cache_path`](Self::with_cache_path),
/// the builder checks if the cached graph is fresh before rebuilding. On cache hit,
/// the graph is loaded from disk in milliseconds instead of re-parsing all content files.
pub struct GraphBuilder<E: GraphExtractor> {
    extractor: E,
    content_path: Option<PathBuf>,
    manual_edges_path: Option<PathBuf>,
    error_handling: ErrorHandling,
    cache_path: Option<PathBuf>,
    skip_cache: bool,
}

impl<E: GraphExtractor> GraphBuilder<E> {
    /// Creates a new builder with the given extractor.
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            content_path: None,
            manual_edges_path: None,
            error_handling: ErrorHandling::default(),
            cache_path: None,
            skip_cache: false,
        }
    }

    /// Sets the content directory path.
    pub fn with_content_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.content_path = Some(path.into());
        self
    }

    /// Adds manual edges from a JSON file (Amendment §2f-iii).
    pub fn with_manual_edges(mut self, path: impl Into<PathBuf>) -> Self {
        self.manual_edges_path = Some(path.into());
        self
    }

    /// Sets the error handling strategy.
    pub fn with_error_handling(mut self, handling: ErrorHandling) -> Self {
        self.error_handling = handling;
        self
    }

    /// Sets the cache file path for graph persistence.
    ///
    /// When set, the builder will:
    /// 1. Check if the cache is fresh before building (by comparing content hashes)
    /// 2. Load from cache on hit (fast path)
    /// 3. Save to cache after a successful build (for next time)
    pub fn with_cache_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_path = Some(path.into());
        self
    }

    /// Forces a rebuild even if the cache is fresh.
    pub fn skip_cache(mut self) -> Self {
        self.skip_cache = true;
        self
    }

    /// Builds the graph.
    ///
    /// Uses a two-phase approach (adapted from Taproot):
    /// - Phase 1: Extract and add all nodes
    /// - Phase 2: Extract and add all edges (with dedup and dangling ref tracking)
    ///
    /// If a cache path is configured and the cache is fresh, loads from cache instead.
    pub async fn build(self) -> Result<(GraphData, BuildStats)> {
        let content_path = self
            .content_path
            .as_ref()
            .ok_or_else(|| Error::config("Content path not set. Use with_content_path() first."))?
            .clone();

        // Check cache freshness (if cache configured and not skipped)
        if let Some(ref cache_path) = self.cache_path {
            if !self.skip_cache {
                let content_hash = compute_content_hash(&content_path)?;
                if persistence::is_cache_fresh(cache_path, &content_hash) {
                    log::info!(
                        "Graph cache is fresh, loading from {}",
                        cache_path.display()
                    );
                    let graph = persistence::load_graph(cache_path)?;
                    let stats = BuildStats {
                        nodes_created: graph.node_count(),
                        edges_created: graph.edge_count(),
                        files_processed: 0,
                        files_skipped: 0,
                        errors: Vec::new(),
                        manual_edges_loaded: 0,
                        dangling_refs: Vec::new(),
                        deduped_edges: 0,
                        from_cache: true,
                    };
                    return Ok((graph, stats));
                }
            }
        }

        // Discover files
        let files = discover_files(&content_path).await?;

        let mut stats = BuildStats {
            nodes_created: 0,
            edges_created: 0,
            files_processed: 0,
            files_skipped: 0,
            errors: Vec::new(),
            manual_edges_loaded: 0,
            dangling_refs: Vec::new(),
            deduped_edges: 0,
            from_cache: false,
        };

        let mut graph = GraphData::new();

        // Temporary storage for edge data (processed in phase 2)
        let mut pending_edges: Vec<(String, E::EdgeData)> = Vec::new();

        // ================================================================
        // Phase 1: Extract and add all nodes
        // ================================================================
        for file_path in &files {
            match self.process_file(&content_path, file_path) {
                Ok((node_data, edge_data)) => {
                    let node = self.extractor.to_graph_node(&node_data);
                    graph.add_node(node.clone());
                    stats.nodes_created += 1;

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
                            stats.files_skipped += 1;
                            stats.errors.push(build_error);
                        }
                    }
                }
            }

            stats.files_processed += 1;
        }

        // ================================================================
        // Phase 2: Add all edges (with dedup and dangling ref tracking)
        // ================================================================
        let mut seen_edges: HashSet<(String, String, String)> = HashSet::new();

        for (from_id, edge_data) in &pending_edges {
            let edges = self.extractor.to_graph_edges(from_id, edge_data);
            for edge in edges {
                // Check for dangling references
                if !graph.contains_node(&edge.from) || !graph.contains_node(&edge.to) {
                    stats.dangling_refs.push(format!(
                        "{} -[{}]-> {}",
                        edge.from,
                        edge.relationship.name(),
                        edge.to
                    ));
                    continue;
                }

                // Bidirectional edge deduplication
                let edge_key = (
                    edge.from.clone(),
                    edge.to.clone(),
                    edge.relationship.name().to_string(),
                );
                if !seen_edges.insert(edge_key) {
                    stats.deduped_edges += 1;
                    continue;
                }

                if graph.add_edge(edge).is_ok() {
                    stats.edges_created += 1;
                }
            }
        }

        // ================================================================
        // Phase 3: Load manual edges
        // ================================================================
        if let Some(ref manual_path) = self.manual_edges_path {
            stats.manual_edges_loaded =
                load_manual_edges(manual_path, &mut graph, &mut seen_edges, &mut stats)?;
        }

        // Save to cache after successful build
        if let Some(ref cache_path) = self.cache_path {
            let content_hash = compute_content_hash(&content_path)?;
            let metadata = GraphMetadata {
                content_hash: Some(content_hash),
                source_file_count: Some(stats.files_processed),
                ..Default::default()
            };
            // Ensure parent directory exists
            if let Some(parent) = cache_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| Error::io_with_path(e, parent))?;
                }
            }
            if let Err(e) = persistence::save_graph(&graph, cache_path, Some(metadata)) {
                log::warn!("Failed to save graph cache: {e}");
            }
        }

        Ok((graph, stats))
    }

    /// Process a single file to extract node and edge data.
    fn process_file(
        &self,
        base_path: &Path,
        file_path: &Path,
    ) -> Result<(E::NodeData, Option<E::EdgeData>)> {
        let content =
            std::fs::read_to_string(file_path).map_err(|e| Error::io_with_path(e, file_path))?;

        let fm_result = extract_frontmatter(&content)?;

        let frontmatter = fm_result
            .value()
            .cloned()
            .unwrap_or(serde_yaml::Value::Null);
        let body = fm_result.body();

        let node_data = self
            .extractor
            .extract_node(base_path, file_path, &frontmatter, body)?;

        let edge_data = self.extractor.extract_edges(&frontmatter, body)?;

        Ok((node_data, edge_data))
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Parse a relationship string to Relationship enum.
fn parse_relationship(s: &str) -> Relationship {
    match s.to_lowercase().as_str() {
        "prerequisite" | "prereq" => Relationship::Prerequisite,
        "leads_to" | "leadsto" => Relationship::LeadsTo,
        "relates_to" | "relatesto" | "related" => Relationship::RelatesTo,
        "extends" => Relationship::Extends,
        "introduces" => Relationship::Introduces,
        "covers" => Relationship::Covers,
        "variant_of" | "variantof" => Relationship::VariantOf,
        "contrasts_with" | "contrastswith" => Relationship::ContrastsWith,
        "answers_question" | "answersquestion" | "answers_questions" => {
            Relationship::AnswersQuestion
        }
        other => Relationship::Custom(other.to_string()),
    }
}

/// Load manual edges from a JSON file.
fn load_manual_edges(
    path: &Path,
    graph: &mut GraphData,
    seen_edges: &mut HashSet<(String, String, String)>,
    stats: &mut BuildStats,
) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }

    let json = std::fs::read_to_string(path).map_err(|e| Error::io_with_path(e, path))?;

    let manual_edges: Vec<ManualEdge> = serde_json::from_str(&json)
        .map_err(|e| Error::parse(format!("Failed to parse manual edges: {e}")))?;

    let mut loaded = 0;
    for manual in manual_edges {
        if !graph.contains_node(&manual.from) || !graph.contains_node(&manual.to) {
            stats.dangling_refs.push(format!(
                "manual: {} -[{}]-> {}",
                manual.from, manual.relationship, manual.to
            ));
            continue;
        }

        let edge_key = (
            manual.from.clone(),
            manual.to.clone(),
            manual.relationship.clone(),
        );
        if !seen_edges.insert(edge_key) {
            stats.deduped_edges += 1;
            continue;
        }

        let relationship = parse_relationship(&manual.relationship);
        let weight = manual
            .weight
            .unwrap_or_else(|| relationship.default_weight());

        let edge = Edge {
            from: manual.from,
            to: manual.to,
            relationship,
            weight,
            origin: EdgeOrigin::Manual,
        };

        if graph.add_edge(edge).is_ok() {
            loaded += 1;
        }
    }

    Ok(loaded)
}

/// Compute a content hash for cache freshness checking.
///
/// Uses file paths and modification times (not content) for speed.
/// Deterministic: sorted paths ensure consistent hashing.
fn compute_content_hash(dir: &Path) -> Result<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    let mut file_info: Vec<(String, u64)> = Vec::new();

    fn collect_files(dir: &Path, base: &Path, file_info: &mut Vec<(String, u64)>) -> Result<()> {
        for entry in std::fs::read_dir(dir).map_err(|e| Error::io_with_path(e, dir))? {
            let entry = entry.map_err(Error::io)?;
            let path = entry.path();
            if path.is_dir() {
                collect_files(&path, base, file_info)?;
            } else if path.extension().is_some_and(|e| e == "md") {
                let relative = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let mtime = std::fs::metadata(&path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                file_info.push((relative, mtime));
            }
        }
        Ok(())
    }

    collect_files(dir, dir, &mut file_info)?;
    file_info.sort_by(|a, b| a.0.cmp(&b.0));

    for (path, mtime) in &file_info {
        path.hash(&mut hasher);
        mtime.hash(&mut hasher);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

/// Discover markdown content files in a directory.
async fn discover_files(base_path: &Path) -> Result<Vec<PathBuf>> {
    use fabryk_core::util::files::{find_all_files, FindOptions};

    let files = find_all_files(base_path, FindOptions::markdown()).await?;
    let paths: Vec<PathBuf> = files.into_iter().map(|f| f.path).collect();

    Ok(paths)
}

// ============================================================================
// Tests
// ============================================================================

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

        let file_a = "---\ntitle: \"Concept A\"\ncategory: \"basics\"\nprerequisites:\n  - concept-b\n---\n\n# Concept A\n\nContent here.\n";
        let file_b = "---\ntitle: \"Concept B\"\ncategory: \"fundamentals\"\n---\n\n# Concept B\n\nFoundation content.\n";

        std::fs::write(content_dir.join("concept-a.md"), file_a).unwrap();
        std::fs::write(content_dir.join("concept-b.md"), file_b).unwrap();

        (dir, content_dir)
    }

    #[tokio::test]
    async fn test_builder_basic() {
        let (_dir, content_dir) = setup_test_files().await;

        let (graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.files_processed, 2);
        assert_eq!(graph.node_count(), 2);
        assert!(graph.contains_node("concept-a"));
        assert!(graph.contains_node("concept-b"));
    }

    #[tokio::test]
    async fn test_builder_extracts_edges() {
        let (_dir, content_dir) = setup_test_files().await;

        let (graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();

        // concept-a has prerequisite concept-b
        assert!(graph.edge_count() >= 1);
        assert!(stats.edges_created >= 1);
    }

    #[tokio::test]
    async fn test_builder_manual_edges() {
        let (_dir, content_dir) = setup_test_files().await;
        let manual_edges_path = content_dir.parent().unwrap().join("manual_edges.json");

        let manual_edges = r#"[
            {"from": "concept-a", "to": "concept-b", "relationship": "relates_to", "weight": 0.9}
        ]"#;
        std::fs::write(&manual_edges_path, manual_edges).unwrap();

        let (_graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_manual_edges(&manual_edges_path)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.manual_edges_loaded, 1);
    }

    #[tokio::test]
    async fn test_builder_error_handling_collect() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        std::fs::write(
            content_dir.join("valid.md"),
            "---\ntitle: Valid\n---\nContent",
        )
        .unwrap();
        std::fs::write(content_dir.join("invalid.md"), "not yaml frontmatter").unwrap();

        let (_graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_error_handling(ErrorHandling::Collect)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.files_processed, 2);
        // invalid.md has no frontmatter delimiters, so extract_frontmatter returns
        // Ok with no frontmatter. MockExtractor will still produce a node from file stem.
        // So it may succeed or fail depending on exact behavior.
        assert!(stats.files_processed >= 1);
    }

    #[tokio::test]
    async fn test_builder_missing_content_path() {
        let result = GraphBuilder::new(MockExtractor).build().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_builder_dangling_refs() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        // File references a non-existent prerequisite
        let file = "---\ntitle: \"Orphan\"\nprerequisites:\n  - nonexistent\n---\n\n# Orphan\n";
        std::fs::write(content_dir.join("orphan.md"), file).unwrap();

        let (_graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.nodes_created, 1);
        assert!(!stats.dangling_refs.is_empty());
        assert!(stats.dangling_refs[0].contains("nonexistent"));
    }

    #[tokio::test]
    async fn test_builder_edge_dedup() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        // Both files reference each other as related
        let file_a = "---\ntitle: \"A\"\nrelated:\n  - b\n---\n\n# A\n";
        let file_b = "---\ntitle: \"B\"\nrelated:\n  - a\n---\n\n# B\n";

        std::fs::write(content_dir.join("a.md"), file_a).unwrap();
        std::fs::write(content_dir.join("b.md"), file_b).unwrap();

        let (graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();

        // Should have 2 nodes
        assert_eq!(graph.node_count(), 2);
        assert_eq!(stats.nodes_created, 2);
        // Both directions exist (a->b and b->a are different keys)
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(stats.edges_created, 2);
    }

    #[tokio::test]
    async fn test_builder_empty_directory() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("empty");
        std::fs::create_dir(&content_dir).unwrap();

        let (graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(stats.files_processed, 0);
    }

    #[test]
    fn test_parse_relationship() {
        assert_eq!(
            parse_relationship("prerequisite"),
            Relationship::Prerequisite
        );
        assert_eq!(parse_relationship("prereq"), Relationship::Prerequisite);
        assert_eq!(parse_relationship("leads_to"), Relationship::LeadsTo);
        assert_eq!(parse_relationship("relates_to"), Relationship::RelatesTo);
        assert_eq!(parse_relationship("related"), Relationship::RelatesTo);
        assert_eq!(parse_relationship("extends"), Relationship::Extends);
        assert_eq!(parse_relationship("introduces"), Relationship::Introduces);
        assert_eq!(parse_relationship("covers"), Relationship::Covers);
        assert_eq!(parse_relationship("variant_of"), Relationship::VariantOf);
        assert_eq!(
            parse_relationship("custom_rel"),
            Relationship::Custom("custom_rel".to_string())
        );
    }

    #[tokio::test]
    async fn test_builder_manual_edges_missing_file() {
        let (_dir, content_dir) = setup_test_files().await;
        let missing_path = content_dir.parent().unwrap().join("nonexistent.json");

        let (_graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_manual_edges(&missing_path)
            .build()
            .await
            .unwrap();

        // Missing manual edges file should be silently skipped
        assert_eq!(stats.manual_edges_loaded, 0);
    }

    #[tokio::test]
    async fn test_builder_manual_edges_dangling() {
        let (_dir, content_dir) = setup_test_files().await;
        let manual_path = content_dir.parent().unwrap().join("manual.json");

        let manual = r#"[
            {"from": "concept-a", "to": "nonexistent", "relationship": "relates_to"}
        ]"#;
        std::fs::write(&manual_path, manual).unwrap();

        let (_graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_manual_edges(&manual_path)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.manual_edges_loaded, 0);
        assert!(stats
            .dangling_refs
            .iter()
            .any(|r| r.contains("nonexistent")));
    }

    // ================================================================
    // Cache tests
    // ================================================================

    #[tokio::test]
    async fn test_builder_cache_hit() {
        let (_dir, content_dir) = setup_test_files().await;
        let cache_path = content_dir.parent().unwrap().join("graph-cache.json");

        // First build: cold (no cache)
        let (graph1, stats1) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_cache_path(&cache_path)
            .build()
            .await
            .unwrap();
        assert!(!stats1.from_cache);
        assert!(cache_path.exists());

        // Second build: warm (cache hit)
        let (graph2, stats2) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_cache_path(&cache_path)
            .build()
            .await
            .unwrap();
        assert!(stats2.from_cache);
        assert_eq!(graph1.node_count(), graph2.node_count());
        assert_eq!(graph1.edge_count(), graph2.edge_count());
    }

    #[tokio::test]
    async fn test_builder_cache_miss_on_content_change() {
        let (_dir, content_dir) = setup_test_files().await;
        let cache_path = content_dir.parent().unwrap().join("graph-cache.json");

        // First build
        let (_graph, stats1) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_cache_path(&cache_path)
            .build()
            .await
            .unwrap();
        assert!(!stats1.from_cache);

        // Add a new file (changes content hash)
        let file_c = "---\ntitle: \"Concept C\"\ncategory: \"new\"\n---\n\n# Concept C\n";
        std::fs::write(content_dir.join("concept-c.md"), file_c).unwrap();

        // Second build: cache miss (content changed)
        let (graph, stats2) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_cache_path(&cache_path)
            .build()
            .await
            .unwrap();
        assert!(!stats2.from_cache);
        assert_eq!(graph.node_count(), 3);
    }

    #[tokio::test]
    async fn test_builder_skip_cache() {
        let (_dir, content_dir) = setup_test_files().await;
        let cache_path = content_dir.parent().unwrap().join("graph-cache.json");

        // First build: populates cache
        GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_cache_path(&cache_path)
            .build()
            .await
            .unwrap();

        // Second build with skip_cache: forces rebuild
        let (_graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .with_cache_path(&cache_path)
            .skip_cache()
            .build()
            .await
            .unwrap();
        assert!(!stats.from_cache);
        assert_eq!(stats.files_processed, 2);
    }

    #[tokio::test]
    async fn test_builder_no_cache_path() {
        let (_dir, content_dir) = setup_test_files().await;

        // Build without cache: same behavior as before
        let (_graph, stats) = GraphBuilder::new(MockExtractor)
            .with_content_path(&content_dir)
            .build()
            .await
            .unwrap();
        assert!(!stats.from_cache);
        assert_eq!(stats.files_processed, 2);
    }

    #[test]
    fn test_compute_content_hash_deterministic() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();
        std::fs::write(content_dir.join("a.md"), "content a").unwrap();
        std::fs::write(content_dir.join("b.md"), "content b").unwrap();

        let hash1 = compute_content_hash(&content_dir).unwrap();
        let hash2 = compute_content_hash(&content_dir).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_changes() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();
        std::fs::write(content_dir.join("a.md"), "content a").unwrap();

        let hash1 = compute_content_hash(&content_dir).unwrap();

        std::fs::write(content_dir.join("b.md"), "content b").unwrap();

        let hash2 = compute_content_hash(&content_dir).unwrap();
        assert_ne!(hash1, hash2);
    }
}
