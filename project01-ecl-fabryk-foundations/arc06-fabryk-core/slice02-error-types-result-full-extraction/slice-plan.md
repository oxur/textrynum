# CC Prompt: Fabryk 1.2 — Error Types & Result (Full Extraction)

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Expand `fabryk-core/src/error.rs` with full error variants from music-theory
2. Use `thiserror` derive (not custom struct) for cleaner API
3. Add inspector methods: `is_io()`, `is_not_found()`, `is_config()`
4. Add constructor helpers for all variants
5. Add `#[backtrace]` support via thiserror 2.x
6. **Separate MCP concerns**: Document that `to_mcp_error()` goes to `fabryk-mcp`
7. Bring over the extensive test suite
8. Verify: `cargo test -p fabryk-core` passes

