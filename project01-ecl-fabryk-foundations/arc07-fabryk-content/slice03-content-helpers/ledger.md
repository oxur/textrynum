# CC Prompt: Fabryk 2.3 — Content Helpers — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-content/src/markdown/helpers.rs` created
- [ ] Functions exported:
  - `extract_list_from_section(content, section_heading, keyword) -> Vec<String>`
  - `extract_section_content(content, section_heading) -> Option<String>`
  - `parse_keyword_list(content, keyword) -> Vec<String>`
  - `parse_comma_list(input) -> Vec<String>`
  - `extract_all_list_items(content, section_heading) -> Vec<String>`
  - `normalize_id(id) -> String`
- [ ] Test coverage for:
  - Basic list extraction
  - Case-insensitive matching
  - Missing section handling
  - Missing keyword handling
  - Multiple sections
  - Section content extraction
  - Subsection handling
  - Keyword list parsing
  - Comma/semicolon/and/or separators
  - Real-world examples (music theory, math, rust)
- [ ] `cargo test -p fabryk-content` passes (all tests including milestones 2.1-2.2)
- [ ] `cargo clippy -p fabryk-content -- -D warnings` clean

