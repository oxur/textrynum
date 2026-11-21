# 4.5.2 — FastEmbed Provider (feature-gated)

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0040-phase-4.5-fabryk-vector.md, lines 107-121. Original bytes are recoverable from import commit 2d006a7. -->

### 4.5.2 — FastEmbed Provider (feature-gated)

Implement `EmbeddingProvider` for fastembed:

- `FastEmbedProvider` wrapping `fastembed::TextEmbedding` in `Arc<Mutex<>>`
- Model selection: map model name string → `fastembed::EmbeddingModel` enum
- Support at minimum: BGESmallENV15 (384d), AllMiniLML6V2 (384d)
- Dimension auto-detection via probe embedding
- `tokio::task::spawn_blocking` for sync fastembed calls
- Cache directory configuration for model files
- `MockEmbeddingProvider` for testing (returns deterministic fixed-dimension vectors)

**Files:** `src/fastembed.rs`, update `src/lib.rs`
**Tests:** Provider creation, embed single, embed batch, dimension probe, mock provider, unknown model error

