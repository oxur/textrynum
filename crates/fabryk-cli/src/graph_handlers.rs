//! Handler functions for graph CLI commands.
//!
//! These functions implement the logic behind `graph validate`, `graph stats`,
//! `graph query`, and `graph build` (with a domain-provided extractor).

use fabryk_core::traits::ConfigProvider;
use fabryk_core::{Error, Result};
use fabryk_graph::{
    GraphBuilder, GraphData, GraphExtractor, GraphMetadata, compute_stats, load_graph,
    neighborhood, prerequisites_sorted, save_graph, shortest_path, validate_graph,
};
use std::path::PathBuf;

// ============================================================================
// Option types
// ============================================================================

/// Options for graph build operations.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Output file path (defaults to data/graphs/graph.json under base_path).
    pub output: Option<String>,
    /// If true, show what would be built without writing.
    pub dry_run: bool,
}

/// Options for graph query operations.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Node ID to query.
    pub id: String,
    /// Type of query: "related", "prerequisites", or "path".
    pub query_type: String,
    /// Target node for path queries.
    pub to: Option<String>,
}

// ============================================================================
// Helper: resolve graph path
// ============================================================================

/// Resolve the default graph file path from config.
fn graph_path<C: ConfigProvider>(config: &C) -> Result<PathBuf> {
    let base = config.base_path()?;
    Ok(base.join("data").join("graphs").join("graph.json"))
}

// ============================================================================
// Handlers
// ============================================================================

/// Build a knowledge graph using the provided extractor.
///
/// Two-phase build: discover content, build graph, optionally save.
pub async fn handle_build<C: ConfigProvider, E: GraphExtractor>(
    config: &C,
    extractor: E,
    options: BuildOptions,
) -> Result<()> {
    let content_path = config.content_path("concepts")?;
    let output_path = match options.output {
        Some(ref p) => PathBuf::from(p),
        None => graph_path(config)?,
    };

    println!("Building graph from: {}", content_path.display());

    let (graph, stats) = GraphBuilder::new(extractor)
        .with_content_path(&content_path)
        .build()
        .await?;

    println!("Graph built:");
    println!("  Nodes:           {}", stats.nodes_created);
    println!("  Edges:           {}", stats.edges_created);
    println!("  Files processed: {}", stats.files_processed);
    println!("  Files skipped:   {}", stats.files_skipped);
    if !stats.errors.is_empty() {
        println!("  Errors:          {}", stats.errors.len());
    }
    if !stats.dangling_refs.is_empty() {
        println!("  Dangling refs:   {}", stats.dangling_refs.len());
    }

    if options.dry_run {
        println!("\nDry run — graph not saved.");
    } else {
        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io_with_path(e, parent))?;
        }

        let metadata = GraphMetadata {
            source_file_count: Some(stats.files_processed),
            ..Default::default()
        };

        save_graph(&graph, &output_path, Some(metadata))?;
        println!("\nGraph saved to: {}", output_path.display());
    }

    Ok(())
}

/// Validate graph integrity.
pub async fn handle_validate<C: ConfigProvider>(config: &C) -> Result<()> {
    let path = graph_path(config)?;
    let graph = load_graph_or_error(&path)?;

    let result = validate_graph(&graph);

    if result.valid {
        println!("Graph is valid.");
    } else {
        println!("Graph has validation issues:");
    }

    for error in &result.errors {
        println!("  ERROR [{}]: {}", error.code, error.message);
        for node in &error.nodes {
            println!("    - {node}");
        }
        for edge in &error.edges {
            println!("    - {edge}");
        }
    }

    for warning in &result.warnings {
        println!("  WARN  [{}]: {}", warning.code, warning.message);
        for node in &warning.nodes {
            println!("    - {node}");
        }
    }

    println!(
        "\nSummary: {} error(s), {} warning(s)",
        result.errors.len(),
        result.warnings.len()
    );

    if result.valid {
        Ok(())
    } else {
        Err(Error::operation(format!(
            "Graph validation failed with {} error(s)",
            result.errors.len()
        )))
    }
}

