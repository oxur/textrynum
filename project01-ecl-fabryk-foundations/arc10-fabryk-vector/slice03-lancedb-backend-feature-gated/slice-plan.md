# 4.5.3 — LanceDB Backend (feature-gated)

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0040-phase-4.5-fabryk-vector.md, lines 122-135. Original bytes are recoverable from import commit 2d006a7. -->

### 4.5.3 — LanceDB Backend (feature-gated)

Implement `VectorBackend` for LanceDB:

- `LancedbBackend` — wraps `lancedb::Connection` + an `Arc<dyn EmbeddingProvider>`
- Generic Arrow schema: id (Utf8), text (Utf8), category (Utf8, nullable), metadata (Utf8/JSON), vector (FixedSizeList<Float32>)
- `build()` — async constructor: connect, create/replace table, insert documents
- `search()` — embed query via provider, call `nearest_to`, apply metadata filters via `only_if`, parse results
- Distance-to-score: `1.0 / (1.0 + distance)` (from Taproot)
- Content hash stored in a metadata table/field for freshness checking

**Files:** `src/lancedb.rs`, update `src/lib.rs`
**Tests:** Schema creation, RecordBatch construction, search with filters, empty table, score normalization

