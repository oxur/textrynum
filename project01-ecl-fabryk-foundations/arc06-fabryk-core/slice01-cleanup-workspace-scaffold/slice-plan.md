# CC Prompt: Fabryk 1.0 & 1.1 — Cleanup & Workspace Scaffold

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Prepare the ECL workspace for Fabryk extraction by:

1. Removing crates that are not part of the extraction plan
2. Gutting existing stubs to empty shells
3. Updating the workspace `Cargo.toml` with new structure and dependencies


<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Create the remaining Fabryk crate stubs so that subsequent milestones can focus
on extraction without scaffolding overhead:

- Create 6 new crate stubs: `fabryk-content`, `fabryk-fts`, `fabryk-graph`,
  `fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph`
- Update all Fabryk crate `Cargo.toml` files with proper dependencies
- Create minimal `README.md` for each crate
- Verify: `cargo check --workspace` succeeds

