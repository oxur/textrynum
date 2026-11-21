# Complete the Fabryk Extraction — Implementation Plan

## Context

The `ai-music-theory` project has a production MCP server (25+ tools, 360+ tests) built on the `fabryk` framework. The `ai-maths` project needs an equivalent server for higher mathematics. Rather than fork-and-diverge, this plan extracts all remaining generic tool patterns from the music server into fabryk, making both domain servers thin shells (~300 lines) over shared infrastructure.

**Repos involved:**

- `~/lab/oxur/ecl/crates/fabryk*` — the domain-agnostic framework
- `~/lab/music-comp/ai-music-theory/mcp-server/` — music theory server (refactor target)

**Hard constraint:** The music server's MCP tool API (tool names, argument schemas) must NOT change — Claude Desktop depends on these.

---

## Phase Overview

| Phase | Goal | Where | Milestones |
|-------|------|-------|------------|
| **1** | Extend fabryk content/source tools | ecl repo | 3 |
| **2** | Extend fabryk FTS + add guide tools | ecl repo | 3 |
| **3** | Extend fabryk graph tools (biggest piece) | ecl repo | 5 |
| **4** | Migrate music server to fabryk tools | music repo | 4 |
| **5** | Thin shell cleanup + template docs | both repos | 2 |

Phases 1-3 modify only fabryk (ecl repo) and can proceed in parallel. Phase 4 depends on all of 1-3. Phase 5 depends on 4.

---

## Phase 1: Extend Fabryk Content and Source Tools

**Goal:** Add missing filter capabilities and extra tool slots to `fabryk-mcp-content` so the music server's `ConceptToolsRegistry` (3 tools) and `SourceToolsRegistry` (5 tools) can be fully replaced.

### Milestone 1.1: Add Extended Filters to ContentItemProvider

The music server's `list_concepts` accepts `tier`, `subcategory`, `source` beyond `category`/`limit`. Add a filter-extension mechanism to the trait without making it domain-specific.

**Files to modify:**

- `ecl/crates/fabryk-mcp-content/src/traits.rs` — add `FilterMap` type alias + `list_items_filtered()` default method
- `ecl/crates/fabryk-mcp-content/src/tools.rs` — add `with_extra_list_schema()` to `ContentTools<P>`, update `call()` to pass extra fields as `FilterMap`

**Key design:**

```rust
pub type FilterMap = serde_json::Map<String, serde_json::Value>;

// New default method on ContentItemProvider:
async fn list_items_filtered(
    &self,
    category: Option<&str>,
    limit: Option<usize>,
    _extra_filters: &FilterMap,
) -> Result<Vec<Self::ItemSummary>> {
    self.list_items(category, limit).await  // default ignores extras
}
```

**Scope:** ~80 lines changed, ~30 lines tests
**Verify:** `cargo test -p fabryk-mcp-content`

### Milestone 1.2: Add Extra Source Tool Slots to SourceTools

`SourceProvider` already has `is_available()` and `get_source_path()` but `SourceTools` doesn't expose them as MCP tools. Add `SLOT_CHECK_AVAILABILITY` and `SLOT_GET_PATH`.

**Files to modify:**

- `ecl/crates/fabryk-mcp-content/src/tools.rs` — add 2 new slots + arg types + handlers to `SourceTools<P>`

**Scope:** ~100 lines new, ~40 lines tests
**Verify:** `cargo test -p fabryk-mcp-content`

### Milestone 1.3: Implement SourceProvider for Music Server

Create `MusicTheorySourceProvider` implementing `SourceProvider`, delegating to existing `tools::sources::*` functions.

**Files to modify:**

- `music-server/crates/server/src/content.rs` — add `MusicTheorySourceProvider` struct + impl

**Scope:** ~120 lines new
**Verify:** `cargo test -p server`

---

## Phase 2: Extend Fabryk FTS and Add Guide Tools

**Goal:** Enrich search tools with domain-filter extension points and create a generic "document collection" pattern for guides.

### Milestone 2.1: Add Extra Filter Support to FtsTools

Add `extra_filters: Option<Value>` to `SearchParams` and `with_extra_search_schema()` to `FtsTools`. Domain servers can inject schema properties (tier, subcategory, min_confidence) that pass through to their `SearchBackend` implementation.

**Files to modify:**

- `ecl/crates/fabryk-fts/src/backend.rs` — add `extra_filters` field to `SearchParams`
- `ecl/crates/fabryk-mcp-fts/src/tools.rs` — add `with_extra_search_schema()`, update call handler

