# Historical criteria — Fabryk Phase 4

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 157-165; full original recoverable from import commit 3a39aea. -->

**Phase 4 exit criteria:**

- `fabryk-graph` compiles and all tests pass
- `GraphExtractor` trait is defined and documented
- `MusicTheoryExtractor` produces identical graph output to pre-extraction
- Graph build: same node count, same edge count, same statistics
- All 15 graph MCP tools work identically
- `cargo test --all-features` passes in both repos

