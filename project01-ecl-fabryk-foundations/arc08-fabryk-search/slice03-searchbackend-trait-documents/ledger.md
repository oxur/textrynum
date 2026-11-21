# CC Prompt: Fabryk 3.3 — SearchBackend Trait & Documents — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `SearchBackend` trait with `search()` and `name()` methods
- [ ] `SearchParams` with all filter options
- [ ] `SearchResult`/`SearchResults` response types
- [ ] `SimpleSearch` fallback backend implemented
- [ ] `create_search_backend()` factory function
- [ ] `SearchDocument` with all 14 schema fields
- [ ] `SearchDocumentBuilder` for convenient construction
- [ ] `matches_query()` for substring matching
- [ ] `relevance()` for weighted scoring
- [ ] `extract_snippet()` for context extraction
- [ ] Filter methods: `matches_category()`, `matches_source()`, `matches_content_type()`
- [ ] All tests pass (~23 tests)
- [ ] Works without `fts-tantivy` feature (SimpleSearch only)

