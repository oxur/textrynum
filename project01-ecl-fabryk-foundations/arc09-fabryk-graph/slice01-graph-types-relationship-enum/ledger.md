# CC Prompt: Fabryk 4.1 — Graph Types & Relationship Enum — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-graph` crate created with proper Cargo.toml
- [ ] `Relationship` enum defined with 7 common variants + `Custom(String)`
- [ ] `Relationship::default_weight()` returns appropriate weights
- [ ] `Node` struct with builder pattern (`with_category`, `with_source`, etc.)
- [ ] `Edge` struct with builder pattern (`with_weight`, `with_origin`)
- [ ] `EdgeOrigin` enum (Frontmatter, ContentBody, Manual, Inferred)
- [ ] `GraphData` struct with petgraph integration
- [ ] All types derive appropriate traits (Clone, Debug, Serialize, Deserialize)
- [ ] `cargo test -p fabryk-graph` passes
- [ ] `cargo clippy -p fabryk-graph -- -D warnings` clean

