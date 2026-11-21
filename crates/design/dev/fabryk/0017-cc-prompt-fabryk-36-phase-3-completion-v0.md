---
title: "CC Prompt: Fabryk 3.6 — Phase 3 Completion & v0.1-alpha Checkpoint"
milestone: "3.6"
phase: 3
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["3.1-3.5 complete"]
governing-docs: [0011-audit §4.3, 0012-amendment §2e, 0013-project-plan]
---

# CC Prompt: Fabryk 3.6 — Phase 3 Completion & v0.1-alpha Checkpoint

## Context

This milestone marks the **v0.1-alpha checkpoint** per Amendment §2e. After
completing Phase 3, we have extracted the core infrastructure that delivers
~60% of the extraction value:

- **fabryk-core**: Error types, utilities, traits, state
- **fabryk-content**: Markdown parsing, frontmatter extraction, helpers
- **fabryk-fts**: Full-text search with Tantivy backend

At this checkpoint, music-theory can begin using Fabryk crates for content and
search while graph functionality remains in-repo (Phase 4 is the highest-risk
phase).

**Key from Amendment §2e:**

> At the v0.1-alpha checkpoint:
> - Music theory skill works with Fabryk core + content + FTS
> - Graph still uses the existing in-repo code (not yet extracted)
> - You can validate the extraction pattern before tackling the hardest piece
> - If something's wrong with the approach, you find out at week 4, not week 10

## Objective

1. Verify all Phase 3 milestones (3.1-3.5) are complete
2. Run full test suite across all Fabryk crates
3. Generate documentation
4. Perform music-theory integration (update imports)
5. Tag v0.1-alpha for checkpoint tracking
6. Document lessons learned for Phase 4

## Verification Steps

### Step 1: Verify crate structure

```bash
cd ~/lab/oxur/ecl

# fabryk-core
tree crates/fabryk-core/src/
# Expected:
# ├── error.rs
# ├── lib.rs
# ├── state.rs
# ├── traits.rs
# └── util/
#     ├── files.rs
#     ├── ids.rs
#     ├── mod.rs
#     ├── paths.rs
#     └── resolver.rs

# fabryk-content
tree crates/fabryk-content/src/
# Expected:
# ├── lib.rs
# └── markdown/
#     ├── frontmatter.rs
#     ├── helpers.rs
#     ├── mod.rs
#     └── parser.rs

# fabryk-fts
tree crates/fabryk-fts/src/
# Expected:
# ├── backend.rs
# ├── builder.rs
# ├── document.rs
# ├── freshness.rs
# ├── indexer.rs
# ├── lib.rs
# ├── query.rs
# ├── schema.rs
# ├── stopwords.rs
# ├── tantivy_search.rs
# └── types.rs
```

### Step 2: Run full test suite

```bash
cd ~/lab/oxur/ecl

# Core crates (no features)
cargo test -p fabryk-core
cargo test -p fabryk-content
cargo test -p fabryk-fts

# FTS with Tantivy
cargo test -p fabryk-fts --features fts-tantivy

# All workspace tests
cargo test --workspace
```

Expected: All tests pass.

### Step 3: Run clippy

```bash
cargo clippy --workspace -- -D warnings
cargo clippy -p fabryk-fts --features fts-tantivy -- -D warnings
```

Expected: No warnings.

### Step 4: Generate documentation

```bash
cargo doc --workspace --no-deps --open
```

Review:
- [ ] fabryk-core documentation complete
- [ ] fabryk-content documentation complete
- [ ] fabryk-fts documentation complete
- [ ] All public types documented
- [ ] Examples in doc comments

### Step 5: Music-theory integration

Perform the integration documented in milestone 2.4:

**A. Add Fabryk dependencies to music-theory:**

