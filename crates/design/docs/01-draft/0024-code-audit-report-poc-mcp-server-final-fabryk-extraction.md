---
number: 24
title: "Code Audit Report: PoC MCP Server — Final Fabryk Extraction"
author: "any Fabryk"
component: All
tags: [change-me]
created: 2026-04-14
updated: 2026-04-14
state: Draft
supersedes: null
superseded-by: null
version: 1.0
---

# Code Audit Report: PoC MCP Server — Final Fabryk Extraction

**Date:** 2026-04-14
**Scope:** `mcp-server/crates/server/src/` (15 Rust source files, 6 modules)
**Purpose:** Fresh audit after repo updates. Identify what's truly domain-specific vs. what should move to Fabryk.
**Method:** For every item, the classification question is: "Does this code require domain-specific *knowledge* (pitch arithmetic, chord theory, metric spaces, etc.) to function, or only domain-specific *data* (config values, display strings)?" If only data, it's generic.

## Changes Since v1 Audit (2026-04-13)

- **New file:** `tools/oth.rs` (~1,850 lines) -- 12 Open Tone Harmony tools for quintal/quartal chord analysis
- **New struct:** `OthToolsRegistry` in `server.rs` (~470 lines) -- registry wiring for the 12 OTH tools
- **Removed:** `search/simple_search.rs` -- `SimpleSearch` now re-exported directly from `fabryk::fts`
- **Updated:** `tools/music_theory.rs` -- minor description clarifications (inversion disambiguation)
- **Updated:** `Cargo.toml` -- `music-comp-mt` bumped to v0.5 with `serde` feature
- **Unchanged:** cache.rs, cli.rs, config.rs, error.rs, state.rs, graph/mod.rs, resources/mod.rs, lib.rs, main.rs

## Summary

| Category | Items | Approx. Lines |
|----------|-------|---------------|
| Truly domain-specific | 14 items | ~3,160 lines |
| Domain-agnostic, ready to move | 36 items | ~2,060 lines |
| Hardcoded paths needing parameterization | 2 items | ~60 lines |

The new OTH tools add ~2,320 lines of genuinely domain-specific code (they do quintal/quartal pitch-class arithmetic, metric space geometry, fiber-bundle computation). The `SimpleSearch` extraction to fabryk is already done. The overall ratio of generic-to-domain code has shifted: more domain code now, but the generic code remains unchanged and ready to move.

---

## Section 1: Truly Domain-Specific Code (Belongs in This Repo, Not Fabryk)

**Classification rationale:** Each item requires domain-specific *knowledge* to function -- music-theory pitch arithmetic, chord construction, metric space geometry, scale theory, or music-specific textual content. These import `music_comp_mt` or contain literal music-theory concepts.

### Music theory computation tools

- **`tools/music_theory.rs`** (entire file, ~552 lines) -- 9 tools (`get_scale_notes`, `get_chord_notes`, `get_interval`, `transpose`, `get_diatonic_chords`, `identify_chord`, `identify_scale`, `check_enharmonic`, `analyze_roman_numerals`) plus Args/Response structs. All call `music_comp_mt` for pitch parsing, chord identification, scale analysis, Roman numeral analysis.

- **`server.rs:51-375` -- `MusicTheoryToolsRegistry`** -- ToolRegistry impl defining 9 music-specific tool schemas (JSON schemas referencing "tonic", "mode", "chord quality", "inversion") and dispatch to `tools::music_theory::*`.

### Open Tone Harmony tools (NEW)

- **`tools/oth.rs`** (entire file, ~1,850 lines) -- 12 tools across 3 tiers for quintal/quartal chord analysis in the [6,8] metric space. Includes orbit analysis, mode enumeration, parent-scale lookups, chord identification, fiber-bundle (chord scale / inversion cycle) computations, geodesic pathfinding, betweenness centrality, and mathematical verification. All require deep music-theory knowledge: pitch-class sets, interval structures, T/I orbits, Tymoczko interscalar transposition. Imports extensively from `music_comp_mt::quintal`, `music_comp_mt::quartal`, `music_comp_mt::set_class`.

- **`server.rs:378-847` -- `OthToolsRegistry`** -- ToolRegistry impl defining 12 OTH-specific tool schemas and dispatch to `tools::oth::*`. Schema descriptions reference "quintal label", "quartal label", "pitch classes", "fiber class", "chord scale", "geodesic distance", etc.

### Domain-specific configuration and content

- **`resources/mod.rs`** (entire file, ~185 lines) -- 4 fallback content functions returning hardcoded music theory text (pitch notation, Roman numerals, Lewin/Tymoczko/Cohn references).

- **`config.rs:79-100` -- `SourcesConfig`** -- Categories `oxford`, `general`, `papers` specific to this project's source material taxonomy.

- **`config.rs:220-229` -- `default_stopword_allowlist()`** -- Music theory Roman numerals and solfege syllables.

