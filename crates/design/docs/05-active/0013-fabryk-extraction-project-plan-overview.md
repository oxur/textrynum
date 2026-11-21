---
number: 13
title: "Fabryk Extraction: Project Plan Overview"
author: "increasing complexity"
component: All
tags: [change-me]
created: 2026-02-03
updated: 2026-02-03
state: Active
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk Extraction: Project Plan Overview

## Overview

The Fabryk extraction project refactors the domain-agnostic infrastructure out
of the music-theory MCP server (~12,000 lines of Rust) into a set of reusable
crates. Post-extraction, ~87% of the code lives in Fabryk as shared
infrastructure; ~2,800 lines remain in ai-music-theory as domain-specific
implementations.

The extraction proceeds in seven phases, ordered by increasing complexity and
dependency depth. Each phase is decomposed into milestones — discrete units of
work, each suitable for a single Claude Code session. Milestones within a phase
are sequential; some early milestones across adjacent phases may overlap.

**Governing documents:**

- Doc 0009 — Fabryk Extraction Audit (file inventory, classifications, trait designs)
- Doc 0010 — Audit Amendment (six refinements that override parts of 0009)
- Doc 0011 — Session Bootstrap (cold-start prompt for new sessions)

**Key constraint:** The music-theory MCP server must remain fully functional
after every milestone. All 25 MCP tools must work. `cargo test --all-features`
must pass in both repos at every merge point.

---

## Phase 1: fabryk-core (1–2 weeks)

**Goal:** Create the Fabryk workspace and extract shared types, traits, errors,
and utilities. This is the foundation that every other crate depends on.

**Crate:** `fabryk-core`
**Dependency level:** 0 (no internal Fabryk dependencies)
**Risk:** Low

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 1.1 | Workspace scaffold | Create Fabryk repo, workspace Cargo.toml with all 10 crate stubs, CI config, LICENSE, README skeleton | Workspace compiles (`cargo check`), all stubs present | §6.1, §7 Phase 1 |
| 1.2 | Error types & Result | Extract `error.rs` to `fabryk-core`. Define `fabryk_core::Error` and `fabryk_core::Result<T>`. No changes needed — file is already generic | `fabryk-core` compiles, error types exported | §4.1 |
| 1.3 | File utilities | Extract `util/files.rs` to `fabryk-core::util::files`. Functions: `find_file_by_id()`, `find_all_files()`, `FindOptions`. Already generic | All file utility tests pass in fabryk-core | §4.1 |
| 1.4 | Path utilities | Extract `util/paths.rs` to `fabryk-core::util::paths`. Parameterise: replace `MUSIC_THEORY_*` env var prefix with configurable project name. Add `compute_id()` utility (Amendment §2f-i) | Path tests pass with parameterised env vars | §4.1, Amend §2f-i |
| 1.5 | ConfigProvider trait | Define `ConfigProvider` trait in `fabryk-core::traits`. Methods: `base_path()`, `content_path()`, `project_name()`. Extract `state.rs` as `AppState<C: ConfigProvider>` | Trait defined, AppState generic, compiles | §4.1 |
| 1.6 | Resource serving | Extract `resources/mod.rs` to `fabryk-core::resources` | Resource serving tests pass | §4.1 |
| 1.7 | Music theory integration | Add `fabryk-core` dependency to ai-music-theory. Update all imports from local modules to `fabryk_core::`. Implement `ConfigProvider` for music theory `Config`. Remove extracted files from music-theory repo | `cargo test --all-features` passes in both repos | §7 Phase 2 |

**Phase 1 exit criteria:**

- `fabryk-core` compiles and all tests pass
- ai-music-theory depends on `fabryk-core` for errors, utilities, paths, state
- No duplicate code between repos for extracted files
- `cargo clippy` clean in both repos

---

## Phase 2: fabryk-content (1 week)

**Goal:** Extract markdown parsing, frontmatter extraction, and content helper
utilities.

