# Historical criteria — Fabryk Phase 1

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 61-67; full original recoverable from import commit 2d006a7. -->

**Phase 1 exit criteria:**

- `fabryk-core` compiles and all tests pass
- ai-music-theory depends on `fabryk-core` for errors, utilities, paths, state
- No duplicate code between repos for extracted files
- `cargo clippy` clean in both repos

