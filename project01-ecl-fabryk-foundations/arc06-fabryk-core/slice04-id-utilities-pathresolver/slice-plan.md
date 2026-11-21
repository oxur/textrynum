# CC Prompt: Fabryk 1.4 — ID Utilities & PathResolver

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Create `fabryk-core::util::ids` module with `normalize_id()` function
2. Create `fabryk-core::util::resolver` module with `PathResolver` struct
3. `PathResolver` uses the generic utilities from milestone 1.3
4. Add comprehensive tests
5. Verify: `cargo test -p fabryk-core` passes

