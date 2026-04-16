# Plan: Cross-Crate Duplicate Code Consolidation

## Context

A comprehensive audit (`workbench/duplicate-code-audit.md`) identified ~1,080 lines of duplicate code across the textrynum workspace. This plan addresses each finding in priority order, organized into 6 independently-shippable phases. Each phase passes `make check` upon completion and can be committed separately.

---

## Phase 1: MCP Tool Helpers — Switch to Core Imports

**Why:** 4 tool crates each re-implement `json_schema()`, `make_tool()`, and `serialize_response()` locally despite `fabryk-mcp-core/src/helpers.rs` already providing `make_tool()` and `serialize_response()`. ~80 lines eliminated.

### Steps

1. **No changes needed to core's `make_tool()`** — it already accepts `serde_json::Value` and extracts the `Map` internally. `Tool::new()` accepts both `Map` and `Arc<Map>` via `Into<Arc<JsonObject>>`, so the local `json_schema()` Arc-wrapping is unnecessary. Core's version is a drop-in replacement.

2. **Align `serialize_response()` error message** — core's version includes `"Serialization error: "` prefix; locals use bare `e.to_string()`. Keep core's version as-is (the prefix aids debugging). No behavioral impact since callers don't parse the error string.

3. **Update each tool crate** — for each of the 4 files below, remove the local `json_schema`, `make_tool`, `serialize_response` functions and replace with imports from `fabryk_mcp_core`:

   | File | Remove lines | Keep local |
   |------|-------------|------------|
   | `crates/fabryk-mcp-fts/src/tools.rs` | Lines 21-40 (3 fns) | — |
   | `crates/fabryk-mcp-content/src/tools.rs` | Lines 20-25, 28-32, 46-52 (3 fns) | `extract_string_field` (lines 36-43) |
   | `crates/fabryk-mcp-graph/src/tools.rs` | Lines 59-72, 85-89 (3 fns) | `apply_node_filter` (lines 74-83) |
   | `crates/fabryk-mcp-semantic/src/tools.rs` | Lines 22-41 (3 fns) | — |

   Add to each: `use fabryk_mcp_core::{make_tool, serialize_response};`

   All 4 crates already depend on `fabryk-mcp-core` in their Cargo.toml.

### Testing
- `cargo test -p fabryk-mcp-core -p fabryk-mcp-fts -p fabryk-mcp-content -p fabryk-mcp-graph -p fabryk-mcp-semantic`
- `make check`

---

## Phase 2: Consolidate `humanize_id` in fabryk-core

**Why:** Two crates independently implement identical kebab/snake_case-to-Title-Case conversion. Natural home is `fabryk-core::util::ids` alongside the existing `normalize_id()` (reverse direction). ~30 lines eliminated.

### Steps

1. **Add `humanize_id()` to `crates/fabryk-core/src/util/ids.rs`** after `id_from_path` (line 59):
   ```rust
   /// Convert a kebab-case or snake_case identifier to Title Case.
   ///
   /// Inverse of [`normalize_id`].
   pub fn humanize_id(id: &str) -> String {
       id.split(['-', '_'])
           .filter(|s| !s.is_empty())
           .map(|word| {
               let mut chars = word.chars();
               match chars.next() {
                   Some(first) => {
                       let mut s = first.to_uppercase().to_string();
                       s.extend(chars);
                       s
                   }
                   None => String::new(),
               }
           })
           .collect::<Vec<_>>()
           .join(" ")
   }
   ```
   Add tests for: `"voice-leading"` → `"Voice Leading"`, `"jazz_theory_book"` → `"Jazz Theory Book"`, `""` → `""`, `"single"` → `"Single"`, `"some--double-dash"` → `"Some Double Dash"`.

2. **Re-export from `fabryk-core`** — check if `util::ids` contents are re-exported at crate root; add `humanize_id` to the same re-export.

3. **Update `crates/fabryk-content/src/metadata.rs`**:
   - Remove private `humanize_id` (lines 278-294)
   - Import: `use fabryk_core::util::ids::humanize_id;`
   - Call site at line 274 unchanged (same function name)

