# Historical criteria — Fabryk Phase 6

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 208-214; full original recoverable from import commit 2d006a7. -->

**Phase 6 exit criteria:**

- `fabryk-cli` compiles and passes tests
- ai-music-theory uses `fabryk-cli` for all CLI operations
- All CLI commands work: `serve`, `index`, `graph build`, `graph validate`, etc.
- `cargo test --all-features` passes in both repos

