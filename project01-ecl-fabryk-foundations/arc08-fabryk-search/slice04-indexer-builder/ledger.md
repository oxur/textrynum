# CC Prompt: Fabryk 3.4 — Indexer & Builder — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-fts/src/indexer.rs` with `Indexer` struct
- [ ] `fabryk-fts/src/builder.rs` with `IndexBuilder` and `DocumentExtractor` trait
- [ ] `fabryk-fts/src/freshness.rs` with `IndexMetadata` and hash functions
- [ ] `DocumentExtractor` trait for domain-specific extraction
- [ ] `IndexStats` for tracking indexing results
- [ ] Content hash computation for freshness checking
- [ ] All tests pass (~28 tests)
- [ ] `cargo clippy` clean