**Crate:** `fabryk-content`
**Dependency level:** 1 (depends on `fabryk-core`)
**Risk:** Low

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 2.1 | Frontmatter extraction | Extract `markdown/frontmatter.rs` to `fabryk-content::markdown`. Functions: `extract_frontmatter()`, `FrontmatterData`. Already fully generic | Frontmatter tests pass in fabryk-content | §4.2 |
| 2.2 | Markdown parser | Extract `markdown/parser.rs` to `fabryk-content::markdown`. Functions: `parse_markdown()`, `extract_first_heading()`. Already fully generic | Parser tests pass in fabryk-content | §4.2 |
| 2.3 | Content helpers | Create `fabryk-content::markdown::helpers`. Implement `extract_list_from_section()` (Amendment §2f-ii). Extract any other generic markdown utilities from metadata/extraction.rs | Helper function tests pass | Amend §2f-ii |
| 2.4 | Music theory integration | Add `fabryk-content` dependency to ai-music-theory. Update imports. Move domain-specific metadata logic (field names, ConceptMetadata type) to music theory's own modules. Remove extracted markdown files from music-theory repo | `cargo test --all-features` passes in both repos | §4.2 |

**Phase 2 exit criteria:**

- `fabryk-content` compiles and all tests pass
- ai-music-theory uses `fabryk-content` for all markdown/frontmatter operations
- Domain-specific metadata types remain in ai-music-theory
- No `MetadataExtractor` trait (deferred per Amendment §2c)

---

## Phase 3: fabryk-fts (2 weeks)

**Goal:** Extract the full-text search subsystem with a default schema.

**Crate:** `fabryk-fts`
**Dependency level:** 1 (depends on `fabryk-core`)
**Risk:** Low–Medium

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 3.1 | SearchBackend trait & default schema | Extract `search/backend.rs` (SearchBackend trait) and `search/schema.rs`. Use default schema with generic field names — no SearchSchemaProvider trait (Amendment §2d). Rename any music-theory-specific field names to generic equivalents | Backend trait and schema compile | §4.3, Amend §2d |
| 3.2 | Search document & query | Extract `search/document.rs` and `search/query.rs`. Ensure document field access uses the default schema field names | Document and query tests pass | §4.3 |
| 3.3 | Tantivy backend | Extract `search/tantivy_search.rs`, `search/indexer.rs`, `search/builder.rs`. Feature-gate behind `tantivy` feature flag | Tantivy search tests pass with feature flag | §4.3 |
| 3.4 | Supporting modules | Extract `search/simple_search.rs`, `search/stopwords.rs`, `search/freshness.rs` | All search module tests pass | §4.3 |
| 3.5 | Music theory integration | Add `fabryk-fts` dependency to ai-music-theory. Update imports. Remove extracted search files. Verify search results are identical (run search QA integration tests) | `cargo test --all-features` passes, search QA green | §4.3 |

**Phase 3 exit criteria:**

- `fabryk-fts` compiles and all tests pass (with `tantivy` feature)
- ai-music-theory uses `fabryk-fts` for all search operations
- Search results identical to pre-extraction (verified by integration tests)
- No `SearchSchemaProvider` trait (deferred per Amendment §2d)

---

## v0.1-alpha Checkpoint

**Gate criteria (all must pass before proceeding to Phase 4):**

1. `fabryk-core`, `fabryk-content`, `fabryk-fts` compile and pass all tests
2. ai-music-theory depends on these three crates
3. All non-graph MCP tools work identically (search, content listing, guides)
4. Graph tools still work via in-repo code (unchanged)
5. `cargo test --all-features` passes in both repos
6. `cargo clippy --all-features` clean in both repos
7. Performance: no measurable regression in search latency or startup time

**Decision point:** If the extraction pattern has revealed problems, address
them before proceeding to Phase 4 (highest risk). If clean, continue.

---

## Phase 4: fabryk-graph (3 weeks)

**Goal:** Extract the graph database with the `GraphExtractor` trait abstraction.
This is the highest-risk, highest-value phase.