```toml
# ai-music-theory/mcp-server/crates/server/Cargo.toml

[dependencies]
fabryk-core = { path = "../../../../oxur/ecl/crates/fabryk-core" }
fabryk-content = { path = "../../../../oxur/ecl/crates/fabryk-content" }
fabryk-fts = { path = "../../../../oxur/ecl/crates/fabryk-fts", features = ["fts-tantivy"] }
```

**B. Update imports:**

```rust
// Replace local imports:
// use crate::error::{Error, Result};
// use crate::markdown::{extract_frontmatter, extract_first_heading};

// With Fabryk imports:
use fabryk_core::{Error, Result};
use fabryk_content::{extract_frontmatter, extract_first_heading, FrontmatterResult};
```

**C. Define domain-specific Frontmatter:**

Keep music-theory's `Frontmatter` struct locally and deserialize from
`FrontmatterResult`:

```rust
impl Frontmatter {
    pub fn extract(content: &str) -> fabryk_core::Result<Option<Self>> {
        let result = fabryk_content::extract_frontmatter(content)?;
        result.deserialize()
    }
}
```

**D. Verify music-theory tests pass:**

```bash
cd ~/lab/music-comp/ai-music-theory/mcp-server
cargo test --all-features
```

### Step 6: v0.1-alpha success criteria

Per Amendment §2e, verify all criteria are met:

- [ ] `fabryk-core`, `fabryk-content`, `fabryk-fts` compile and pass tests
- [ ] Music theory MCP server depends on these crates for content + search
- [ ] All 10 non-graph MCP tools work identically
- [ ] Graph tools still work via in-repo code (unchanged)
- [ ] `cargo test --all-features` passes in both repos

### Step 7: Create v0.1-alpha tag

```bash
cd ~/lab/oxur/ecl
git tag -a fabryk-v0.1-alpha -m "Fabryk v0.1-alpha checkpoint

Phase 1-3 complete. Core infrastructure extracted:
- fabryk-core: Error types, utilities, ConfigProvider trait
- fabryk-content: Markdown parsing, frontmatter, helpers
- fabryk-fts: Full-text search with Tantivy backend

Music-theory integration validated. 10 non-graph MCP tools
working with Fabryk. Graph extraction (Phase 4) next.

Ref: Doc 0013 v0.1-alpha checkpoint, Amendment §2e"
```

## Phase 3 Summary

| Crate | Modules | Lines (est.) | Tests |
|-------|---------|--------------|-------|
| fabryk-fts | 10 | ~1,600 | ~70 |

**fabryk-fts provides:**

| Module | Purpose |
|--------|---------|
| `types` | `QueryMode`, `SearchConfig` |
| `schema` | Default 14-field Tantivy schema |
| `stopwords` | Stopword filtering with allowlist |
| `document` | `SearchDocument` with relevance scoring |
| `backend` | `SearchBackend` trait, `SimpleSearch` |
| `indexer` | Tantivy index writer wrapper |
| `builder` | Batch indexing with `DocumentExtractor` |
| `freshness` | Content hash validation |
| `query` | Weighted multi-field query building |
| `tantivy_search` | Full Tantivy search backend |

## v0.1-alpha Checkpoint Summary

**Extracted to Fabryk:**

| Phase | Crate | Description |
|-------|-------|-------------|
| 1 | fabryk-core | Error types, utilities, traits, state |
| 2 | fabryk-content | Markdown, frontmatter, helpers |
| 3 | fabryk-fts | Full-text search |

**Still in music-theory:**

| Component | Reason |
|-----------|--------|
| Graph building | Phase 4 (highest risk) |
| MCP server | Phase 5 |
| CLI | Phase 6 |
| Domain-specific content | Stays in domain |

**Validation results:**

- [x] Core compilation: ✅
- [x] Content parsing: ✅
- [x] Search indexing: ✅
- [x] Search queries: ✅
- [x] Music-theory integration: ✅
- [x] MCP tools (non-graph): ✅

## Lessons Learned for Phase 4

### What worked well

