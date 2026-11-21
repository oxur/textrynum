# CC Prompt: Fabryk 2.1 — Frontmatter Extraction — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-content/src/markdown/mod.rs` created with module structure
- [ ] `fabryk-content/src/markdown/frontmatter.rs` with generic parsing
- [ ] `FrontmatterResult` type with:
  - `has_frontmatter()` - check if valid frontmatter found
  - `had_delimiters()` - check if delimiters were present
  - `value()` - get raw `serde_yaml::Value`
  - `body()` - get body content after frontmatter
  - `deserialize<T>()` - deserialize into domain-specific type
  - `get_str()` - convenience accessor for string fields
  - `get_string_list()` - convenience accessor for string lists
- [ ] `extract_frontmatter()` function returns `Result<FrontmatterResult>`
- [ ] `strip_frontmatter()` convenience function
- [ ] Test coverage for:
  - Valid frontmatter extraction
  - No frontmatter handling
  - Empty frontmatter
  - Invalid YAML graceful degradation
  - Nested YAML structures
  - Unicode content
  - Edge cases
- [ ] `cargo test -p fabryk-content` passes
- [ ] `cargo clippy -p fabryk-content -- -D warnings` clean

