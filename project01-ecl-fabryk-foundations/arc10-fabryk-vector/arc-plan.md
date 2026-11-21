# Fabryk Phase 4.5 — Vector search

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0040-phase-4.5-fabryk-vector.md, lines 1-86. Original bytes are recoverable from import commit 2d006a7. -->

# Phase 4.5: fabryk-vector — Vector Search Infrastructure

## Context

We completed Phase 4 (fabryk-graph, 134 tests). The next step before MCP tooling (Phase 5) is extracting a generic vector search capability into `fabryk-vector`. This draws from two sources:

1. **Taproot production implementation** (`~/lab/banyan/taproot/crates/taproot-server/src/knowledge/vector.rs`, 786 lines) — battle-tested LanceDB + fastembed integration with hybrid search
2. **Design doc 0013** (`crates/design/dev/0013-lancedb-integration-guide-for-rust-mcp-servers.md`) — reference architecture for LanceDB in Rust MCP servers

Both are domain-specific (hardcoded to Taproot's concept cards). Our goal is a **generic, domain-agnostic** vector crate following the same patterns as fabryk-fts (backend trait) and fabryk-graph (extractor trait).

---

## Evaluation of Existing Implementations

### Taproot vector.rs — Strengths
- Graceful degradation: vector failure doesn't crash server; FTS continues
- Hybrid search via Reciprocal Rank Fusion (RRF), k=60
- Distance-to-score normalization: `1/(1+distance)`
- Docker-friendly model pre-caching CLI (`vectordb get-model`)
- Clean Arrow RecordBatch construction
- `Option<VectorEngine>` pattern — vector is always optional
- Metadata filtering via LanceDB's `only_if` SQL predicates

### Taproot vector.rs — Weaknesses (to fix in fabryk-vector)
- **Hardcoded model**: only "bge-small-en-v1.5" supported
- **No embedding provider trait**: fastembed is directly coupled
- **No incremental updates**: full rebuild from scratch at every startup
- **No freshness checking**: unlike graph's `is_cache_fresh`
- **Domain-specific schema**: hardcoded slug/concept/category/tier columns
- **`unsafe impl Send + Sync`** on VectorEngine — can be designed cleanly
- **Text composition is hardcoded** (`compose_embedding_text`) — not extensible
- **No embedding persistence**: re-embeds all content on every startup
- **Single Mutex for model**: serializes all embedding operations

### Design Doc 0013 — Strengths
- Three-database architecture pattern (Tantivy + petgraph + LanceDB)
- Shows both fastembed (local) and OpenAI (API) embedding approaches
- Batch ingestion pipeline with configurable batch_size
- Incremental upsert support
- Rich metadata schema with `graph_node_ids` cross-references
- Good testing examples

### Design Doc 0013 — Weaknesses (to fix)
- Uses `anyhow` (fabryk uses `fabryk_core::Error`)
- Hardcoded embedding dimensions (384/1536)
- No trait abstraction for embedding providers or vector backends
- No feature gating for optional dependencies
- No freshness checking or cache validation
- `record_batch_to_documents` is verbose/fragile (column-index based)
- Schema is fixed, not extensible per domain
- Missing async considerations (fastembed is sync, needs spawn_blocking)
- **Outdated versions**: lancedb 0.10 / arrow 53 (Taproot uses 0.26 / 57)

---

## Architecture

### Dependency Layer

```
Level 0: fabryk-core
Level 1: fabryk-content
Level 2: fabryk-vector (new) — depends on fabryk-core
         ├─ EmbeddingProvider trait (always available)
         ├─ VectorBackend trait (always available)
         ├─ LancedbBackend (feature: vector-lancedb)
         └─ FastEmbedProvider (feature: vector-fastembed)
```

### Key Design Decisions

1. **Two independent traits**: `EmbeddingProvider` for generating embeddings, `VectorBackend` for storing/searching. These are separate concerns — you might use OpenAI embeddings with LanceDB storage, or fastembed embeddings with a future Qdrant backend.

2. **`VectorDocument`**: A generic, domain-agnostic document type with `id`, `text`, `metadata` HashMap, and optional `embedding` vector. Domains provide text composition via the `EmbeddingProvider::compose_text()` method or by pre-composing the text field.

3. **Feature gating**: LanceDB and fastembed are both optional features, following the fabryk-fts pattern where `fts-tantivy` gates the Tantivy backend.

4. **Freshness via content hash**: Like `fabryk-graph::is_cache_fresh`, support hash-based staleness detection so embeddings can persist across restarts.

5. **No unsafe**: Design VectorEngine to be naturally `Send + Sync` using `Arc<Mutex<>>` for the embedding model and `Arc` for the LanceDB connection.

---

## Implementation Milestones


<!-- Historical source: crates/design/dev/fabryk/0040-phase-4.5-fabryk-vector.md, lines 168-255. Original bytes are recoverable from import commit 2d006a7. -->

## Key Files

| File | Purpose |
|------|---------|
| `crates/fabryk-vector/Cargo.toml` | Manifest with optional lancedb, fastembed, arrow |
| `crates/fabryk-vector/src/lib.rs` | Module declarations and re-exports |
| `crates/fabryk-vector/src/types.rs` | VectorConfig, VectorDocument, EmbeddedDocument, search params/results |
| `crates/fabryk-vector/src/backend.rs` | VectorBackend trait + SimpleVectorBackend fallback |
| `crates/fabryk-vector/src/embedding.rs` | EmbeddingProvider trait + MockEmbeddingProvider |
| `crates/fabryk-vector/src/extractor.rs` | VectorExtractor trait + MockVectorExtractor |
| `crates/fabryk-vector/src/fastembed.rs` | FastEmbedProvider (feature: vector-fastembed) |
| `crates/fabryk-vector/src/lancedb.rs` | LancedbBackend (feature: vector-lancedb) |
| `crates/fabryk-vector/src/builder.rs` | VectorIndexBuilder orchestration |
| `crates/fabryk-vector/src/hybrid.rs` | RRF, HybridSearchResult |
| `crates/fabryk-vector/src/persistence.rs` | Freshness checking, index metadata |

**Reference files:**
- Taproot vector.rs: `~/lab/banyan/taproot/crates/taproot-server/src/knowledge/vector.rs`
- Taproot hybrid search: `~/lab/banyan/taproot/crates/taproot-server/src/tools/knowledge.rs`
- Taproot config: `~/lab/banyan/taproot/crates/taproot-server/src/config.rs`
- Design doc: `crates/design/dev/0013-lancedb-integration-guide-for-rust-mcp-servers.md`
- FTS backend pattern: `crates/fabryk-fts/src/backend.rs`
- Graph extractor pattern: `crates/fabryk-graph/src/extractor.rs`
- Graph builder pattern: `crates/fabryk-graph/src/builder.rs`

---

## Workspace Changes

### New workspace dependencies (Cargo.toml)
```toml
# Vector database
lancedb = "0.26"
arrow-array = "57"
arrow-schema = "57"

# Local embedding generation
fastembed = "4"
```

### New workspace member
```toml
members = [
    # ... existing ...
    "crates/fabryk-vector",
]
```

### Umbrella crate updates (crates/fabryk/Cargo.toml)
```toml
fabryk-vector = { path = "../fabryk-vector", optional = true }

[features]
vector = ["dep:fabryk-vector", "vector-lancedb", "vector-fastembed"]
vector-lancedb = ["fabryk-vector?/vector-lancedb"]
vector-fastembed = ["fabryk-vector?/vector-fastembed"]
full = ["fts", "graph", "vector", "mcp", "cli"]
```

---

## What Is Deferred

These items are explicitly **out of scope** for Phase 4.5 and deferred to later phases:

- **MCP tool integration** (fabryk-mcp-vector) → Phase 5
- **OpenAI/API-based embedding provider** → Phase 5 or 5.5 (config revisit)
- **VectorConfig in fabryk-core ConfigProvider** → Phase 5.5 (config revisit)
- **CLI commands** (model download, index rebuild) → Phase 5
- **Domain-specific extractors** (MusicTheoryVectorExtractor) → Phase 4.7+ in domain repos
- **Chunking strategies** for long documents → future enhancement
- **Index compaction / maintenance** → future enhancement

---

## Verification

After each milestone:
1. `cargo check -p fabryk-vector`
2. `cargo test -p fabryk-vector` (target: 95%+ coverage)
3. `cargo clippy -p fabryk-vector -- -D warnings`
4. `cargo fmt --check -p fabryk-vector`

After Phase 4.5 completion:
5. `cargo test --workspace` — no regressions
6. Verify fabryk umbrella builds with `--features vector`

**Integration test note:** Tests requiring the fastembed model download (~50MB) should be `#[ignore]` by default and run explicitly. Unit tests should use `MockEmbeddingProvider` for deterministic, fast testing.

## Reconstructed slice locations

- [4.5.1 — Core Types & Traits](slice01-core-types-traits/slice-plan.md)
- [4.5.2 — FastEmbed Provider (feature-gated)](slice02-fastembed-provider-feature-gated/slice-plan.md)
- [4.5.3 — LanceDB Backend (feature-gated)](slice03-lancedb-backend-feature-gated/slice-plan.md)
- [4.5.4 — Index Builder](slice04-index-builder/slice-plan.md)
- [4.5.5 — Hybrid Search & Utilities](slice05-hybrid-search-utilities/slice-plan.md)
