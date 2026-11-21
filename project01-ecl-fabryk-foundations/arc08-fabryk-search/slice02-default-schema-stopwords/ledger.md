# CC Prompt: Fabryk 3.2 — Default Schema & Stopwords — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-fts/src/schema.rs` with `SearchSchema` struct
- [ ] Schema has 14 fields matching Amendment §2d specification
- [ ] `SCHEMA_VERSION` constant for cache invalidation
- [ ] `register_tokenizers()` for English stemming
- [ ] `full_text_fields()` returns fields with boost weights
- [ ] `fabryk-fts/src/stopwords.rs` with `StopwordFilter`
- [ ] Stopword filtering with allowlist support
- [ ] Fallback to original if all terms filtered
- [ ] All tests pass (~26 tests total)
- [ ] `cargo clippy` clean

