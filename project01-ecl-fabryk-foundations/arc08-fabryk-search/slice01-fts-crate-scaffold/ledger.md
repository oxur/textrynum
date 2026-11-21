# CC Prompt: Fabryk 3.1 — FTS Crate Scaffold — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-fts/Cargo.toml` updated with full dependencies
- [ ] Feature flag `fts-tantivy` configured
- [ ] `fabryk-fts/src/lib.rs` with module structure and re-exports
- [ ] `fabryk-fts/src/types.rs` with `QueryMode` and `SearchConfig`
- [ ] `fabryk-fts/src/backend.rs` stub with `SearchBackend` trait
- [ ] `fabryk-fts/src/document.rs` stub with `SearchDocument`
- [ ] `cargo check -p fabryk-fts` passes
- [ ] `cargo check -p fabryk-fts --features fts-tantivy` passes
- [ ] README updated with feature documentation

