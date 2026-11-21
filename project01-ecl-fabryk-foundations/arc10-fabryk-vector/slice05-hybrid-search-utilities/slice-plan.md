# 4.5.5 — Hybrid Search & Utilities

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0040-phase-4.5-fabryk-vector.md, lines 152-167. Original bytes are recoverable from import commit 2d006a7. -->

### 4.5.5 — Hybrid Search & Utilities

Higher-level search combining vector + FTS:

- `reciprocal_rank_fusion(vector_results, fts_results, limit, k)` — standalone RRF implementation (from Taproot, generalized)
- `HybridSearchResult` — unified result type with `source` field ("vector"/"keyword"/"hybrid")
- `is_index_fresh(db_path, content_hash)` — freshness check for vector index
- `quick_summary(backend)` — document count and dimension info
- Factory function: `create_vector_backend(config) -> Result<Box<dyn VectorBackend>>`
- Factory function: `create_embedding_provider(config) -> Result<Box<dyn EmbeddingProvider>>`

**Files:** `src/hybrid.rs`, `src/persistence.rs`, update `src/lib.rs`
**Tests:** RRF merge logic (identical IDs, disjoint sets, score ordering), freshness check, factory functions

---

