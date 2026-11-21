---
title: "CC Prompt: Fabryk 4.8 — Graph Integration"
milestone: "4.8"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["4.1-4.7 complete"]
governing-docs: [0011-audit §7, 0013-project-plan]
---

# CC Prompt: Fabryk 4.8 — Graph Integration

## Context

This is the **final milestone of Phase 4** — the highest-risk phase of the
Fabryk extraction. This milestone wires everything together:

1. Update music-theory to use fabryk-graph
2. Wire MusicTheoryExtractor into the graph build pipeline
3. Remove the old graph/parser.rs from music-theory
4. Verify graph output is identical to pre-extraction

**Success criteria:** The graph built with the new infrastructure must have
identical node count, edge count, and statistics to the pre-extraction graph.

## Objective

Complete the fabryk-graph integration in ai-music-theory:

1. Add fabryk-graph dependency to music-theory
2. Update graph build command to use GraphBuilder<MusicTheoryExtractor>
3. Update all graph imports to use fabryk-graph types
4. Remove extracted files from music-theory
5. Run verification tests

## Pre-Integration Baseline

**CRITICAL:** Before making any changes, capture the current graph state:

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server

# Build current graph
cargo run -- graph build --output /tmp/baseline-graph.json

# Capture statistics
cargo run -- graph stats > /tmp/baseline-stats.txt

# Record counts
echo "Baseline node count: $(jq '.nodes | length' /tmp/baseline-graph.json)"
echo "Baseline edge count: $(jq '.edges | length' /tmp/baseline-graph.json)"
```

Save these values — they are your success criteria.

## Implementation Steps

### Step 1: Update Cargo.toml

Add fabryk-graph dependency to `crates/music-theory/mcp-server/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...

# Fabryk crates
fabryk-core = { path = "../../fabryk-core" }
fabryk-content = { path = "../../fabryk-content" }
fabryk-fts = { path = "../../fabryk-fts" }
fabryk-graph = { path = "../../fabryk-graph" }
```

### Step 2: Update graph build command

Update `crates/music-theory/mcp-server/src/graph/cli.rs` (or equivalent):

```rust
use crate::music_theory_extractor::MusicTheoryExtractor;
use fabryk_graph::{GraphBuilder, save_graph, GraphMetadata};

pub async fn handle_build(config: &Config, output: Option<PathBuf>) -> Result<()> {
    let content_path = config.content_path("concepts")?;
    let manual_edges_path = config.base_path()?.join("data/graphs/manual_edges.json");

    let extractor = MusicTheoryExtractor::new();

    let mut builder = GraphBuilder::new(extractor)
        .with_content_path(&content_path);

    // Add manual edges if file exists
    if manual_edges_path.exists() {
        builder = builder.with_manual_edges(&manual_edges_path);
    }

    println!("Building graph from {}...", content_path.display());

    let result = builder.build().await?;

    println!(
        "Built graph: {} nodes, {} edges ({} manual)",
        result.graph.node_count(),
        result.graph.edge_count(),
        result.manual_edges_loaded
    );

    if !result.errors.is_empty() {
        println!("Warnings during build:");
        for error in &result.errors {
            println!("  - {}: {}", error.file.display(), error.message);
        }
    }

    // Save graph
    let output_path = output.unwrap_or_else(|| config.base_path().unwrap().join("data/graphs/graph.json"));

    let metadata = GraphMetadata {
        source_file_count: Some(result.files_processed),
        ..Default::default()
    };

    save_graph(&result.graph, &output_path, Some(metadata))?;
    println!("Saved to {}", output_path.display());

    Ok(())
}
```

### Step 3: Update graph module imports

Update `crates/music-theory/mcp-server/src/graph/mod.rs`:

```rust
//! Graph module - re-exports from fabryk-graph plus domain-specific code.

// Re-export fabryk-graph types
pub use fabryk_graph::{
    // Types
    Edge, EdgeOrigin, GraphData, Node, Relationship,
    // Builder
    GraphBuilder, BuildResult, ErrorHandling,
    // Algorithms
    neighborhood, shortest_path, prerequisites_sorted,
    calculate_centrality, find_bridges, get_related,
    NeighborhoodResult, PathResult, PrerequisitesResult,
    // Persistence
    save_graph, load_graph, is_cache_fresh, GraphMetadata,
    // Query types
    NodeSummary, PathResponse, NeighborhoodResponse, PrerequisitesResponse,
    // Stats & validation
    compute_stats, validate_graph, GraphStats, ValidationResult,
};

// Domain-specific
pub use crate::music_theory_extractor::{
    MusicTheoryExtractor, MusicTheoryNodeData, MusicTheoryEdgeData,
};

// CLI commands (local)
pub mod cli;
```

### Step 4: Update MCP graph tools

Update `crates/music-theory/mcp-server/src/tools/graph.rs` to use fabryk-graph types:

```rust
use fabryk_graph::{
    neighborhood, shortest_path, prerequisites_sorted,
    NodeSummary, PathResponse, NeighborhoodResponse, PrerequisitesResponse,
};

