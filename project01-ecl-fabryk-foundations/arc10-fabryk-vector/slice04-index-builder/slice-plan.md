# 4.5.4 — Index Builder

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0040-phase-4.5-fabryk-vector.md, lines 136-151. Original bytes are recoverable from import commit 2d006a7. -->

### 4.5.4 — Index Builder

Orchestrate vectorization and indexing:

- `VectorIndexBuilder` — fluent API mirroring `GraphBuilder` pattern
  - `with_content_path()`, `with_db_path()`, `with_embedding_provider()`, `with_error_handling()`
- Two-phase build: Phase 1 (discover + extract documents), Phase 2 (batch embed + insert)
- `VectorExtractor` trait — domain implementations provide `extract_document(frontmatter, content) -> Result<VectorDocument>` to control text composition
- `MockVectorExtractor` for testing
- `VectorIndexStats` tracking: docs indexed, files processed/skipped, errors, content hash, embedding dimension
- Batch embedding with configurable batch_size (default 64)
- Content hash computation (blake3) for freshness

**Files:** `src/builder.rs`, `src/extractor.rs`, update `src/lib.rs`
**Tests:** Build from mock extractor, batch processing, error handling, content hash computation

