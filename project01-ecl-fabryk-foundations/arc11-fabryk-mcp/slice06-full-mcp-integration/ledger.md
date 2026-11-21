# CC Prompt: Fabryk 5.6 — Full MCP Integration — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] All four fabryk-mcp-* crates added as dependencies
- [ ] `build_tool_registry()` composes all tool sources
- [ ] Extracted tool files removed from music-theory
- [ ] All 25 MCP tools verified via inspector:
  - [ ] concepts_list, concepts_get, concepts_categories
  - [ ] sources_list, sources_chapters, sources_get_chapter
  - [ ] search, search_suggest, search_index_status
  - [ ] graph_related, graph_path, graph_prerequisites
  - [ ] graph_neighborhood, graph_info, graph_validate
  - [ ] graph_centrality, graph_bridges
  - [ ] health
  - [ ] (other tools)
- [ ] `cargo test --all-features` passes in both repos
- [ ] `cargo clippy` clean in all crates