/// Show graph statistics.
pub async fn handle_stats<C: ConfigProvider>(config: &C) -> Result<()> {
    let path = graph_path(config)?;
    let graph = load_graph_or_error(&path)?;

    let stats = compute_stats(&graph);

    println!("Graph Statistics");
    println!("================");
    println!("Nodes:          {}", stats.node_count);
    println!("  Canonical:    {}", stats.canonical_count);
    println!("  Variants:     {}", stats.variant_count);
    println!("  Orphans:      {}", stats.orphan_count);
    println!("Edges:          {}", stats.edge_count);
    println!("Avg degree:     {:.2}", stats.avg_degree);
    println!("Max in-degree:  {}", stats.max_in_degree);
    println!("Max out-degree: {}", stats.max_out_degree);

    if let Some(ref node_id) = stats.most_depended_on {
        println!(
            "Most depended on: {node_id} (in-degree: {})",
            stats.max_in_degree
        );
    }
    if let Some(ref node_id) = stats.most_dependencies {
        println!(
            "Most dependencies: {node_id} (out-degree: {})",
            stats.max_out_degree
        );
    }

    if !stats.category_distribution.is_empty() {
        println!("\nCategories:");
        let mut cats: Vec<_> = stats.category_distribution.iter().collect();
        cats.sort_by(|a, b| b.1.cmp(a.1));
        for (cat, count) in cats {
            println!("  {cat}: {count}");
        }
    }

    if !stats.relationship_distribution.is_empty() {
        println!("\nRelationships:");
        let mut rels: Vec<_> = stats.relationship_distribution.iter().collect();
        rels.sort_by(|a, b| b.1.cmp(a.1));
        for (rel, count) in rels {
            println!("  {rel}: {count}");
        }
    }

    Ok(())
}

/// Query the graph.
pub async fn handle_query<C: ConfigProvider>(config: &C, options: QueryOptions) -> Result<()> {
    let path = graph_path(config)?;
    let graph = load_graph_or_error(&path)?;

    match options.query_type.as_str() {
        "related" => query_related(&graph, &options.id).await,
        "prerequisites" => query_prerequisites(&graph, &options.id).await,
        "path" => {
            let to = options
                .to
                .ok_or_else(|| Error::config("--to is required for path queries"))?;
            query_path(&graph, &options.id, &to).await
        }
        other => Err(Error::config(format!("Unknown query type: {other}"))),
    }
}

// ============================================================================
// Query implementations
// ============================================================================

async fn query_related(graph: &GraphData, id: &str) -> Result<()> {
    let result = neighborhood(graph, id, 1, None)?;

    println!("Related to '{id}':");
    if result.nodes.is_empty() {
        println!("  (no related nodes)");
    } else {
        for node in &result.nodes {
            println!("  - {} ({})", node.id, node.title);
        }
    }
    println!("\n{} related node(s)", result.nodes.len());

    Ok(())
}

async fn query_prerequisites(graph: &GraphData, id: &str) -> Result<()> {
    let result = prerequisites_sorted(graph, id)?;

    println!("Prerequisites for '{}' (learning order):", result.target.id);
    if result.ordered.is_empty() {
        println!("  (no prerequisites)");
    } else {
        for (i, node) in result.ordered.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, node.id, node.title);
        }
    }
    if result.has_cycles {
        println!("\n  WARNING: Prerequisite cycle detected — ordering is approximate.");
    }

    Ok(())
}

