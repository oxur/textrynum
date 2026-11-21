# Fabryk Phase 3 — Full-text search

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 95-119. Original bytes are recoverable from import commit 2d006a7. -->

## Phase 3: fabryk-fts (2 weeks)

**Goal:** Extract the full-text search subsystem with a default schema.

**Crate:** `fabryk-fts`
**Dependency level:** 1 (depends on `fabryk-core`)
**Risk:** Low–Medium

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 3.1 | SearchBackend trait & default schema | Extract `search/backend.rs` (SearchBackend trait) and `search/schema.rs`. Use default schema with generic field names — no SearchSchemaProvider trait (Amendment §2d). Rename any music-theory-specific field names to generic equivalents | Backend trait and schema compile | §4.3, Amend §2d |
| 3.2 | Search document & query | Extract `search/document.rs` and `search/query.rs`. Ensure document field access uses the default schema field names | Document and query tests pass | §4.3 |
| 3.3 | Tantivy backend | Extract `search/tantivy_search.rs`, `search/indexer.rs`, `search/builder.rs`. Feature-gate behind `tantivy` feature flag | Tantivy search tests pass with feature flag | §4.3 |
| 3.4 | Supporting modules | Extract `search/simple_search.rs`, `search/stopwords.rs`, `search/freshness.rs` | All search module tests pass | §4.3 |
| 3.5 | Music theory integration | Add `fabryk-fts` dependency to ai-music-theory. Update imports. Remove extracted search files. Verify search results are identical (run search QA integration tests) | `cargo test --all-features` passes, search QA green | §4.3 |

**Phase 3 exit criteria:**

- `fabryk-fts` compiles and all tests pass (with `tantivy` feature)
- ai-music-theory uses `fabryk-fts` for all search operations
- Search results identical to pre-extraction (verified by integration tests)
- No `SearchSchemaProvider` trait (deferred per Amendment §2d)

---


## Reconstructed slice locations

- [CC Prompt: Fabryk 3.1 — FTS Crate Scaffold](slice01-fts-crate-scaffold/slice-plan.md)
- [CC Prompt: Fabryk 3.2 — Default Schema & Stopwords](slice02-default-schema-stopwords/slice-plan.md)
- [CC Prompt: Fabryk 3.3 — SearchBackend Trait & Documents](slice03-searchbackend-trait-documents/slice-plan.md)
- [CC Prompt: Fabryk 3.4 — Indexer & Builder](slice04-indexer-builder/slice-plan.md)
- [CC Prompt: Fabryk 3.5 — Query Builder & TantivySearch](slice05-query-builder-tantivysearch/slice-plan.md)
- [CC Prompt: Fabryk 3.6 — Phase 3 Completion & v0.1-alpha Checkpoint](slice06-phase-3-completion-v0-1-alpha-checkpoint/slice-plan.md)
- [CC Prompt: Fabryk Checkpoint — Music Theory Migration](slice07-music-theory-migration/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
