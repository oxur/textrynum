# 4.5.1 — Core Types & Traits

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0040-phase-4.5-fabryk-vector.md, lines 87-106. Original bytes are recoverable from import commit 2d006a7. -->

### 4.5.1 — Core Types & Traits

Create `fabryk-vector` crate with foundational types:

**Types:**
- `VectorConfig` — backend selection, model name, dimension, paths, enabled flag
- `VectorDocument` — id, text (to embed), category, metadata HashMap
- `EmbeddedDocument` — VectorDocument + `Vec<f32>` embedding
- `VectorSearchParams` — query string, limit, similarity threshold, metadata filters
- `VectorSearchResult` — id, score (0-1), distance, metadata snapshot
- `VectorSearchResults` — items vec, total count, backend name
- `VectorIndexStats` — documents indexed, embedding dimension, content hash, build time

**Traits:**
- `EmbeddingProvider: Send + Sync` — `async fn embed(&self, text: &str) -> Result<Vec<f32>>`, `async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>`, `fn dimension(&self) -> usize`, `fn name(&self) -> &str`
- `VectorBackend: Send + Sync` — `async fn search(&self, params: VectorSearchParams) -> Result<VectorSearchResults>`, `fn name(&self) -> &str`, `fn is_ready(&self) -> bool`, `fn document_count(&self) -> Result<usize>`

**Files:** `Cargo.toml`, `src/lib.rs`, `src/types.rs`, `src/backend.rs`, `src/embedding.rs`
**Tests:** Type construction, serialization, default configs, trait object safety

