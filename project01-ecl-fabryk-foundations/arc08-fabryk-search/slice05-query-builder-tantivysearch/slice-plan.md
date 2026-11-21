# CC Prompt: Fabryk 3.5 — Query Builder & TantivySearch

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Extract `query.rs` to `fabryk-fts/src/query.rs`
2. Extract `tantivy_search.rs` to `fabryk-fts/src/tantivy_search.rs`
3. Integrate with previously extracted modules
4. Update backend factory to return `TantivySearch`
5. Verify: Full search round-trip works

