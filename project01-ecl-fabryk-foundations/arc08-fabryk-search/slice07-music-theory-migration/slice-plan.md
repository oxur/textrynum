# CC Prompt: Fabryk Checkpoint — Music Theory Migration

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Add all Fabryk crates as dependencies to ai-music-theory
2. Implement required traits (`ConfigProvider`, `ContentProvider`, etc.)
3. Update all imports from local modules to Fabryk crates
4. Remove extracted files from the music-theory repo
5. Verify: `cargo test --all-features` passes in **both** repos