- **`config.rs:439` -- `MUSIC_THEORY_CONFIG_DIR` env var** -- Project-specific environment variable.

### Domain-specific naming/branding

- **`server.rs:1249-1257`** -- The `.with_description("Music Theory AI Skill - ...")` call and the 4 resource definitions with `fallback: Some(crate::resources::default_*())` calls. (The rest of `build_server()` is generic registry composition -- see Section 2.)

- **`cli.rs:22-42` -- `Cli` struct naming** -- `"music-theory-mcp"`, `"Music Theory AI Skill MCP Server"`.

- **`cache.rs:32-35` -- `DEFAULT_RELEASE_BASE_URL` / `DEFAULT_PROJECT_PREFIX`** -- Hardcoded GitHub URL and `"music-theory"` prefix.

- **`main.rs`** -- Crate name `music_theory_mcp`. Minimal but project-specific.

- **`lib.rs:1` -- doc comment** `"Music Theory MCP Server library"`.

---

## Section 2: Domain-Agnostic Code (Ready to Move to Fabryk)

**Classification rationale:** Each item implements a generic pattern. None require domain-specific knowledge to function. They operate on generic knowledge-management abstractions (concepts, sources, guides, graphs, search) or pure infrastructure (caching, error mapping, config, CLI dispatch, state management). Names like "concept_cards", "sources", "prerequisites" are standard Fabryk vocabulary.

### Cache management (candidate: `fabryk-cli` or `fabryk-cache`)

- **`cache.rs:45-78` -- `CacheBackend` enum** -- Generic backend discriminator (Graph, Fts, Vector) with Display/FromStr.
- **`cache.rs:83-148` -- `CacheManifest`, `CacheEntry`, `BackendStatus`, `CacheStatusReport`** -- Generic manifest tracking. Data model: version, checksum, downloaded_at, files_present.
- **`cache.rs:177-189` -- `archive_name()`, `release_url()`, `checksum_url()`** -- URL/naming helpers (need project prefix parameterized).
- **`cache.rs:200-220` -- `load_manifest()`, `save_manifest()`** -- JSON manifest persistence.
- **`cache.rs:262-340` -- `shell_download()`, `verify_checksum()`, `extract_archive()`** -- Shell-based download/checksum/extract utilities.
- **`cache.rs:358-414` -- `download_cache()`** -- Cache download orchestration.
- **`cache.rs:424-467` -- `package_cache()`** -- Cache packaging (except hardcoded paths, see Section 3).
- **`cache.rs:477-487` -- `parse_backend_arg()`** -- Parses "all" or individual backend name.
- **`cache.rs:497-531` -- `iso8601_now()`, `days_to_date()`** -- Pure date/time utilities.

### Error handling (candidate: `fabryk-mcp-core`)

- **`error.rs:17-44` -- `McpErrorContextExt` trait** -- Maps Error variants to MCP ErrorCode. Has existing TODO: "Phase 3 -- consider upstreaming to fabryk_mcp_core::McpErrorExt".

### Search (candidate: `fabryk-fts`)

- **`search/mod.rs:44-87` -- `to_fabryk_search_config()`, `to_fabryk_query_mode()`** -- Config conversion adapters.
- **`search/mod.rs:98-142` -- `IndexStats` adapter** -- Per-content-type document counts with backward-compatible field names.
- **`search/mod.rs:152-189` -- `IndexMetadata` adapter** -- Backward-compatible wrapper over fabryk's IndexMetadata.
- **`search/mod.rs:205-300` -- `build_index()`** -- Orchestrates multi-directory index building using fabryk's IndexBuilder.
- **`search/mod.rs:311-320` -- `is_index_fresh()`** -- Delegates freshness check to fabryk.
- **(ALREADY DONE)** `SimpleSearch` -- now re-exported from `fabryk::fts::SimpleSearch`.

### Configuration (candidate: `fabryk-core` / `fabryk-cli`)

- **`config.rs:33-76` -- `PathsConfig`** -- Named content paths (concept_cards, sources_md, guides, etc.) with tilde expansion.
- **`config.rs:116-270` -- `QueryMode` enum + `SearchConfig`** -- Generic search configuration (backend, fuzzy, query mode, field boosts, stopwords).
- **`config.rs:273-308` -- `LanceDbConfig`** -- Generic vector search config.
- **`config.rs:360-393` -- `ConfigProvider` impl** -- Maps content type strings to paths.
- **`config.rs:399-448` -- `ConfigManager` impl** -- Config loading with path resolution, env var fallback, marker-based search.

### Application state (candidate: `fabryk-mcp` or `fabryk-core`)

- **`state.rs` -- `AppState`** (entire struct + methods, ~300 lines) -- Generic state managing search backends, graph data (with service lifecycle via ServiceHandle), and vector index. Methods: `search_backend()`, `active_backend_name()`, `is_fts_ready()`, `update_fts_backend()`, `require_graph()`, `require_vector()`, `update_vector_backend()`.

### CLI (candidate: `fabryk-cli`)

