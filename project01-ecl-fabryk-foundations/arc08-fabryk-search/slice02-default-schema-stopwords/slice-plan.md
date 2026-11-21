# CC Prompt: Fabryk 3.2 — Default Schema & Stopwords

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Extract `schema.rs` to `fabryk-fts/src/schema.rs`
2. Extract `stopwords.rs` to `fabryk-fts/src/stopwords.rs`
3. Adapt for Fabryk types (config, errors)
4. Preserve all test coverage
5. Verify: `cargo test -p fabryk-fts --features fts-tantivy` passes

