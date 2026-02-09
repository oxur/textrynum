---
title: "CC Prompt: Fabryk 5.6 — Full MCP Integration"
milestone: "5.6"
phase: 5
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["5.1-5.5 complete"]
governing-docs: [0011-audit §7, 0013-project-plan]
---

# CC Prompt: Fabryk 5.6 — Full MCP Integration

## Context

This is the **final milestone of Phase 5**. It integrates all four fabryk-mcp
crates into ai-music-theory and verifies that all 25 MCP tools work correctly.

## Objective

Complete MCP integration in ai-music-theory:

1. Wire all fabryk-mcp-* crates into the music-theory server
2. Implement `ToolRegistry` composition
3. Remove extracted tool files from music-theory
4. Verify all 25 MCP tools via MCP inspector

## Pre-Integration Baseline

Capture current tool functionality before changes:

```bash
# List all current tools
# Use MCP inspector or similar to document all 25 tools and their schemas
```

## Implementation Steps

### Step 1: Update music-theory Cargo.toml

```toml
[dependencies]
# Fabryk MCP crates
fabryk-mcp = { path = "../../fabryk-mcp" }
fabryk-mcp-content = { path = "../../fabryk-mcp-content" }
fabryk-mcp-fts = { path = "../../fabryk-mcp-fts" }
fabryk-mcp-graph = { path = "../../fabryk-mcp-graph" }
```

### Step 2: Create composite tool registry

Update `crates/music-theory/mcp-server/src/lib.rs`:

```rust
use fabryk_mcp::{CompositeRegistry, FabrykMcpServer, ToolRegistry};
use fabryk_mcp_content::{ContentTools, SourceTools};
use fabryk_mcp_fts::FtsTools;
use fabryk_mcp_graph::GraphTools;
use std::sync::Arc;

/// Build the complete tool registry for music theory.
pub fn build_tool_registry(
    config: Arc<Config>,
    graph: GraphData,
    search_backend: impl SearchBackend + 'static,
) -> impl ToolRegistry {
    // Content providers
    let content_provider = MusicTheoryContentProvider::new(Arc::clone(&config));
    let source_provider = MusicTheorySourceProvider::new(Arc::clone(&config));

    // Create tool registries
    let content_tools = ContentTools::new(content_provider).with_prefix("concepts");
    let source_tools = SourceTools::new(source_provider);
    let fts_tools = FtsTools::new(search_backend);
    let graph_tools = GraphTools::new(graph);

    // Combine all tools
    CompositeRegistry::new()
        .add(content_tools)    // concepts_list, concepts_get, concepts_categories
        .add(source_tools)     // sources_list, sources_chapters, sources_get_chapter
        .add(fts_tools)        // search, search_suggest, search_index_status
        .add(graph_tools)      // graph_related, graph_path, graph_prerequisites, etc.
}

/// Create and configure the MCP server.
pub async fn create_server(config: Config) -> Result<impl std::future::Future<Output = Result<()>>> {
    let config = Arc::new(config);

    // Load graph
    let graph_path = config.base_path()?.join("data/graphs/graph.json");
    let graph = if graph_path.exists() {
        fabryk_graph::load_graph(&graph_path)?
    } else {
        GraphData::new()
    };

    // Initialize search backend
    let index_path = config.base_path()?.join("data/index");
    let search_backend = TantivySearch::open(&index_path)?;

    // Build registry
    let registry = build_tool_registry(Arc::clone(&config), graph, search_backend);

    // Create server
    let server = FabrykMcpServer::new((*config).clone(), registry)
        .with_name("music-theory")
        .with_description("Music theory knowledge assistant");

    Ok(server.run())
}
```

### Step 3: Update main.rs

```rust
use music_theory_mcp_server::{create_server, Config};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();

    // Load configuration
    let config = Config::load()?;

    // Create and run server
    let server = create_server(config).await?;
    server.await?;

    Ok(())
}
```

### Step 4: Remove extracted tool files

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server/src