Note: `fabryk-cli` already has `BaseCommand`, `GraphCommand`, `GraphSubcommand`, `ConfigCommand`, `ConfigAction`, `SourcesCommand`, `VectordbCommand`, `FabrykCli<C>`. Several items below may already be partially duplicated in fabryk-cli.

- **`cli.rs:96-149` -- `GraphCommands`/`GraphSubcommand`, `VectordbCommands`/`VectordbSubcommand`, `CacheCommands`/`CacheSubcommand`** -- Generic CLI subcommand definitions. Overlap with fabryk-cli's `GraphCommand`, `VectordbCommand`.
- **`cli.rs:199-273` -- `handle_command()` dispatch** -- Generic server dispatch.
- **`cli.rs:276-299` -- `handle_config_command()`** -- Generic config command dispatch using `fabryk_cli::config_handlers`.

### Graph (candidate: already mostly in `fabryk-graph`)

- **`graph/mod.rs:29-60` -- re-exports from `fabryk::graph`** -- Pass-through.
- **`graph/mod.rs:73-96` -- `GraphStats`, `LoadedGraph`** -- Backward-compatible adapter types.
- **`graph/mod.rs:102-154` -- node discrimination helpers** -- `is_concept_node()`, `is_source_node()`, `node_id()`, `node_title()`, `node_category()`, `source_author()`, `source_year()`, `source_is_converted()`. Generic graph-node inspection (check NodeType::Domain vs NodeType::Custom, read metadata keys).
- **`graph/mod.rs:160-198` -- `to_fabryk_relationship()` / `from_fabryk_relationship()`** -- Maps generic knowledge-graph relationships (Prerequisite, RelatesTo, Extends, etc.) to fabryk variants. Adapter/display code.
- **`graph/mod.rs:217-254` -- `load_concept_graph()`, `compute_graph_stats()`** -- Loads graph from JSON, computes node-type statistics.
- **`graph/mod.rs:276-296` -- `build_graph()`** -- Generic graph building using fabryk's GraphBuilder.

### MCP helpers (candidate: `fabryk-mcp` / `fabryk-mcp-core`)

- **`server.rs:24-46` -- `make_tool()`, `serialize_response()`, `to_mcp_error()`** -- Generic MCP helpers. `make_tool` builds a Tool from name/desc/schema; `serialize_response` wraps any `T: Serialize` into CallToolResult; `to_mcp_error` wraps error conversion.
- **`server.rs:853-871` -- `tier_confidence_schema()`** -- Generic metadata filter schema (tier: foundational/intermediate/advanced, confidence: high/medium/low). Applicable to any knowledge base.
- **`server.rs:894-1247` -- most of `build_server()`** -- Generic composition of Fabryk registries (ContentTools, SourceTools, GuideTools, FtsTools, GraphTools, SemanticSearchTools, QuestionSearchTools, HealthTools) with config-driven names and descriptions. The registry-composition pattern is reusable by any Fabryk-based MCP server. The only domain-specific parts are the two `.add(music_theory_tools)` / `.add(oth_tools)` calls and the fallback resource content.

---

## Section 3: Hardcoded Paths Needing Parameterization

**Classification rationale:** The logic is generic, but file paths are hardcoded rather than read from configuration, blocking extraction regardless of naming.

- **`cache.rs:230-255` -- `cache_status()`** -- Checks hardcoded: `data/graphs/concept_graph.json`, `.tantivy-index`, `.cache/vector/vector-cache.json`. **Fix:** Accept paths from config.
- **`cache.rs:437-466` -- `package_cache()` backend-specific paths** -- Same hardcoded paths in `match backend` arms. **Fix:** Same.

---

## Appendix: Recommended Migration Priority

1. **`cache.rs`** -> `fabryk-cli` or `fabryk-cache` (largest single block, ~500 lines of generic code)
2. **`McpErrorContextExt`** -> `fabryk-mcp-core` (already has a TODO)
3. **`AppState`** pattern -> `fabryk-mcp` or `fabryk-core` (~300 lines, enables all downstream projects)
4. **`make_tool()` / `serialize_response()` / `tier_confidence_schema()`** -> `fabryk-mcp` (small, high-frequency)
5. **CLI deduplication** -- reconcile local `GraphCommands`/`VectordbCommands`/`CacheCommands` with fabryk-cli's existing `GraphCommand`/`VectordbCommand`/`BaseCommand`
6. **`build_server()` pattern** -> extract generic registry-composition scaffold to `fabryk-mcp`
7. **Graph adapters** (`GraphStats`, `LoadedGraph`, node helpers, relationship mapping) -> `fabryk-graph`
8. **Search adapters** (`IndexStats`, `IndexMetadata`, `build_index()`, config conversion) -> `fabryk-fts`

## Appendix: What's Already Been Moved (since v1)

- `SimpleSearch` -- now re-exported from `fabryk::fts::SimpleSearch` (was local `search/simple_search.rs`, now deleted)
