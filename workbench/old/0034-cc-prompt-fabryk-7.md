---
title: "CC Prompt: Fabryk 7.1 — Comprehensive Testing"
milestone: "7.1"
phase: 7
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["Phase 6 complete"]
governing-docs: [0011-audit §7, 0013-project-plan]
---

# CC Prompt: Fabryk 7.1 — Comprehensive Testing

## Context

Phase 7 is the final integration and documentation phase. This milestone runs
comprehensive tests across both repositories to verify the extraction was
successful.

## Objective

Run full test suites and verify:

1. All Fabryk crates pass tests
2. ai-music-theory passes tests
3. Graph output matches pre-extraction baseline
4. Search results match pre-extraction baseline
5. All MCP tools functional

## Testing Steps

### Step 1: Test all Fabryk crates

```bash
cd ~/lab/oxur/ecl

# Test each crate individually
cargo test -p fabryk-core
cargo test -p fabryk-content
cargo test -p fabryk-fts
cargo test -p fabryk-graph
cargo test -p fabryk-mcp
cargo test -p fabryk-mcp-content
cargo test -p fabryk-mcp-fts
cargo test -p fabryk-mcp-graph
cargo test -p fabryk-cli

# Test with all features
cargo test -p fabryk-fts --features fts-tantivy
cargo test -p fabryk-graph --features rkyv-cache

# Test entire workspace
cargo test --workspace
```

### Step 2: Test music-theory

```bash
cd ~/lab/oxur/ecl

cargo test -p music-theory-mcp-server
cargo test -p music-theory-mcp-server --all-features
```

### Step 3: Run Clippy on all crates

```bash
cd ~/lab/oxur/ecl

# Clippy all Fabryk crates
cargo clippy -p fabryk-core -p fabryk-content -p fabryk-fts -p fabryk-graph \
  -p fabryk-mcp -p fabryk-mcp-content -p fabryk-mcp-fts -p fabryk-mcp-graph \
  -p fabryk-cli -- -D warnings

# Clippy music-theory
cargo clippy -p music-theory-mcp-server -- -D warnings

# Clippy entire workspace
cargo clippy --workspace -- -D warnings
```

### Step 4: Verify graph output

Compare against pre-extraction baseline:

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server

# Build current graph
cargo run -- graph build --output /tmp/current-graph.json

# Compare with baseline (captured before extraction started)
echo "Baseline nodes: $(jq '.nodes | length' /tmp/baseline-graph.json)"
echo "Current nodes: $(jq '.nodes | length' /tmp/current-graph.json)"

echo "Baseline edges: $(jq '.edges | length' /tmp/baseline-graph.json)"
echo "Current edges: $(jq '.edges | length' /tmp/current-graph.json)"

# Detailed comparison
cargo run -- graph stats > /tmp/current-stats.txt
diff /tmp/baseline-stats.txt /tmp/current-stats.txt
```

**Expected:** Node count and edge count must match exactly.

### Step 5: Verify search results

```bash
# Run search QA tests
cargo test -p music-theory-mcp-server search_qa

# Manual verification of key searches
cargo run -p music-theory-mcp-server -- search "picardy third"
cargo run -p music-theory-mcp-server -- search "dominant seventh"
cargo run -p music-theory-mcp-server -- search "neo-riemannian"
```

### Step 6: MCP tool verification

Start server and verify all 25 tools via inspector:

```bash
# Terminal 1: Start server
cargo run -p music-theory-mcp-server -- serve

# Terminal 2: Run MCP inspector tests
# Verify each tool returns expected response format

# Automated tool check (if available)
cargo test -p music-theory-mcp-server mcp_tools
```

### Step 7: Integration test checklist

Create a verification checklist:

```markdown
## Test Results Checklist

### Fabryk Crates
- [ ] fabryk-core: tests pass
- [ ] fabryk-content: tests pass
- [ ] fabryk-fts: tests pass (with fts-tantivy)
- [ ] fabryk-graph: tests pass (with rkyv-cache)
- [ ] fabryk-mcp: tests pass
- [ ] fabryk-mcp-content: tests pass
- [ ] fabryk-mcp-fts: tests pass
- [ ] fabryk-mcp-graph: tests pass
- [ ] fabryk-cli: tests pass

### Music Theory
- [ ] music-theory-mcp-server: tests pass
- [ ] Graph node count matches baseline
- [ ] Graph edge count matches baseline
- [ ] Graph stats match baseline
- [ ] Search results quality unchanged

### MCP Tools (all 25)
- [ ] health
- [ ] concepts_list
- [ ] concepts_get
- [ ] concepts_categories
- [ ] sources_list
- [ ] sources_chapters
- [ ] sources_get_chapter
- [ ] search
- [ ] search_suggest
- [ ] search_index_status
- [ ] graph_related
- [ ] graph_path
- [ ] graph_prerequisites
- [ ] graph_neighborhood
- [ ] graph_info
- [ ] graph_validate
- [ ] graph_centrality
- [ ] graph_bridges
- [ ] (7 more tools)

### CLI Commands
- [ ] version
- [ ] health
- [ ] serve
- [ ] index
- [ ] graph build
- [ ] graph validate
- [ ] graph stats
- [ ] graph query

### Code Quality
- [ ] cargo clippy --workspace: clean
- [ ] cargo fmt --check: clean
- [ ] No compiler warnings
```

## Exit Criteria

- [ ] All Fabryk crate tests pass
- [ ] Music-theory tests pass
- [ ] Graph output identical to baseline
- [ ] Search QA tests pass
- [ ] All 25 MCP tools verified
- [ ] All CLI commands verified
- [ ] `cargo clippy --workspace` clean
- [ ] No compiler warnings

## Commit Message

```
test: comprehensive testing for Fabryk extraction

Run full test suite across both repositories:
- All 9 Fabryk crates pass tests
- music-theory-mcp-server passes tests
- Graph output matches pre-extraction baseline
- All 25 MCP tools verified
- All CLI commands verified
- Clippy clean, no warnings

Phase 7 milestone 7.1 of Fabryk extraction.

Ref: Doc 0013 Phase 7

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
