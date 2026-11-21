# CC Prompt: Fabryk 3.4 — Indexer & Builder

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Extract `indexer.rs` to `fabryk-fts/src/indexer.rs`
2. Extract `builder.rs` to `fabryk-fts/src/builder.rs` (generalized)
3. Extract `freshness.rs` to `fabryk-fts/src/freshness.rs` (generalized)
4. Provide trait/callback pattern for domain-specific document extraction
5. Verify: `cargo test -p fabryk-fts --features fts-tantivy` passes

