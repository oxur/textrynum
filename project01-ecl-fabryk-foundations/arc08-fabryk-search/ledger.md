# Historical criteria — Fabryk Phase 3

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 111-117; full original recoverable from import commit 3a39aea. -->

**Phase 3 exit criteria:**

- `fabryk-fts` compiles and all tests pass (with `tantivy` feature)
- ai-music-theory uses `fabryk-fts` for all search operations
- Search results identical to pre-extraction (verified by integration tests)
- No `SearchSchemaProvider` trait (deferred per Amendment §2d)

