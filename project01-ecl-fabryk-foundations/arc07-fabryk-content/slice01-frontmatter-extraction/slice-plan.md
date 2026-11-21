# CC Prompt: Fabryk 2.1 — Frontmatter Extraction

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Create `fabryk-content/src/markdown/` module structure
2. Extract generic frontmatter parsing that returns `serde_yaml::Value`
3. Provide helper function to deserialize into any `serde::Deserialize` type
4. Comprehensive test suite for generic parsing
5. Verify: `cargo test -p fabryk-content` passes

