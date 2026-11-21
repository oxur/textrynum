# CC Prompt: Fabryk 4.8 — Graph Integration — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] fabryk-graph dependency added to music-theory
- [ ] GraphBuilder<MusicTheoryExtractor> used for graph build
- [ ] All graph imports updated to use fabryk-graph
- [ ] Old graph files removed from music-theory
- [ ] Node count matches baseline exactly
- [ ] Edge count matches baseline exactly
- [ ] `cargo test -p fabryk-graph` passes
- [ ] `cargo test -p music-theory-mcp-server` passes
- [ ] `cargo clippy` clean in both crates
- [ ] All 15 graph MCP tools work identically
- [ ] Graph statistics match baseline

