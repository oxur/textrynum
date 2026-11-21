---
title: "CC Prompt: Fabryk 7.4 — Cleanup & Release"
milestone: "7.4"
phase: 7
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["7.1-7.3 complete"]
governing-docs: [0013-project-plan]
---

# CC Prompt: Fabryk 7.4 — Cleanup & Release

## Context

This is the **final milestone** of the Fabryk extraction project. It handles
final cleanup, release preparation, and tagging `fabryk v0.1.0`.

## Objective

Complete the extraction project:

1. Remove any remaining dead code
2. Run final quality checks
3. Clean up git history (optional)
4. Tag `fabryk v0.1.0`
5. Update ai-music-theory to use tagged version

## Cleanup Steps

### Step 1: Find and remove dead code

```bash
cd ~/lab/oxur/ecl

# Find unused dependencies
cargo +nightly udeps --workspace

# Find unused code
cargo clippy --workspace -- -W dead_code

# Check for TODO/FIXME comments that should be resolved
grep -r "TODO\|FIXME\|XXX" crates/fabryk-* crates/music-theory
```

### Step 2: Final quality checks

```bash
cd ~/lab/oxur/ecl

# Format check
cargo fmt --all -- --check

# Clippy (strict)
cargo clippy --workspace --all-targets -- -D warnings -D clippy::all

# Build in release mode
cargo build --workspace --release

# Run all tests
cargo test --workspace --all-features

# Check documentation builds
cargo doc --workspace --no-deps
```

### Step 3: Verify no music-theory code remains in Fabryk

Check that Fabryk crates don't have music-theory-specific code:

```bash
# Should return nothing
grep -r "music.theory\|concept.card\|tymoczko\|lewin" crates/fabryk-*

# Should return nothing except legitimate uses
grep -r "harmony\|rhythm\|melody" crates/fabryk-*
```

### Step 4: Update version numbers

Update all Fabryk crate versions to 0.1.0:

```bash
# fabryk-core/Cargo.toml
# fabryk-content/Cargo.toml
# etc.

[package]
version = "0.1.0"
```

### Step 5: Create release commit

```bash
cd ~/lab/oxur/ecl

# Stage all changes
git add -A

# Create release commit
git commit -m "release: fabryk v0.1.0

Fabryk v0.1.0 - First release of the knowledge domain framework.

Crates included:
- fabryk-core v0.1.0
- fabryk-content v0.1.0
- fabryk-fts v0.1.0
- fabryk-graph v0.1.0
- fabryk-mcp v0.1.0
- fabryk-mcp-content v0.1.0
- fabryk-mcp-fts v0.1.0
- fabryk-mcp-graph v0.1.0
- fabryk-cli v0.1.0

Extracted from ai-music-theory MCP server (~12,000 lines).
87% of code is now shared infrastructure.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

### Step 6: Tag the release

```bash
cd ~/lab/oxur/ecl

# Create annotated tag
git tag -a fabryk-v0.1.0 -m "Fabryk v0.1.0

First release of the Fabryk knowledge domain framework.

## Highlights

- **fabryk-core**: Error types, utilities, ConfigProvider trait
- **fabryk-content**: Markdown parsing, frontmatter extraction
- **fabryk-fts**: Full-text search with Tantivy backend
- **fabryk-graph**: Knowledge graph with GraphExtractor trait
- **fabryk-mcp**: MCP server infrastructure and tools
- **fabryk-cli**: CLI framework

## Key Abstractions

- GraphExtractor: Domain-specific graph extraction
- ContentItemProvider: Content listing for MCP
- SourceProvider: Source material access
- ToolRegistry: MCP tool registration

## Documentation

See crates/fabryk/README.md for getting started.

## Migration

Extracted from ai-music-theory MCP server.
~87% of code is now reusable infrastructure.
"

# Verify tag
git show fabryk-v0.1.0
```

### Step 7: Update music-theory to use tagged version

Update `crates/music-theory/mcp-server/Cargo.toml`:

```toml
[dependencies]
# Use tagged version (or path for monorepo)
fabryk-core = { path = "../../fabryk-core" }
fabryk-content = { path = "../../fabryk-content" }
fabryk-fts = { path = "../../fabryk-fts" }
fabryk-graph = { path = "../../fabryk-graph" }
fabryk-mcp = { path = "../../fabryk-mcp" }
fabryk-mcp-content = { path = "../../fabryk-mcp-content" }
fabryk-mcp-fts = { path = "../../fabryk-mcp-fts" }
fabryk-mcp-graph = { path = "../../fabryk-mcp-graph" }
fabryk-cli = { path = "../../fabryk-cli" }

# If publishing to crates.io in future:
# fabryk-core = "0.1"
# etc.
```

### Step 8: Final verification

```bash
cd ~/lab/oxur/ecl

# Clean build from scratch
cargo clean
cargo build --workspace --release

# Run all tests
cargo test --workspace --all-features

# Verify music-theory works
cargo run -p music-theory-mcp-server -- --help
cargo run -p music-theory-mcp-server -- health
cargo run -p music-theory-mcp-server -- graph stats
```

### Step 9: Push release

```bash
cd ~/lab/oxur/ecl

# Push commits and tags
git push origin main
git push origin fabryk-v0.1.0
```

## Project Completion Checklist

```markdown
## Fabryk Extraction - Final Checklist

### Code Quality
- [ ] No dead code warnings
- [ ] No clippy warnings
- [ ] cargo fmt clean
- [ ] All tests pass
- [ ] Documentation builds

### Functionality
- [ ] All 25 MCP tools work
- [ ] All CLI commands work
- [ ] Graph output matches baseline
- [ ] Search results match baseline
- [ ] No performance regressions > 5%

### Documentation
- [ ] README for each crate
- [ ] Implementation guides
- [ ] API documentation
- [ ] CHANGELOG

### Release
- [ ] Version numbers set to 0.1.0
- [ ] Release commit created
- [ ] Tag fabryk-v0.1.0 created
- [ ] Tag pushed to remote

### Music Theory
- [ ] Depends on Fabryk crates
- [ ] Domain-specific code only (~2,800 lines)
- [ ] All functionality preserved
```

## Exit Criteria

- [ ] No dead code or unused dependencies
- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy` clean (strict mode)
- [ ] All tests pass
- [ ] No music-theory-specific code in Fabryk
- [ ] All crates at version 0.1.0
- [ ] Release commit created
- [ ] `fabryk-v0.1.0` tag created and pushed
- [ ] music-theory depends on tagged/path version
- [ ] Final verification passes

## Project Complete!

After this milestone:

- **Fabryk v0.1.0** is released
- **ai-music-theory** uses Fabryk infrastructure
- **~87%** of code is shared
- **All 25 MCP tools** functional
- **Documentation** complete

The extraction project is complete. Future domains (math, Rust patterns, etc.)
can now use Fabryk as their foundation.

## Commit Message

```
release: fabryk v0.1.0 - extraction complete

Complete the Fabryk extraction project:
- Remove dead code and unused dependencies
- Final quality checks pass
- Documentation complete
- Tag fabryk-v0.1.0

Project summary:
- 9 Fabryk crates created
- ~87% code extracted from music-theory
- ~2,800 lines remain domain-specific
- All 25 MCP tools functional
- No performance regressions

The Fabryk knowledge domain framework is ready for use.

Phase 7 milestone 7.4 - PROJECT COMPLETE

Ref: Doc 0013 Phase 7

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
