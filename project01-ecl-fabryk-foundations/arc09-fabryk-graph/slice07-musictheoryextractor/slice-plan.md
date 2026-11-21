# CC Prompt: Fabryk 4.7 — MusicTheoryExtractor

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Create `MusicTheoryExtractor` in ai-music-theory that implements `GraphExtractor`:

1. Define `MusicTheoryNodeData` (domain-specific node info)
2. Define `MusicTheoryEdgeData` (domain-specific relationship info)
3. Implement `extract_node()` for music theory frontmatter
4. Implement `extract_edges()` for prerequisites and related concepts
5. Implement `to_graph_node()` and `to_graph_edges()` conversions


<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Verification

Run this comparison to verify the extractor matches current behavior:

```bash
# Build current graph
cargo run -p music-theory-mcp-server -- graph build --output /tmp/old-graph.json

# Build with new extractor (after integration in 4.8)
cargo run -p music-theory-mcp-server -- graph build --output /tmp/new-graph.json

# Compare node and edge counts
jq '.nodes | length' /tmp/old-graph.json /tmp/new-graph.json
jq '.edges | length' /tmp/old-graph.json /tmp/new-graph.json
```

