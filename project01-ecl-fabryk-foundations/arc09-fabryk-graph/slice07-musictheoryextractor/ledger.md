# CC Prompt: Fabryk 4.7 — MusicTheoryExtractor — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `MusicTheoryNodeData` struct defined with all concept fields
- [ ] `MusicTheoryEdgeData` struct captures all relationship types
- [ ] `extract_node()` parses music theory frontmatter correctly
- [ ] `extract_edges()` handles frontmatter and optional body extraction
- [ ] `to_graph_node()` converts to generic Node with metadata
- [ ] `to_graph_edges()` uses appropriate Relationship variants
- [ ] Variant nodes (source-specific) handled correctly
- [ ] All tests pass
- [ ] Extractor produces same output as current parser.rs

