# CC Prompt: Fabryk 1.3 — File & Path Utilities

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Extract `util/files.rs` into `fabryk-core::util::files` (full)
2. Extract generic parts of `util/paths.rs` into `fabryk-core::util::paths`
3. Set up the `util` module structure in fabryk-core
4. Ensure all tests pass in the new location
5. Verify: `cargo test -p fabryk-core` passes

