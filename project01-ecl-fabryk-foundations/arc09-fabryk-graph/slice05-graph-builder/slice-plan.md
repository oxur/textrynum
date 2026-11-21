# CC Prompt: Fabryk 4.5 — Graph Builder

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Extract `GraphBuilder<E: GraphExtractor>` to `fabryk-graph::builder`:

1. File discovery using content glob patterns
2. Node extraction via `GraphExtractor::extract_node()`
3. Edge extraction via `GraphExtractor::extract_edges()`
4. Manual edges support via `with_manual_edges()` (Amendment §2f-iii)
5. Progress reporting and error handling options

