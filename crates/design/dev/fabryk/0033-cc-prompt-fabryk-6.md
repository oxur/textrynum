---
title: "CC Prompt: Fabryk 6.2 — Graph CLI Commands"
milestone: "6.2"
phase: 6
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["6.1 CLI framework complete"]
governing-docs: [0011-audit §4.10, 0013-project-plan]
---

# CC Prompt: Fabryk 6.2 — Graph CLI Commands

## Context

This milestone implements the actual graph CLI command handlers using
`fabryk-graph` infrastructure. The handlers are parameterized over
`GraphExtractor` to work with any domain.

## Objective

Implement graph CLI command handlers:

1. `handle_build<E: GraphExtractor>()` - Build graph using extractor
2. `handle_validate()` - Run validation and report issues
3. `handle_stats()` - Show graph statistics
4. `handle_query()` - Query graph interactively

## Implementation Steps

### Step 1: Create graph CLI handlers

Create `fabryk-cli/src/graph_handlers.rs`:

```rust
//! Graph CLI command handlers.

use fabryk_core::traits::ConfigProvider;
use fabryk_core::Result;
use fabryk_graph::{
    compute_stats, load_graph, save_graph, validate_graph, GraphBuilder, GraphData,
    GraphExtractor, GraphMetadata,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Options for graph build command.
pub struct BuildOptions {
    pub output: Option<PathBuf>,
    pub dry_run: bool,
    pub manual_edges: Option<PathBuf>,
}

/// Handle the graph build command.
///
/// Parameterized over `E: GraphExtractor` to work with any domain.
pub async fn handle_build<C, E>(
    config: &C,
    extractor: E,
    options: BuildOptions,
) -> Result<()>
where
    C: ConfigProvider,
    E: GraphExtractor + 'static,
{
    let content_path = config.content_path("concepts")?;

    info!("Building graph from {}", content_path.display());
    info!("Extractor: {}", extractor.name());

    let mut builder = GraphBuilder::new(extractor).with_content_path(&content_path);

    if let Some(ref manual_path) = options.manual_edges {
        if manual_path.exists() {
            info!("Loading manual edges from {}", manual_path.display());
            builder = builder.with_manual_edges(manual_path);
        }
    }

    if options.dry_run {
        println!("Dry run - would build graph from:");
        println!("  Content path: {}", content_path.display());
        if let Some(ref manual_path) = options.manual_edges {
            println!("  Manual edges: {}", manual_path.display());
        }
        return Ok(());
    }

    let result = builder.build().await?;

    println!("Graph built successfully:");
    println!("  Nodes: {}", result.graph.node_count());
    println!("  Edges: {}", result.graph.edge_count());
    println!("  Files processed: {}", result.files_processed);
    println!("  Files skipped: {}", result.files_skipped);
    println!("  Manual edges: {}", result.manual_edges_loaded);

    if !result.errors.is_empty() {
        println!("\nWarnings ({}):", result.errors.len());
        for error in result.errors.iter().take(10) {
            println!("  - {}: {}", error.file.display(), error.message);
        }
        if result.errors.len() > 10 {
            println!("  ... and {} more", result.errors.len() - 10);
        }
    }

    // Save graph
    let output_path = options
        .output
        .unwrap_or_else(|| config.base_path().unwrap().join("data/graphs/graph.json"));

    let metadata = GraphMetadata {
        source_file_count: Some(result.files_processed),
        ..Default::default()
    };

    save_graph(&result.graph, &output_path, Some(metadata))?;
    println!("\nSaved to: {}", output_path.display());

    Ok(())
}

/// Handle the graph validate command.
pub async fn handle_validate<C: ConfigProvider>(config: &C) -> Result<()> {
    let graph_path = config.base_path()?.join("data/graphs/graph.json");

    if !graph_path.exists() {
        println!("Graph not found at {}", graph_path.display());
        println!("Run 'graph build' first.");
        return Ok(());
    }

    info!("Loading graph from {}", graph_path.display());
    let graph = load_graph(&graph_path)?;

    info!("Validating graph...");
    let result = validate_graph(&graph);

    if result.valid {
        println!("✓ Graph is valid");
    } else {
        println!("✗ Graph has issues");
    }

    println!("\nSummary:");
    println!("  Nodes: {}", graph.node_count());
    println!("  Edges: {}", graph.edge_count());

    if !result.errors.is_empty() {
        println!("\nErrors ({}):", result.errors.len());
        for error in &result.errors {
            println!("  [{}] {}", error.code, error.message);
            if !error.nodes.is_empty() {
                for node in error.nodes.iter().take(3) {
                    println!("    - {}", node);
                }
                if error.nodes.len() > 3 {
                    println!("    ... and {} more", error.nodes.len() - 3);
                }
            }
        }
    }

    if !result.warnings.is_empty() {
        println!("\nWarnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            println!("  [{}] {}", warning.code, warning.message);
        }
    }

    Ok(())
}

/// Handle the graph stats command.
pub async fn handle_stats<C: ConfigProvider>(config: &C) -> Result<()> {
    let graph_path = config.base_path()?.join("data/graphs/graph.json");

    if !graph_path.exists() {
        println!("Graph not found at {}", graph_path.display());
        println!("Run 'graph build' first.");
        return Ok(());
    }

    let graph = load_graph(&graph_path)?;
    let stats = compute_stats(&graph);

    println!("Graph Statistics");
    println!("================");
    println!();
    println!("Nodes: {}", stats.node_count);
    println!("  Canonical: {}", stats.canonical_count);
    println!("  Variants: {}", stats.variant_count);
    println!("  Orphans: {}", stats.orphan_count);
    println!();
    println!("Edges: {}", stats.edge_count);
    println!("  Avg degree: {:.2}", stats.avg_degree);
    println!("  Max in-degree: {}", stats.max_in_degree);
    println!("  Max out-degree: {}", stats.max_out_degree);
    println!();

    if let Some(ref node) = stats.most_depended_on {
        println!("Most depended on: {}", node);
    }
    if let Some(ref node) = stats.most_dependencies {
        println!("Most dependencies: {}", node);
    }

    println!();
    println!("Categories:");
    let mut categories: Vec<_> = stats.category_distribution.iter().collect();
    categories.sort_by(|a, b| b.1.cmp(a.1));
    for (cat, count) in categories.iter().take(10) {
        println!("  {}: {}", cat, count);
    }

    println!();
    println!("Relationships:");
    let mut relationships: Vec<_> = stats.relationship_distribution.iter().collect();
    relationships.sort_by(|a, b| b.1.cmp(a.1));
    for (rel, count) in relationships {
        println!("  {}: {}", rel, count);
    }

    Ok(())
}

/// Options for graph query command.
pub struct QueryOptions {
    pub id: String,
    pub query_type: String,
    pub to: Option<String>,
}

/// Handle the graph query command.
pub async fn handle_query<C: ConfigProvider>(config: &C, options: QueryOptions) -> Result<()> {
    let graph_path = config.base_path()?.join("data/graphs/graph.json");

    if !graph_path.exists() {
        println!("Graph not found at {}", graph_path.display());
        return Ok(());
    }

    let graph = load_graph(&graph_path)?;

    match options.query_type.as_str() {
        "related" => {
            let result = fabryk_graph::neighborhood(&graph, &options.id, 1, None)?;
            println!("Related to '{}':", result.center.title);
            for node in &result.nodes {
                println!("  - {} ({})", node.title, node.id);
            }
        }
        "prerequisites" => {
            let result = fabryk_graph::prerequisites_sorted(&graph, &options.id)?;
            println!("Prerequisites for '{}' (learn in this order):", result.target.title);
            for (i, node) in result.ordered.iter().enumerate() {
                println!("  {}. {} ({})", i + 1, node.title, node.id);
            }
            if result.has_cycles {
                println!("\n⚠ Warning: Cycles detected in prerequisites");
            }
        }
        "path" => {
            let to = options
                .to
                .ok_or_else(|| fabryk_core::Error::config("--to required for path query"))?;

            let result = fabryk_graph::shortest_path(&graph, &options.id, &to)?;
            if result.found {
                println!("Path from '{}' to '{}':", options.id, to);
                for (i, node) in result.path.iter().enumerate() {
                    println!("  {}. {} ({})", i + 1, node.title, node.id);
                }
                println!("\nTotal weight: {:.2}", result.total_weight);
            } else {
                println!("No path found from '{}' to '{}'", options.id, to);
            }
        }
        _ => {
            println!("Unknown query type: {}", options.query_type);
            println!("Available: related, prerequisites, path");
        }
    }

    Ok(())
}
```