async fn query_path(graph: &GraphData, from: &str, to: &str) -> Result<()> {
    let result = shortest_path(graph, from, to)?;

    if !result.found {
        println!("No path found from '{from}' to '{to}'.");
        return Ok(());
    }

    println!("Path from '{from}' to '{to}':");
    for (i, node) in result.path.iter().enumerate() {
        if i > 0
            && let Some(edge) = result.edges.get(i - 1)
        {
            println!("    --[{}]--> ", edge.relationship.name());
        }
        println!("  {}. {} ({})", i + 1, node.id, node.title);
    }
    println!("\nTotal weight: {:.2}", result.total_weight);

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

/// Load graph from the standard path, returning a helpful error message.
fn load_graph_or_error(path: &PathBuf) -> Result<GraphData> {
    if !path.exists() {
        return Err(Error::file_not_found(path));
    }
    load_graph(path)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use fabryk_graph::{Edge, Node, Relationship};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[derive(Clone)]
    struct TestConfig {
        base: PathBuf,
    }

    impl ConfigProvider for TestConfig {
        fn project_name(&self) -> &str {
            "test"
        }

        fn base_path(&self) -> Result<PathBuf> {
            Ok(self.base.clone())
        }

        fn content_path(&self, content_type: &str) -> Result<PathBuf> {
            Ok(self.base.join(content_type))
        }
    }

    /// Create a test graph and save it to the standard path.
    fn setup_graph(dir: &std::path::Path) -> PathBuf {
        let graph_dir = dir.join("data").join("graphs");
        std::fs::create_dir_all(&graph_dir).unwrap();
        let graph_path = graph_dir.join("graph.json");

        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "Node A").with_category("basics"));
        graph.add_node(Node::new("b", "Node B").with_category("basics"));
        graph.add_node(Node::new("c", "Node C").with_category("advanced"));
        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "c", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("a", "c", Relationship::RelatesTo))
            .unwrap();

        save_graph(&graph, &graph_path, None).unwrap();
        graph_path
    }

    // ------------------------------------------------------------------------
    // validate handler
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_validate_valid_graph() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let result = handle_validate(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_validate_missing_graph() {
        let dir = tempdir().unwrap();
        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let result = handle_validate(&config).await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // stats handler
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_stats() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let result = handle_stats(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_stats_missing_graph() {
        let dir = tempdir().unwrap();
        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let result = handle_stats(&config).await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // query handler: related
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_query_related() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let options = QueryOptions {
            id: "a".to_string(),
            query_type: "related".to_string(),
            to: None,
        };

        let result = handle_query(&config, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_query_related_unknown_node() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let options = QueryOptions {
            id: "nonexistent".to_string(),
            query_type: "related".to_string(),
            to: None,
        };

        let result = handle_query(&config, options).await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // query handler: prerequisites
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_query_prerequisites() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let options = QueryOptions {
            id: "c".to_string(),
            query_type: "prerequisites".to_string(),
            to: None,
        };

        let result = handle_query(&config, options).await;
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------------
    // query handler: path
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_query_path() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let options = QueryOptions {
            id: "a".to_string(),
            query_type: "path".to_string(),
            to: Some("c".to_string()),
        };

        let result = handle_query(&config, options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_query_path_missing_to() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let options = QueryOptions {
            id: "a".to_string(),
            query_type: "path".to_string(),
            to: None,
        };

        let result = handle_query(&config, options).await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // query handler: unknown type
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_query_unknown_type() {
        let dir = tempdir().unwrap();
        setup_graph(dir.path());

        let config = TestConfig {
            base: dir.path().to_path_buf(),
        };

        let options = QueryOptions {
            id: "a".to_string(),
            query_type: "unknown".to_string(),
            to: None,
        };

        let result = handle_query(&config, options).await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // build handler (dry run)
    // ------------------------------------------------------------------------

    #[test]
    fn test_build_options_default() {
        let options = BuildOptions {
            output: None,
            dry_run: true,
        };
        assert!(options.dry_run);
        assert!(options.output.is_none());
    }

    #[test]
    fn test_build_options_with_output() {
        let options = BuildOptions {
            output: Some("/tmp/graph.json".to_string()),
            dry_run: false,
        };
        assert!(!options.dry_run);
        assert_eq!(options.output.unwrap(), "/tmp/graph.json");
    }

    // ------------------------------------------------------------------------
    // helper: graph_path
    // ------------------------------------------------------------------------

    #[test]
    fn test_graph_path() {
        let config = TestConfig {
            base: PathBuf::from("/project"),
        };
        let path = graph_path(&config).unwrap();
        assert_eq!(path, PathBuf::from("/project/data/graphs/graph.json"));
    }

    // ------------------------------------------------------------------------
    // helper: load_graph_or_error
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_graph_or_error_missing() {
        let result = load_graph_or_error(&PathBuf::from("/nonexistent/graph.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_graph_or_error_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("graph.json");

        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A"));
        save_graph(&graph, &path, None).unwrap();

        let loaded = load_graph_or_error(&path).unwrap();
        assert_eq!(loaded.node_count(), 1);
    }
}
