# Historical criteria — Crate hierarchy verification

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/docs/06-final/0015-restructure-fabryk-crate-hierarchy-for-better-downstream-ux.md, lines 143-149; full original recoverable from import commit 3a39aea. -->

## Verification

1. `cargo check --workspace` — full compilation
2. `cargo test --workspace` — all tests pass
3. Verify `fabryk` facade exposes: `core`, `auth`, `acl`, `content`, `fts`, `graph`, `vector`
4. Verify `fabryk-mcp` facade exposes: all infra symbols (backward compat via `*` re-export) plus `auth`, `content`, `fts`, `graph`, `semantic`
5. Verify `fabryk-mcp-core` compiles independently with `http` feature
