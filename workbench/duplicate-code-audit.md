# Cross-Crate Duplicate Code Audit

**Date:** 2026-04-16
**Scope:** All 40 crates in the textrynum workspace
**Goal:** Identify duplicate code/functionality across crates, propose consolidation

---

## Table of Contents

1. [Critical: Auth/Token Provider Duplication (GCS vs GDrive)](#1-critical-authtokenProvider-duplication)
2. [High: MCP Tool Helper Functions](#2-high-mcp-tool-helper-functions)
3. [High: Concept Card Extractor Duplication](#3-high-concept-card-extractor-duplication)
4. [High: Builder Pattern Duplication (FTS/Vector/Graph)](#4-high-builder-pattern-duplication)
5. [Medium: String Utility Functions](#5-medium-string-utility-functions)
6. [Medium: Error Type Duplication](#6-medium-error-type-duplication)
7. [Medium: Probe/Health Check Patterns](#7-medium-probehealth-check-patterns)
8. [Medium: Persistence/Metadata Patterns](#8-medium-persistencemetadata-patterns)
9. [Low: Config Struct Patterns](#9-low-config-struct-patterns)
10. [Summary Matrix](#10-summary-matrix)

---

## 1. Critical: Auth/Token Provider Duplication

**Impact:** ~400 lines of near-identical code across 2 crates
**Effort:** Medium (create shared `ecl-adapter-auth` crate or move to `ecl-core`)

### Duplicated Files

| Component | ecl-adapter-gcs/src/auth.rs | ecl-adapter-gdrive/src/auth.rs |
|-----------|---------------------------|-------------------------------|
| `TokenProvider` struct | Lines 28-36 | Lines 28-34 |
| `CachedToken` struct | Lines 38-43 | Lines 36-41 |
| `JwtClaims` struct | Lines 46-53 | Lines 44-51 |
| `create_service_account_jwt()` | Lines 151-174 | Lines 145-167 |
| `exchange_jwt_for_token()` | Lines 177-207 | Lines 170-200 |
| `get_token()` | Lines 56-105 | Lines 53-99 |
| `service_account_flow()` | Lines 108-148 | Lines 102-142 |
| `adc_flow()` | Lines 232-252 | Lines 226-248 |
| `refresh_token_flow()` | Lines 285-321 | Lines 281-317 |
| `well_known_adc_path()` | Lines 325-327 | Lines 321-323 |
| `resolve_credential_file()` | Lines 255-282 | Lines 251-278 |

### Analysis

- `exchange_jwt_for_token()` is **byte-for-byte identical** except error type names
- `refresh_token_flow()` is **byte-for-byte identical** except error type names
- `well_known_adc_path()` is **exactly identical**
- `JwtClaims` and `CachedToken` structs are **exactly identical**
- Only meaningful difference: GCS `TokenProvider` has a `scope: String` field for dynamic OAuth scope; GDrive hardcodes `DRIVE_READONLY_SCOPE`

### Proposed Fix

Extract a shared `GoogleTokenProvider<E>` generic over the error type into either:
- A new `ecl-adapter-auth` crate, or
- `ecl-core::auth` module

The provider should accept scope as a constructor parameter (GCS pattern). Both adapter crates then re-export a type alias with their error type. This eliminates ~300 lines of duplication while preserving per-adapter error types.

### Related: fabryk-gcp/src/adc.rs

`fabryk-gcp` has its own `resolve_adc_path()` (lines 53-81) which is more sophisticated than the adapters' `well_known_adc_path()` (checks macOS vs Linux paths, validates file existence). The adapters do NOT use it. Consider unifying ADC resolution into `fabryk-gcp` and having the adapters depend on it.

---

## 2. High: MCP Tool Helper Functions

**Impact:** ~60 lines duplicated 4x = ~180 wasted lines, plus inconsistent use of core helpers
**Effort:** Low (just switch imports)

### Duplicated Functions (identical in all 4 tool crates)

| Function | fabryk-mcp-content/src/tools.rs | fabryk-mcp-fts/src/tools.rs | fabryk-mcp-graph/src/tools.rs | fabryk-mcp-semantic/src/tools.rs |
|----------|------|------|------|------|
| `json_schema()` | Lines 20-25 | Lines 21-26 | Lines 59-64 | Lines 22-27 |
| `make_tool()` | Lines 46-52 | Lines 28-34 | Lines 66-72 | Lines 29-35 |
| `serialize_response()` | Lines 28-32 | Lines 36-40 | Lines 85-89 | Lines 37-41 |

### Analysis

`fabryk-mcp-core/src/helpers.rs` already provides `make_tool()` (line 34) and `serialize_response()` (line 57), but **none of the 4 tool crates import them**. They all re-implement locally.

`json_schema()` is NOT in core yet but should be.

### Proposed Fix

1. Add `json_schema()` to `fabryk-mcp-core/src/helpers.rs`
2. Delete the local copies in all 4 tool crates
3. Replace with imports: `use fabryk_mcp_core::helpers::{json_schema, make_tool, serialize_response};`

### Additional: Tool Registry Boilerplate

All tool structs (7 total across 4 crates) repeat:
- `custom_names: HashMap<String, String>` and `custom_descriptions: HashMap<String, String>` fields
- `fn tool_name(&self, slot) -> String` and `fn tool_description(&self, slot, default) -> String` private helpers
- `fn new()`, `fn from_boxed()` / `fn with_shared()` constructors

**Proposed Fix:** Extract a `ToolCustomization` struct to core that handles name/description overrides, or create a trait with default methods. This is a larger refactor but eliminates significant per-tool-struct boilerplate.

### Additional: Schema Merge Pattern

All tool crates merge extra schema properties into tool schemas. `fabryk-mcp-graph` already extracted this to a `merge_extra_schema()` method. The others do it inline.

**Proposed Fix:** Add `merge_extra_schema(base: &mut Value, extra: &Value)` to `fabryk-mcp-core/src/helpers.rs`.

---

## 3. High: Concept Card Extractor Duplication

**Impact:** ~80 lines of shared logic reimplemented 3x
**Effort:** Medium

### Duplicated Across 3 Crates

| Logic | fabryk-fts (concept_card_extractor.rs) | fabryk-vector (concept_card_extractor.rs) | fabryk-graph (concept_card_extractor.rs) |
|-------|-------|--------|--------|
| Title resolution | Lines 57-63 | Lines 49-53 | Lines 101-106 |
| Category resolution | Lines 66-72 | Lines 55-58 | Lines 108-112 |
| `extract_string_array()` | N/A | Lines 108-119 | Lines 235-246 |

### Analysis

- **Title resolution** follows the same priority chain in all 3: `frontmatter.title -> frontmatter.concept -> first_heading -> file_stem`. Each reimplements this.
- **Category resolution** follows the same priority: `frontmatter.category -> parent_dir -> "general"`. Each reimplements.
- **`extract_string_array()`** is **identically duplicated** in fabryk-vector and fabryk-graph (extracts YAML array or comma-separated string into `Vec<String>`).

### Proposed Fix

1. Move `extract_string_array()` to `fabryk-content::markdown::helpers` (alongside existing helpers)
2. Create `resolve_title(frontmatter, content, file_stem) -> String` in `fabryk-content`
3. Create `resolve_category(frontmatter, file_path, base_path) -> String` in `fabryk-content`
4. All three extractors call these shared functions instead of reimplementing

---

## 4. High: Builder Pattern Duplication (FTS/Vector/Graph)

**Impact:** Significant structural duplication across 3 builder modules
**Effort:** Medium-High

### Parallel Implementations

| Component | fabryk-fts/src/builder.rs | fabryk-vector/src/builder.rs | fabryk-graph/src/builder.rs |
|-----------|--------------------------|-----------------------------|-----------------------------|
| File discovery | `find_files_with_extensions()` (custom) | `discover_files()` (uses core) | `discover_files()` (uses core) |
| Error handling enum | Inline | `ErrorHandling` enum | `ErrorHandling` enum |
| Build stats | `IndexStats` | `VectorIndexStats` | `BuildStats` |
| Content hash | Via `IndexMetadata` | blake3 + content bytes | `DefaultHasher` + mtime |

### Key Issues

1. **File discovery inconsistency:** FTS implements its own `find_files_with_extensions()` instead of using `fabryk_core::util::files::{find_all_files, FindOptions}` which Vector and Graph use correctly.

2. **Content hash inconsistency:** Vector uses blake3 on actual content bytes (reliable). Graph uses `DefaultHasher` on file path + modification time (misses content-only changes without mtime update). These should be standardized.

3. **ErrorHandling enum:** Defined separately in Vector and Graph builders with identical variants (FailFast, Collect, Skip). Should be in core.

### Proposed Fix

1. **Refactor FTS builder** to use `fabryk_core::util::files::find_all_files` instead of custom file walker
2. **Standardize content hashing** on blake3 + content bytes (Vector's approach) — move to `fabryk-core::util`
3. **Move `ErrorHandling` enum** to `fabryk-core` since it's shared across builders
4. Consider a `BuilderBase` trait or shared struct for the common file-discovery + stats-accumulation pattern

---

## 5. Medium: String Utility Functions

**Impact:** ~30 lines duplicated across 2 crates
**Effort:** Low

### Duplicated Functions

| Function | Location | Lines |
|----------|----------|-------|
| `humanize_id()` (private) | `fabryk-content/src/metadata.rs` | 278-294 |
| `humanize_source_id()` (public) | `fabryk-mcp-content/src/fs_source_provider.rs` | 339-354 |

### Analysis

Both convert kebab-case/snake_case identifiers to title case (e.g., `"open-music-theory"` -> `"Open Music Theory"`). Functionally identical; only minor style differences in the uppercase conversion.

`fabryk-core/src/util/ids.rs` already has `normalize_id()` which does the reverse (title case -> kebab-case).

### Proposed Fix

Add `humanize_id()` to `fabryk-core::util::ids` alongside `normalize_id()` for a complete bidirectional transformation suite. Both current call sites import from core.

---

## 6. Medium: Error Type Duplication

**Impact:** ~100 lines of structural duplication
**Effort:** Medium

### Identical Error Enums

`ecl-adapter-gcs/src/error.rs` and `ecl-adapter-gdrive/src/error.rs` are **97 lines each** and structurally identical:

```
Auth { message }
InvalidCredentials { message }
InvalidPattern { pattern, message }
ApiError { status, message }
Http(#[from] reqwest::Error)
Json(#[from] serde_json::Error)
Io(#[from] std::io::Error)
```

Only the type name differs (`GcsAdapterError` vs `DriveAdapterError`).

### Additional: InvalidPattern Variant

The `InvalidPattern { pattern, message }` variant appears in 3 adapter error types:
- `ecl-adapter-fs/src/error.rs` (lines 14-20)
- `ecl-adapter-gcs/src/error.rs` (lines 23-30)
- `ecl-adapter-gdrive/src/error.rs` (lines 23-30)

### Proposed Fix

1. **Immediate:** Unify `GcsAdapterError` and `DriveAdapterError` into a shared `GoogleAdapterError` in a new `ecl-adapter-auth` crate (or `ecl-core::error`)
2. **Follow-up:** Extract common error variants (`Auth`, `InvalidPattern`, `ApiError`) into a shared base, and have adapter-specific errors compose via `#[from]`

---

## 7. Medium: Probe/Health Check Patterns

**Impact:** ~60 lines of structural duplication
**Effort:** Low-Medium

### Duplicate Probe Adapters

| Component | fabryk-fts/src/probe.rs | fabryk-vector/src/probe.rs |
|-----------|------------------------|---------------------------|
| Struct | `SearchProbe<B>` | `VectorProbe<B>` |
| Wraps | `Arc<B>` where `B: SearchBackend` | `Arc<B>` where `B: VectorBackend` |
| Implements | `BackendProbe` trait | `BackendProbe` trait |
| Factory fn | `search_probe()` | `vector_probe()` |

Both are thin adapters that delegate `probe_name()`, `probe_ready()`, `probe_kind()` to the wrapped backend. Structurally identical.

### Proposed Fix

Create a generic `GenericProbe<B, F>` in `fabryk-core` that wraps any backend and maps its methods to `BackendProbe`. Or use a macro to generate the boilerplate. The domain-specific probe behavior (e.g., `probe_document_count()`) can be handled via an optional closure or trait bound.

---

## 8. Medium: Persistence/Metadata Patterns

**Impact:** ~80 lines of parallel implementation
**Effort:** Medium

### Parallel Implementations

| Component | fabryk-vector/src/persistence.rs | fabryk-graph/src/persistence.rs |
|-----------|--------------------------------|-------------------------------|
| Metadata struct | `IndexMetadata` (content_hash, document_count, built_at, etc.) | `GraphMetadata` (built_at, builder_version, content_hash, source_file_count) |
| Save function | `save_metadata()` — serde_json to file | `save_graph()` — serde_json to file |
| Load function | `load_metadata()` — serde_json from file | `load_graph()` — serde_json from file |
| Freshness check | `is_index_fresh()` — compare content_hash | Inline hash comparison in builder |

### Proposed Fix

Extract a generic `JsonPersistence<T: Serialize + DeserializeOwned>` utility to `fabryk-core::util` with:
- `save_json(path, &T) -> Result<()>`
- `load_json(path) -> Result<T>`
- `is_fresh(path, current_hash) -> bool`

Both crates use the shared utility with their domain-specific metadata types.

---

## 9. Low: Config Struct Patterns

**Impact:** Structural similarity, not true duplication
**Effort:** N/A (acceptable as-is)

### Observation

`SearchConfig` (fabryk-fts) and `VectorConfig` (fabryk-vector) follow identical patterns:
- `#[serde(default = "fn_name")]` for all fields
- Similar `default_*()` functions
- Builder-style `.with_*()` methods

However, the fields are domain-specific enough that consolidation would be forced abstraction. **No action recommended** — this is acceptable parallel structure.

---

## 10. Summary Matrix

| # | Category | Duplicated Lines | Crates Involved | Effort | Priority |
|---|----------|-----------------|-----------------|--------|----------|
| 1 | Auth/Token Providers | ~400 | ecl-adapter-gcs, ecl-adapter-gdrive | Medium | **Critical** |
| 2 | MCP Tool Helpers | ~180 | 4 fabryk-mcp-* crates | Low | **High** |
| 3 | Concept Card Extractors | ~80 | fabryk-fts, fabryk-vector, fabryk-graph | Medium | **High** |
| 4 | Builder Patterns | ~150 | fabryk-fts, fabryk-vector, fabryk-graph | Medium-High | **High** |
| 5 | String Utilities | ~30 | fabryk-content, fabryk-mcp-content | Low | **Medium** |
| 6 | Error Types | ~100 | ecl-adapter-gcs, ecl-adapter-gdrive, ecl-adapter-fs | Medium | **Medium** |
| 7 | Probe Adapters | ~60 | fabryk-fts, fabryk-vector | Low-Medium | **Medium** |
| 8 | Persistence/Metadata | ~80 | fabryk-vector, fabryk-graph | Medium | **Medium** |
| 9 | Config Structs | N/A | fabryk-fts, fabryk-vector | N/A | **None** |

### Estimated Total Duplicated Lines: ~1,080

### Recommended Execution Order

1. **MCP Tool Helpers** (Item 2) — lowest effort, immediate payoff, zero risk
2. **String Utilities** (Item 5) — quick win, already has a natural home in fabryk-core
3. **Auth/Token Providers** (Item 1) — highest impact, but needs careful error type design
4. **Error Types** (Item 6) — pairs naturally with auth provider consolidation
5. **Concept Card Extractors** (Item 3) — medium effort, clear benefit
6. **Probe Adapters** (Item 7) — straightforward generic extraction
7. **Persistence Patterns** (Item 8) — useful utility for core
8. **Builder Patterns** (Item 4) — largest refactor, do last
