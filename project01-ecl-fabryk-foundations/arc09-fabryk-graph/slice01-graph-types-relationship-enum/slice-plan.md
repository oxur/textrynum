# CC Prompt: Fabryk 4.1 — Graph Types & Relationship Enum

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Create `fabryk-graph` crate with core graph types:

1. `Node` enum with generic structure
2. `Edge` struct with relationship and weight
3. `Relationship` enum with `Custom(String)` variant (Amendment §2b)
4. `GraphData` for graph storage
5. `EdgeOrigin` enum for edge provenance

