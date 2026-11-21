# CC Prompt: Fabryk 4.4 — Graph Persistence

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Extract graph persistence to `fabryk-graph::persistence`:

1. `save_graph()` - Serialize graph to JSON
2. `load_graph()` - Deserialize graph from JSON
3. `to_petgraph()` - Rebuild petgraph from loaded data
4. Optional rkyv cache with freshness validation

