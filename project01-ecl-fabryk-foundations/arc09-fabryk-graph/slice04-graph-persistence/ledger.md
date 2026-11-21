# CC Prompt: Fabryk 4.4 — Graph Persistence — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `save_graph()` serializes graph to JSON
- [ ] `load_graph()` deserializes and rebuilds petgraph
- [ ] `load_graph_from_str()` for non-file sources
- [ ] `is_cache_fresh()` validates cache against content hash
- [ ] `GraphMetadata` captures build timestamp, version, content hash
- [ ] `rkyv-cache` feature gates optional binary caching
- [ ] `compute_content_hash()` and `compute_directory_hash()` (rkyv-cache)
- [ ] Handles edges with missing nodes gracefully
- [ ] All tests pass (including with `--features rkyv-cache`)