# Remove extracted MCP tool files
rm tools/concepts.rs   # Now via ContentTools
rm tools/sources.rs    # Now via SourceTools
rm tools/search.rs     # Now via FtsTools
rm tools/graph.rs      # Now via GraphTools
rm tools/graph_query.rs # Now via GraphTools

# Keep these files (domain-specific or not yet extracted):
# tools/health.rs      - Extracted to fabryk-mcp
# tools/guides.rs      - Could be ContentTools with different prefix
# tools/mod.rs         - Update to just re-export fabryk types
```

### Step 5: Update tools/mod.rs

```rust
//! MCP tools - delegates to fabryk-mcp-* crates.

// Re-export from fabryk-mcp crates
pub use fabryk_mcp::{handle_health, health_tool_info};
pub use fabryk_mcp_content::{ContentTools, SourceTools};
pub use fabryk_mcp_fts::FtsTools;
pub use fabryk_mcp_graph::GraphTools;

// Domain-specific tools that haven't been extracted
// pub mod prompts;  // If there are domain-specific MCP prompts
```

### Step 6: Verify all 25 tools work

Use MCP inspector to test each tool:

```bash
# Start the server
cargo run -p music-theory-mcp-server -- serve

# In MCP inspector, verify each tool:

# Content tools (3)
- concepts_list
- concepts_get
- concepts_categories

# Source tools (3)
- sources_list
- sources_chapters
- sources_get_chapter

# Search tools (3)
- search
- search_suggest
- search_index_status

# Graph tools (8)
- graph_related
- graph_path
- graph_prerequisites
- graph_neighborhood
- graph_info
- graph_validate
- graph_centrality
- graph_bridges

# Other tools (8+)
- health
- guides_list (if kept)
- ... (document all remaining tools)
```

### Step 7: Run full test suite

```bash
cd ~/lab/oxur/ecl

# Test all Fabryk MCP crates
cargo test -p fabryk-mcp
cargo test -p fabryk-mcp-content
cargo test -p fabryk-mcp-fts
cargo test -p fabryk-mcp-graph

# Test music-theory
cargo test -p music-theory-mcp-server

# Clippy all
cargo clippy -p fabryk-mcp -p fabryk-mcp-content -p fabryk-mcp-fts -p fabryk-mcp-graph -p music-theory-mcp-server -- -D warnings
```

## Exit Criteria

- [ ] All four fabryk-mcp-* crates added as dependencies
- [ ] `build_tool_registry()` composes all tool sources
- [ ] Extracted tool files removed from music-theory
- [ ] All 25 MCP tools verified via inspector:
  - [ ] concepts_list, concepts_get, concepts_categories
  - [ ] sources_list, sources_chapters, sources_get_chapter
  - [ ] search, search_suggest, search_index_status
  - [ ] graph_related, graph_path, graph_prerequisites
  - [ ] graph_neighborhood, graph_info, graph_validate
  - [ ] graph_centrality, graph_bridges
  - [ ] health
  - [ ] (other tools)
- [ ] `cargo test --all-features` passes in both repos
- [ ] `cargo clippy` clean in all crates

## Phase 5 Completion

After this milestone, Phase 5 is complete:

| Crate | Tools Provided |
|-------|---------------|
| fabryk-mcp | health, ToolRegistry, CompositeRegistry, FabrykMcpServer |
| fabryk-mcp-content | list/get/categories for any ContentItemProvider |
| fabryk-mcp-fts | search, suggest, index_status |
| fabryk-mcp-graph | related, path, prerequisites, neighborhood, info, validate, centrality, bridges |

**Music-theory now uses Fabryk for all MCP tool infrastructure.**

## Commit Message

```
feat(music-theory): complete MCP integration, Phase 5 done

Wire all fabryk-mcp-* crates into music-theory:
- CompositeRegistry combines content, source, FTS, graph tools
- FabrykMcpServer handles server lifecycle
- Remove extracted tool files from music-theory

Verified: All 25 MCP tools functional via inspector.

Phase 5 complete. MCP infrastructure fully extracted to Fabryk.

Ref: Doc 0013 Phase 5 completion

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