// Update tool implementations to use the new types
// Most of the logic should work unchanged since we designed
// the fabryk-graph types to match the existing interfaces
```

### Step 5: Update graph query tools

Similarly update `crates/music-theory/mcp-server/src/tools/graph_query.rs`:

```rust
use fabryk_graph::{get_related, calculate_centrality, find_bridges};
```

### Step 6: Remove extracted files

Once everything compiles and tests pass, remove the files that were extracted:

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server/src

# Remove extracted graph modules (keep only cli.rs and mod.rs)
rm graph/types.rs        # Extracted to fabryk-graph
rm graph/algorithms.rs   # Extracted to fabryk-graph
rm graph/persistence.rs  # Extracted to fabryk-graph
rm graph/builder.rs      # Extracted to fabryk-graph
rm graph/query.rs        # Extracted to fabryk-graph
rm graph/stats.rs        # Extracted to fabryk-graph
rm graph/validation.rs   # Extracted to fabryk-graph
rm graph/loader.rs       # Extracted to fabryk-graph
rm graph/parser.rs       # Replaced by MusicTheoryExtractor
```

**Keep these files:**
- `graph/mod.rs` - Updated module that re-exports from fabryk-graph
- `graph/cli.rs` - CLI command handlers (will be extracted in Phase 6)

### Step 7: Verify graph output

**CRITICAL VERIFICATION:**

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server

# Build new graph
cargo run -- graph build --output /tmp/new-graph.json

# Compare with baseline
echo "Old node count: $(jq '.nodes | length' /tmp/baseline-graph.json)"
echo "New node count: $(jq '.nodes | length' /tmp/new-graph.json)"

echo "Old edge count: $(jq '.edges | length' /tmp/baseline-graph.json)"
echo "New edge count: $(jq '.edges | length' /tmp/new-graph.json)"

# Run stats
cargo run -- graph stats > /tmp/new-stats.txt
diff /tmp/baseline-stats.txt /tmp/new-stats.txt
```

**Expected:** Node count and edge count should be **identical**.

If counts differ, investigate:
1. Check if MusicTheoryExtractor parses all the same files
2. Check if relationship extraction matches the old parser
3. Check if manual edges are being loaded

### Step 8: Run full test suite

```bash
cd ~/lab/oxur/ecl

# Test fabryk-graph
cargo test -p fabryk-graph

# Test music-theory
cargo test -p music-theory-mcp-server

# Clippy both
cargo clippy -p fabryk-graph -- -D warnings
cargo clippy -p music-theory-mcp-server -- -D warnings
```

### Step 9: Test MCP tools

Use the MCP inspector to verify all 15 graph tools work:

```bash
# Start server
cargo run -p music-theory-mcp-server -- serve

# In another terminal, use MCP inspector or test client
# Verify these tools work:
# - graph_info
# - graph_related
# - graph_path
# - graph_prerequisites
# - graph_neighborhood
# - graph_stats
# - graph_validate
# - graph_centrality
# - graph_bridges
# - graph_query
# ... and other graph tools
```

### Step 10: Create dry-run comparison

For extra confidence, add a dry-run comparison mode:

```bash
# Compare full graph structure
cargo run -p music-theory-mcp-server -- graph build --dry-run

# Should output:
# Dry run complete:
# - 342 nodes would be created
# - 1,247 edges would be created (123 manual)
# - This matches the baseline
```

## Troubleshooting

### Node count mismatch

1. Check glob pattern in MusicTheoryExtractor (should be `**/*.md`)
2. Verify base path is correct
3. Check if any files are being skipped due to parse errors

### Edge count mismatch

1. Compare relationship extraction logic
2. Check if `extract_from_body` is enabled
3. Verify manual_edges.json is being loaded

### Tool failures

1. Check import paths
2. Verify response types match expected MCP schemas
3. Check if GraphData methods match old interface

## Exit Criteria

- [ ] fabryk-graph dependency added to music-theory
- [ ] GraphBuilder<MusicTheoryExtractor> used for graph build
- [ ] All graph imports updated to use fabryk-graph
- [ ] Old graph files removed from music-theory
- [ ] Node count matches baseline exactly
- [ ] Edge count matches baseline exactly
- [ ] `cargo test -p fabryk-graph` passes
- [ ] `cargo test -p music-theory-mcp-server` passes
- [ ] `cargo clippy` clean in both crates
- [ ] All 15 graph MCP tools work identically
- [ ] Graph statistics match baseline

## Phase 4 Completion

After this milestone, Phase 4 is complete:

| Milestone | Status | Deliverable |
|-----------|--------|-------------|
| 4.1 | ✓ | Graph types, Relationship enum |
| 4.2 | ✓ | GraphExtractor trait |
| 4.3 | ✓ | Graph algorithms |
| 4.4 | ✓ | Graph persistence |
| 4.5 | ✓ | GraphBuilder |
| 4.6 | ✓ | Query, stats, validation |
| 4.7 | ✓ | MusicTheoryExtractor |
| 4.8 | ✓ | Integration complete |

**fabryk-graph is now the source of truth for graph infrastructure.**

## Next Steps

Proceed to Phase 5: fabryk-mcp

- 5.1: MCP core infrastructure
- 5.2: Content & source traits
- 5.3: Music theory content providers
- 5.4: FTS MCP tools
- 5.5: Graph MCP tools
- 5.6: Full MCP integration

## Commit Message

```
feat(music-theory): integrate fabryk-graph, complete Phase 4

Wire fabryk-graph into music-theory MCP server:
- Add fabryk-graph dependency
- Update graph build to use GraphBuilder<MusicTheoryExtractor>
- Update all imports to use fabryk-graph types
- Remove extracted graph modules from music-theory

Verification:
- Node count: [X] (matches baseline)
- Edge count: [X] (matches baseline)
- All 15 graph MCP tools functional
- Graph statistics identical

Phase 4 complete. fabryk-graph is now the canonical graph
infrastructure for the Fabryk ecosystem.

Ref: Doc 0013 Phase 4 completion
Ref: Doc 0011 §7 (music theory integration)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
