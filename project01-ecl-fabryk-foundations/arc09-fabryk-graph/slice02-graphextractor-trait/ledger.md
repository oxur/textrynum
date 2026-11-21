# CC Prompt: Fabryk 4.2 — GraphExtractor Trait — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `GraphExtractor` trait defined with associated types `NodeData` and `EdgeData`
- [ ] `extract_node()` method defined with correct signature
- [ ] `extract_edges()` method returns `Option<EdgeData>` for files without relationships
- [ ] `to_graph_node()` converts domain data to generic `Node`
- [ ] `to_graph_edges()` converts domain data to `Vec<Edge>`
- [ ] Default `content_glob()` returns `"**/*.md"`
- [ ] `MockExtractor` implemented for testing
- [ ] `test-utils` feature gates mock types
- [ ] All tests pass
- [ ] Documentation includes usage examples

