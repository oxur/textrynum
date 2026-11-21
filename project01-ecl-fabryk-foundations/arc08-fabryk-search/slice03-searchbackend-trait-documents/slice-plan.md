# CC Prompt: Fabryk 3.3 — SearchBackend Trait & Documents

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Complete `fabryk-fts/src/backend.rs` with full `SearchBackend` trait
2. Complete `fabryk-fts/src/document.rs` with `SearchDocument` implementation
3. Add `SimpleSearch` fallback backend (no Tantivy dependency)
4. Implement backend factory function
5. Verify: `cargo test -p fabryk-fts` passes (without fts-tantivy feature)