**Scope:** ~80 lines changed, ~30 lines tests
**Verify:** `cargo test -p fabryk-fts -p fabryk-mcp-fts`

### Milestone 2.2: Extract QuestionSearchRegistry in Music Server

`search_by_question` is domain-specific (searches competency questions in frontmatter). Keep it as a ~30-line domain-specific registry rather than generalizing into fabryk.

**Files to modify:**

- `music-server/crates/server/src/server.rs` — extract `QuestionSearchRegistry` as a standalone struct (it's currently embedded in `SearchToolsRegistry`)

**Scope:** ~30 lines (restructure, no new logic)
**Verify:** `cargo test -p server`

### Milestone 2.3: Create GuideProvider Trait and GuideTools

Guides follow a generic pattern: list markdown files from a directory, get one by ID. Add `GuideProvider` trait + `GuideTools<P>` to `fabryk-mcp-content`.

**Files to modify:**

- `ecl/crates/fabryk-mcp-content/src/traits.rs` — add `GuideProvider` trait
- `ecl/crates/fabryk-mcp-content/src/tools.rs` — add `GuideTools<P>` with 2 slots (list_guides, get_guide)

**Scope:** ~120 lines new, ~60 lines tests
**Verify:** `cargo test -p fabryk-mcp-content`

---

## Phase 3: Extend Fabryk Graph Tools

**Goal:** Grow `fabryk-mcp-graph` from 8 tools to 16+ and add a domain-filter extension point. This is the largest phase because the music server's `GraphToolsRegistry` is ~630 lines with 16 tools.

### Milestone 3.1: Add get_node and get_node_edges

Fundamental node-level queries missing from fabryk.

**Files to modify:**

- `ecl/crates/fabryk-mcp-graph/src/tools.rs` — add `SLOT_GET_NODE`, `SLOT_GET_NODE_EDGES` + handlers
- `ecl/crates/fabryk-graph/src/query.rs` — add `NodeDetail` response type if needed

**Scope:** ~120 lines new, ~50 lines tests
**Verify:** `cargo test -p fabryk-graph -p fabryk-mcp-graph`

### Milestone 3.2: Add get_dependents and graph_status

`get_dependents` = reverse prerequisites (who depends on X?). `graph_status` = is the graph loaded + basic counts.

**Files to modify:**

- `ecl/crates/fabryk-graph/src/algorithms.rs` — add `dependents()` function
- `ecl/crates/fabryk-mcp-graph/src/tools.rs` — add `SLOT_DEPENDENTS`, `SLOT_STATUS`

**Scope:** ~150 lines new, ~50 lines tests
**Verify:** `cargo test -p fabryk-graph -p fabryk-mcp-graph`

### Milestone 3.3: Add source-concept relationship queries

Three tools querying Source↔Concept edges: `concept_sources` (what sources teach X?), `concept_variants` (source-specific versions of X), `source_coverage` (what does source Y teach?).

**Files to modify:**

- `ecl/crates/fabryk-graph/src/algorithms.rs` — add `concept_sources()`, `concept_variants()`, `source_coverage()`
- `ecl/crates/fabryk-mcp-graph/src/tools.rs` — add 3 new slots

**Scope:** ~200 lines new, ~80 lines tests
**Verify:** `cargo test -p fabryk-graph -p fabryk-mcp-graph`

### Milestone 3.4: Add learning_path and bridge_between_categories

`learning_path` = topologically sorted prerequisite chain with step numbers. `bridge_between_categories` = nodes connecting two specific categories.

**Files to modify:**

- `ecl/crates/fabryk-graph/src/algorithms.rs` — add `learning_path()`, `bridge_between_categories()`
- `ecl/crates/fabryk-mcp-graph/src/tools.rs` — add 2 new slots

**Scope:** ~180 lines new, ~60 lines tests
**Verify:** `cargo test -p fabryk-graph -p fabryk-mcp-graph`

### Milestone 3.5: Add GraphNodeFilter extension trait

The music server's graph tools have `tier` and `min_confidence` filters on many tools. Add a post-filter extension point.

**Files to modify:**

- `ecl/crates/fabryk-mcp-graph/src/tools.rs` — add `GraphNodeFilter` trait, `with_node_filter()`, `with_extra_schema()`, update handlers to apply filter

**Key design:**

```rust
pub trait GraphNodeFilter: Send + Sync {
    fn matches(&self, node: &Node, extra_args: &serde_json::Value) -> bool;
}
```

**Scope:** ~150 lines changed, ~40 lines tests
**Verify:** `cargo test -p fabryk-mcp-graph`

---

## Phase 4: Migrate Music Server to Fabryk Tools

**Goal:** Replace all 6 hand-rolled registries in `server.rs` with fabryk's tools + thin adapters. `server.rs` shrinks from ~1850 lines to ~300 lines.

### Milestone 4.1: Replace ConceptToolsRegistry and SourceToolsRegistry

Remove `ConceptToolsRegistry` (~120 lines) and `SourceToolsRegistry` (~170 lines). Replace with `ContentTools::new(provider).with_names(...)` and `SourceTools::new(provider).with_names(...)`.

**Files to modify:**

- `music-server/crates/server/src/server.rs` — remove 2 registries, update `build_server()`
- `music-server/crates/server/src/content.rs` — update `MusicTheoryContentProvider` to implement `list_items_filtered`

**Scope:** ~200 lines removed, ~80 lines added (net -120)
**Verify:** `cargo test -p server` (especially `test_build_server_has_all_tools`)

### Milestone 4.2: Replace SearchToolsRegistry and GuideToolsRegistry

Remove `SearchToolsRegistry` (~190 lines) and `GuideToolsRegistry` (~60 lines). Replace with `FtsTools.with_names(...)`, small `QuestionSearchRegistry`, and `GuideTools::new(provider).with_names(...)`.

**Files to modify:**

- `music-server/crates/server/src/server.rs` — remove 2 registries, update `build_server()`
- `music-server/crates/server/src/content.rs` — add `MusicTheoryGuideProvider` implementing `GuideProvider`

**Scope:** ~250 lines removed, ~100 lines added (net -150)
**Verify:** `cargo test -p server`

### Milestone 4.3: Replace GraphToolsRegistry

The biggest single migration. Remove `GraphToolsRegistry` (~630 lines). Replace with `GraphTools::with_shared(data).with_names(...).with_node_filter(...)`.

**Files to modify:**

- `music-server/crates/server/src/server.rs` — remove GraphToolsRegistry, update `build_server()`
- `music-server/crates/server/src/graph/mod.rs` — add `MusicTheoryNodeFilter` implementing `GraphNodeFilter`

**Scope:** ~630 lines removed, ~100 lines added (net -530)
**Verify:** `cargo test -p server`

### Milestone 4.4: Clean Up Dead Code

Remove ~20 orphaned arg structs, 3 helper functions (`json_schema`, `make_tool`, `serialize_response`), 5 default-value functions, and unused imports.

**Files to modify:**

- `music-server/crates/server/src/server.rs` — delete dead code

**Scope:** ~300 lines removed
**Verify:** `cargo test -p server && cargo clippy -p server -- -D warnings`

---

## Phase 5: Thin Shell Finalization

### Milestone 5.1: Consolidate server.rs

Final form of `server.rs` should contain:

- `MusicTheoryResources` (~50 lines)
- `QuestionSearchRegistry` (~30 lines)
- `HealthToolsRegistry` (~30 lines)
- `build_server()` (~60 lines)
- Tests (~100 lines)
- Total: ~270 lines

Reorder, add section comments, verify all tests pass.

**Verify:** Full test suite + manual Claude Desktop integration test

### Milestone 5.2: Create Phase/Milestone Implementation Docs

Write detailed implementation docs for each phase and milestone to `ai-maths/workbench/plans/`. These docs will serve as the blueprint for Claude Code instances executing each milestone.

**Deliverables:**

- `workbench/plans/phase-1-content-source-tools.md`
- `workbench/plans/phase-2-fts-guide-tools.md`
- `workbench/plans/phase-3-graph-tools.md`
- `workbench/plans/phase-4-music-server-migration.md`
- `workbench/plans/phase-5-finalization.md`
- Individual milestone docs within each phase doc

---

## Key Risks

1. **Argument schema drift** — fabryk's tools may produce slightly different JSON schemas than the music server's hand-rolled ones. Mitigate with `.with_names()` + `.with_descriptions()` + integration tests comparing schemas before/after.

2. **Response format differences** — fabryk's `GraphTools` returns generic `NodeSummary` while music server returns richer domain types. Mitigate by ensuring fabryk response types are at least as rich, and the music server's adapter code post-processes where needed.

3. **Feature gate alignment** — music server uses `#[cfg(feature = "graph")]` extensively. The `ServiceAwareRegistry` wrapper in fabryk handles this correctly.

## Verification Strategy

After each milestone:

1. `cargo test` in the modified repo
2. `cargo clippy -- -D warnings`
3. After Phase 4 milestones: compare tool list output before/after to verify no tools were lost or renamed
4. After Phase 5: manual Claude Desktop integration test