4. **Update `crates/fabryk-mcp-content/src/fs_source_provider.rs`**:
   - Remove `humanize_source_id` (lines 339-354) and its tests
   - Import: `use fabryk_core::util::ids::humanize_id;`
   - Replace all `humanize_source_id(...)` calls with `humanize_id(...)`

5. **Update `crates/fabryk-mcp-content/src/lib.rs`**:
   - Remove `humanize_source_id` from re-exports (line 51)
   - Check if anything external depends on `fabryk_mcp_content::humanize_source_id` — if so, add a deprecated re-export alias

### Testing
- `cargo test -p fabryk-core -p fabryk-content -p fabryk-mcp-content`
- `make check`

---

## Phase 3: Extract Shared `extract_string_array` to fabryk-content

**Why:** `extract_string_array(frontmatter: &yaml_serde::Value, key: &str) -> Vec<String>` is identically duplicated in fabryk-vector and fabryk-graph concept card extractors. ~15 lines eliminated.

### Steps

1. **Add `extract_string_array()` to `crates/fabryk-content/src/markdown/frontmatter.rs`** (where `extract_frontmatter` lives — same YAML concern):
   ```rust
   /// Extract an array of strings from a YAML frontmatter value.
   ///
   /// Handles both YAML sequences (`[a, b]`) and falls back to empty vec.
   pub fn extract_string_array(frontmatter: &yaml_serde::Value, key: &str) -> Vec<String> {
       frontmatter
           .get(key)
           .and_then(|v| v.as_sequence())
           .map(|seq| seq.iter().filter_map(|v| v.as_str()).map(String::from).collect())
           .unwrap_or_default()
   }
   ```
   Add tests. Re-export from `crates/fabryk-content/src/lib.rs`.

2. **Verify deps**: Confirm `fabryk-vector/Cargo.toml` and `fabryk-graph/Cargo.toml` depend on `fabryk-content`. Add if missing.

3. **Update `crates/fabryk-vector/src/concept_card_extractor.rs`**:
   - Remove local `extract_string_array` (lines 108-119)
   - Import: `use fabryk_content::extract_string_array;`

4. **Update `crates/fabryk-graph/src/concept_card_extractor.rs`**:
   - Remove local `extract_string_array` (lines 235-246)
   - Import: `use fabryk_content::extract_string_array;`

### Testing
- `cargo test -p fabryk-content -p fabryk-vector -p fabryk-graph`
- `make check`

---

## Phase 4: Move `ErrorHandling` Enum to fabryk-core

**Why:** The `ErrorHandling` enum (FailFast/Collect/Skip) is identically defined in both fabryk-vector and fabryk-graph builders. ~12 lines eliminated + proper centralization.

### Steps

1. **Add to `crates/fabryk-core/src/lib.rs`** (or a new `error_handling.rs` module):
   ```rust
   /// Strategy for handling errors during batch operations.
   #[derive(Clone, Debug, Default)]
   pub enum ErrorHandling {
       /// Stop processing on first error.
       #[default]
       FailFast,
       /// Collect all errors and return them at the end.
       Collect,
       /// Log errors and skip failed items.
       Skip,
   }
   ```
   Re-export from crate root.

2. **Update `crates/fabryk-vector/src/builder.rs`**: Remove local `ErrorHandling`, import from `fabryk_core`. Update any re-exports in `fabryk-vector/src/lib.rs`.

3. **Update `crates/fabryk-graph/src/builder.rs`**: Same as above.

### Testing
- `cargo test -p fabryk-core -p fabryk-vector -p fabryk-graph`
- `make check`

---

## Phase 5: Create `ecl-gcp-auth` Shared Auth Crate

**Why:** `ecl-adapter-gcs/src/auth.rs` (495 lines) and `ecl-adapter-gdrive/src/auth.rs` (498 lines) are 99% identical — same JWT creation, token exchange, refresh flow, ADC resolution, and caching. This is the highest-LOC savings (~335 net lines eliminated).

### Steps

1. **Create `crates/ecl-gcp-auth/`** with Cargo.toml:
   - Dependencies: `ecl-pipeline-spec`, `chrono`, `dirs`, `jsonwebtoken`, `reqwest` (with `form` feature), `serde`, `serde_json`, `thiserror`, `tokio`, `tracing`
   - Dev-dependencies: `tempfile`, `wiremock = "0.6"`
   - Add to workspace members in root `Cargo.toml`