**Crate:** `fabryk-graph`
**Dependency level:** 2 (depends on `fabryk-core`, `fabryk-content`)
**Risk:** High

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 4.1 | Graph types & Relationship enum | Extract `graph/types.rs` to `fabryk-graph`. Implement `Relationship` enum with `Custom(String)` variant (Amendment §2b). Types: `Node`, `Edge`, `GraphData`, `EdgeOrigin`. Add `Relationship::default_weight()` | Types compile, Relationship enum complete | §4.4, Amend §2b |
| 4.2 | GraphExtractor trait | Define `GraphExtractor` trait in `fabryk-graph::extractor`. Associated types: `NodeData`, `EdgeData`. Methods: `extract_node()`, `extract_edges()`, `to_graph_node()`, `to_graph_edges()` | Trait defined, mock extractor compiles | §3, §9 |
| 4.3 | Graph algorithms | Extract `graph/algorithms.rs` to `fabryk-graph`. Functions: `neighborhood()`, `shortest_path()`, `prerequisites_sorted()`, centrality. Already fully generic | All algorithm tests pass | §4.4 |
| 4.4 | Graph persistence | Extract `graph/persistence.rs` to `fabryk-graph`. Functions: `save_graph()`, `load_graph()`, `to_petgraph()`. Feature-gate rkyv cache behind `rkyv-cache` feature. Already fully generic | Persistence tests pass with feature flag | §4.4 |
| 4.5 | Graph builder | Extract `graph/builder.rs` to `fabryk-graph`. Refactor to `GraphBuilder<E: GraphExtractor>`. Add `with_manual_edges()` support (Amendment §2f-iii). Builder uses extractor trait instead of `parse_concept_card()` | Builder compiles with generic extractor | §4.4, Amend §2f-iii |
| 4.6 | Query, stats, validation | Extract `graph/query.rs`, `graph/stats.rs`, `graph/validation.rs`, `graph/loader.rs` to `fabryk-graph`. All generic or lightly parameterised | All supporting module tests pass | §4.4 |
| 4.7 | MusicTheoryExtractor | Create `music_theory_extractor.rs` in ai-music-theory. Implement `GraphExtractor` for music theory. Move `ConceptCard`, `RelatedConcepts`, and all parsing logic from `graph/parser.rs` here | Extractor compiles, matches current parser output | §3, §9.1 |
| 4.8 | Graph integration | Wire `MusicTheoryExtractor` into ai-music-theory's graph build pipeline. Remove `graph/parser.rs` from music-theory. Run `graph build --dry-run` and verify identical node/edge counts. Run `graph stats` and compare | `cargo test --all-features`, graph output identical | §7 Phase 5 |

**Phase 4 exit criteria:**

- `fabryk-graph` compiles and all tests pass
- `GraphExtractor` trait is defined and documented
- `MusicTheoryExtractor` produces identical graph output to pre-extraction
- Graph build: same node count, same edge count, same statistics
- All 15 graph MCP tools work identically
- `cargo test --all-features` passes in both repos

---

## Phase 5: fabryk-mcp + fabryk-mcp-* (1–2 weeks)

**Goal:** Extract MCP server infrastructure and all generic MCP tools.

**Crates:** `fabryk-mcp`, `fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph`
**Dependency level:** 2–3
**Risk:** Medium

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 5.1 | MCP core infrastructure | Extract `server.rs` to `fabryk-mcp`. Parameterise: `FabrykMcpServer<C: ConfigProvider>`, configurable server name. Define `ToolRegistry` trait. Extract `tools/health.rs` | MCP server starts with mock registry | §4.6 |
| 5.2 | Content & source traits | Define `ContentItemProvider` and `SourceProvider` traits in `fabryk-mcp-content` (Amendment §2a). Extract `tools/guides.rs` → `documents.rs`. Generalise guide tools to document tools | Traits defined, document tools compile | Amend §2a, §4.7 |
| 5.3 | Music theory content providers | Implement `ContentItemProvider` for music theory concepts (from `tools/concepts.rs` logic). Implement `SourceProvider` for music theory sources (from `tools/sources.rs` logic). Wire into tool registration | Concept and source tools work via traits | Amend §2a |
| 5.4 | FTS MCP tools | Extract `tools/search.rs` to `fabryk-mcp-fts`. Parameterise response types to use default schema fields | Search tool works via fabryk-mcp-fts | §4.8 |
| 5.5 | Graph MCP tools | Extract `tools/graph.rs` and `tools/graph_query.rs` to `fabryk-mcp-graph`. Parameterise over generic node/edge types | All 15 graph tools work via fabryk-mcp-graph | §4.9 |
| 5.6 | Full MCP integration | Wire all fabryk-mcp-* crates into ai-music-theory. Implement `ToolRegistry` for music theory. Remove extracted tool files. Verify all 25 tools via MCP inspector | All 25 MCP tools functional, inspector confirms | §7 Phase 6 |

**Phase 5 exit criteria:**

- All four MCP crates compile and pass tests
- ai-music-theory registers tools via `ToolRegistry` trait
- MCP inspector shows all 25 tools with correct schemas
- `cargo test --all-features` passes in both repos

---

## Phase 6: fabryk-cli (1 week)

**Goal:** Extract the CLI framework.