### Step 2: Update app.rs

Update `fabryk-cli/src/app.rs` to use the handlers:

```rust
// Add to imports
use crate::graph_handlers::{handle_build, handle_stats, handle_validate, handle_query, BuildOptions, QueryOptions};

// Update handle_graph method
async fn handle_graph(&self, cmd: GraphCommand) -> Result<()> {
    match cmd {
        GraphCommand::Build { output, dry_run } => {
            // This is the base implementation - domains will override
            // by providing their own extractor
            println!("Base graph build not implemented.");
            println!("Domains should use handle_build<E>() with their extractor.");
            Ok(())
        }
        GraphCommand::Validate => {
            handle_validate(self.config.as_ref()).await
        }
        GraphCommand::Stats => {
            handle_stats(self.config.as_ref()).await
        }
        GraphCommand::Query { id, query_type, to } => {
            let options = QueryOptions { id, query_type, to };
            handle_query(self.config.as_ref(), options).await
        }
    }
}
```

### Step 3: Update lib.rs

```rust
pub mod app;
pub mod cli;
pub mod graph_handlers;

pub use app::FabrykCli;
pub use cli::{BaseCommand, CliArgs, CliExtension, GraphCommand};
pub use graph_handlers::{handle_build, handle_query, handle_stats, handle_validate, BuildOptions, QueryOptions};
```

### Step 4: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-cli
cargo test -p fabryk-cli
cargo clippy -p fabryk-cli -- -D warnings
```

## Exit Criteria

- [ ] `handle_build<E: GraphExtractor>()` builds graph using any extractor
- [ ] `handle_validate()` reports graph issues
- [ ] `handle_stats()` shows comprehensive statistics
- [ ] `handle_query()` supports related, prerequisites, path queries
- [ ] Build command shows progress and summary
- [ ] Validate command categorizes errors vs warnings
- [ ] Stats command shows categories and relationships
- [ ] All tests pass

## Commit Message

```
feat(cli): add graph CLI command handlers

Add handlers for graph CLI commands:
- handle_build<E: GraphExtractor>(): Build graph using any extractor
- handle_validate(): Validate structure, report errors/warnings
- handle_stats(): Show comprehensive statistics
- handle_query(): Query related, prerequisites, paths

Build handler shows progress, summary, and optional warnings.
All handlers parameterized over ConfigProvider for domain flexibility.

Phase 6 milestone 6.2 of Fabryk extraction.

Ref: Doc 0011 §4.10 (graph CLI)
Ref: Doc 0013 Phase 6

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