2. **Create `ecl-gcp-auth/src/error.rs`**:
   - `GcpAuthError` enum with: `Auth { message }`, `InvalidCredentials { message }`, `Http(reqwest::Error)`, `Json(serde_json::Error)`, `Io(std::io::Error)`

3. **Create `ecl-gcp-auth/src/types.rs`**:
   - Move shared types: `ServiceAccountKey`, `AuthorizedUserCredentials`, `TokenResponse` (use GDrive's superset with optional `token_type`), `GOOGLE_TOKEN_URL` constant
   - Keep per-adapter types (GcsObject, DriveFile, API URLs, scope constants) in their original crates

4. **Create `ecl-gcp-auth/src/auth.rs`**:
   - `TokenProvider` with `scope: String` field (GCS pattern — GDrive passes `DRIVE_READONLY_SCOPE` at construction)
   - All shared methods: `new()`, `static_token()`, `with_token_url()`, `with_scope()`, `get_token()`, `service_account_flow()`, `env_var_flow()`, `adc_flow()`, `resolve_credential_file()`, `refresh_token_flow()`
   - Private helpers: `create_service_account_jwt()`, `exchange_jwt_for_token()`
   - `well_known_adc_path()` as module-level function
   - Migrate tests from both adapter crates

5. **Update `ecl-adapter-gcs`**:
   - `Cargo.toml`: Add `ecl-gcp-auth` dependency
   - `src/auth.rs`: Replace with `pub use ecl_gcp_auth::auth::TokenProvider;` (preserves import path for callers)
   - `src/types.rs`: Remove `ServiceAccountKey`, `AuthorizedUserCredentials`, `TokenResponse`, `GOOGLE_TOKEN_URL`; add `pub use ecl_gcp_auth::types::*;` or selective re-exports
   - `src/error.rs`: Add `#[error(transparent)] GcpAuth(#[from] ecl_gcp_auth::GcpAuthError)` variant, or implement `From<GcpAuthError> for GcsAdapterError`

6. **Update `ecl-adapter-gdrive`**: Same pattern as GCS above.

### Testing
- `cargo test -p ecl-gcp-auth` (all migrated auth tests)
- `cargo test -p ecl-adapter-gcs -p ecl-adapter-gdrive` (existing adapter tests)
- `make check`

### Key Risk
Callers importing `ecl_adapter_gcs::auth::TokenProvider` must still work. The re-export in step 5 preserves this path.

---

## Phase 6 (Optional): Probe Adapter Consolidation

**Why:** `SearchProbe` (fabryk-fts) and `VectorProbe` (fabryk-vector) are structurally identical thin wrappers. ~30 lines eliminated, but the generic solution adds complexity.

**Recommendation:** Defer unless a 3rd probe type is imminent. The duplication is small (~55 lines each) and the probes are unlikely to diverge.

If proceeding:
1. Define `ProbeableBackend` trait in `fabryk-core` with `name()`, `is_ready()`, `document_count() -> Option<usize>`
2. Create `GenericProbe<B: ProbeableBackend>` implementing `BackendProbe`
3. Replace `SearchProbe` and `VectorProbe` with type aliases

---

## Verification (After All Phases)

```bash
make check          # build + lint + test
make check-all      # build + lint + coverage (verify no coverage regression)
```

Verify coverage didn't regress on moved code — tests should migrate with the functions. Run `cargo llvm-cov --html` and spot-check the new shared modules.

## Summary

| Phase | Effort | Lines Removed | Files Changed | Commit Scope |
|-------|--------|--------------|---------------|-------------|
| 1 — MCP helpers | Low | ~80 | 6 | Single commit |
| 2 — humanize_id | Low | ~30 | 4 | Single commit |
| 3 — extract_string_array | Low | ~15 | 4 | Single commit |
| 4 — ErrorHandling | Low | ~12 | 3-4 | Single commit |
| 5 — ecl-gcp-auth | Medium | ~335 | 10+ new/modified | Single commit |
| 6 — Probes (optional) | Low-Med | ~30 | 4 | Defer |
| **Total** | | **~500** | | **5 commits** |