1. **Domain-agnostic traits**: `DocumentExtractor` allows domain integration
   without path coupling
2. **Feature flags**: `fts-tantivy` keeps dependencies optional
3. **Default schema**: Amendment §2d was correct — schema is domain-agnostic
4. **Checkpoint approach**: Early validation before highest-risk phase

### Watch out for in Phase 4

1. **Graph extractor trait design**: Most complex abstraction
2. **Relationship enum vs trait**: Decision made (enum with Custom), verify it works
3. **Manual edges support**: Need to preserve human-curated edges
4. **Persistence format**: rkyv caching needs careful design
5. **Algorithm genericity**: Ensure graph algorithms work with any domain

## Next Steps

Phase 3 complete. v0.1-alpha checkpoint achieved. Proceed to Phase 4:

- **Milestone 4.1**: fabryk-graph crate scaffold
- **Milestone 4.2**: Graph types (`Node`, `Edge`, `Relationship` enum)
- **Milestone 4.3**: `GraphExtractor` trait
- **Milestone 4.4**: Graph builder with manual edges support
- **Milestone 4.5**: Algorithms (prerequisites, paths)
- **Milestone 4.6**: Persistence (rkyv cache)
- **Milestone 4.7**: Phase 4 completion

Phase 4 is the **highest risk** phase — take time, test thoroughly.

## Commit Message

```
chore(fabryk): complete Phase 3 and v0.1-alpha checkpoint

Phases 1-3 complete. v0.1-alpha checkpoint achieved:

fabryk-core:
- Error types with backtrace support
- File/path utilities
- ConfigProvider trait
- Generic AppState<C>

fabryk-content:
- Frontmatter extraction (generic)
- Markdown parsing
- Content helpers (extract_list_from_section)

fabryk-fts:
- Default 14-field schema (Amendment §2d)
- Tantivy search backend
- DocumentExtractor trait for domain integration
- Query building with weighted fields

Music-theory integration validated:
- 10 non-graph MCP tools working with Fabryk
- Graph still in-repo (Phase 4)

Ref: Doc 0013 Phase 3, Amendment §2e

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

## Appendix: fabryk-fts API Summary

### Types (always available)

```rust
pub enum QueryMode { Smart, And, Or, MinimumMatch }
pub struct SearchConfig { ... }
pub struct SearchParams { query, limit, category, source, content_types, ... }
pub struct SearchResult { id, title, description, category, relevance, snippet, ... }
pub struct SearchResults { items, total, backend }
pub struct SearchDocument { id, title, content, category, ... }
```

### Backend trait

```rust
#[async_trait]
pub trait SearchBackend: Send + Sync {
    async fn search(&self, params: SearchParams) -> Result<SearchResults>;
    fn name(&self) -> &str;
}

pub async fn create_search_backend(config: &SearchConfig) -> Result<Box<dyn SearchBackend>>;
```

### Document extraction (for indexing)

```rust
#[async_trait]
pub trait DocumentExtractor: Send + Sync {
    async fn extract(&self, path: &Path) -> Result<Option<SearchDocument>>;
    fn extensions(&self) -> &[&str];
}
```

### Tantivy-specific (requires `fts-tantivy` feature)

```rust
pub struct SearchSchema { ... }
pub struct StopwordFilter { ... }
pub struct Indexer { ... }
pub struct IndexBuilder { ... }
pub struct IndexMetadata { ... }
pub struct QueryBuilder<'a> { ... }
pub struct TantivySearch { ... }
```

### Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `fabryk-core` | workspace | Error types, Result |
| `async-trait` | workspace | Async traits |
| `serde` | workspace | Serialization |
| `tantivy` | workspace | FTS engine (optional) |
| `stop-words` | workspace | Stopword list (optional) |
| `async-walkdir` | workspace | File discovery |
| `futures` | workspace | Async utilities |
| `chrono` | workspace | Timestamps |