**Crate:** `fabryk-cli`
**Dependency level:** 3
**Risk:** Low

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 6.1 | CLI framework | Extract `cli.rs`, `main.rs`, `lib.rs` to `fabryk-cli`. Parameterise: `FabrykCli<C: ConfigProvider>`, configurable project name and commands. Support domain-specific subcommand registration | CLI framework compiles with mock config | §4.10 |
| 6.2 | Graph CLI commands | Extract `graph/cli.rs` to `fabryk-cli`. Parameterise: `handle_build<E: GraphExtractor>()`, `handle_validate()`. Pass extractor instance from domain code | Graph CLI commands work via fabryk-cli | §4.10 |
| 6.3 | Music theory CLI integration | Update ai-music-theory `main.rs` to use `fabryk-cli`. Register music-theory-specific subcommands. Remove extracted CLI files | `cargo run -- --help` shows correct commands | §7 Phase 7 |

**Phase 6 exit criteria:**

- `fabryk-cli` compiles and passes tests
- ai-music-theory uses `fabryk-cli` for all CLI operations
- All CLI commands work: `serve`, `index`, `graph build`, `graph validate`, etc.
- `cargo test --all-features` passes in both repos

---

## Phase 7: Integration & Documentation (1 week)

**Goal:** Final cleanup, documentation, performance verification, and release.

**Risk:** Low

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 7.1 | Comprehensive testing | Run full test suites in both repos. Run all integration tests. Verify graph build output matches pre-extraction snapshot. Run search QA tests | All tests green, output verified | §7 Phase 8 |
| 7.2 | Performance benchmarking | Benchmark startup time, graph build time, search latency, index build time. Compare against pre-extraction baselines. Flag any regressions > 5% | Benchmark report, no regressions | §7 Phase 8 |
| 7.3 | Documentation | Write README for each Fabryk crate. Write top-level Fabryk README with architecture overview. Document `GraphExtractor` trait with implementation guide. Document `ContentItemProvider` and `SourceProvider` traits | All crate READMEs present, trait guides complete | §8.4 |
| 7.4 | Cleanup & release | Remove any remaining dead code. Run `cargo clippy`, `cargo fmt --check`. Squash/clean commit history. Merge feature branches. Tag `fabryk v0.1.0`. Update ai-music-theory to use tagged version | `v0.1.0` tagged, ai-music-theory on release dep | §7 Phase 8 |

**Phase 7 exit criteria (project completion):**

- `fabryk v0.1.0` tagged and pushed
- ai-music-theory depends on tagged Fabryk release
- All 25 MCP tools functional
- All tests passing
- No performance regressions
- Documentation complete
- Clean git history in both repos

---

## Summary

| Phase | Crate(s) | Milestones | Risk | Estimated Duration |
|-------|----------|------------|------|--------------------|
| 1 | fabryk-core | 1.1 – 1.7 | Low | 1–2 weeks |
| 2 | fabryk-content | 2.1 – 2.4 | Low | 1 week |
| 3 | fabryk-fts | 3.1 – 3.5 | Low–Med | 2 weeks |
| — | *v0.1-alpha checkpoint* | — | — | — |
| 4 | fabryk-graph | 4.1 – 4.8 | **High** | 3 weeks |
| 5 | fabryk-mcp, fabryk-mcp-* | 5.1 – 5.6 | Medium | 1–2 weeks |
| 6 | fabryk-cli | 6.1 – 6.3 | Low | 1 week |
| 7 | Integration & docs | 7.1 – 7.4 | Low | 1 week |

**Total milestones:** 37
**Total estimated duration:** 10–14 weeks (side project pace)
**v0.1-alpha checkpoint:** ~4 weeks (after Phase 3)

---

## Appendix: Milestone → CC Prompt Index

Each milestone will have a corresponding Claude Code prompt document. Prompts
are created as needed, not all upfront. The naming convention is:

```
cc-fabryk-{phase}.{milestone}-{short-name}.md
```

Examples:

```
cc-fabryk-1.1-workspace-scaffold.md
cc-fabryk-1.5-config-provider-trait.md
cc-fabryk-4.2-graph-extractor-trait.md
cc-fabryk-4.7-music-theory-extractor.md
cc-fabryk-5.2-content-source-traits.md
```

The bootstrap document (Doc 0011) should be updated with current status before
creating each prompt. The prompt itself should reference this overview document
for context on what came before and what comes after the current milestone.
