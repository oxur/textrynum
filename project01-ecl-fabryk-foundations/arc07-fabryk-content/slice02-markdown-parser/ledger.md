# CC Prompt: Fabryk 2.2 — Markdown Parser — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-content/src/markdown/parser.rs` created
- [ ] Functions exported:
  - `extract_first_heading(content) -> Option<(HeadingLevel, String)>`
  - `extract_first_paragraph(content, max_chars) -> Option<String>`
  - `extract_text_content(content) -> String`
- [ ] Helper functions:
  - `truncate_text()` - truncate with "..."
  - `normalize_whitespace()` - collapse spaces
- [ ] `HeadingLevel` re-exported from `pulldown_cmark` for API convenience
- [ ] Test coverage for:
  - H1, H2 heading extraction
  - Heading with formatting (bold, italic, code)
  - Heading extraction skipping initial text
  - No heading case
  - Paragraph after heading
  - Paragraph with formatting
  - Paragraph truncation
  - Empty paragraph skipping
  - Text content formatting stripping
  - Code block exclusion
  - Inline code preservation
  - Link text extraction
  - Whitespace normalization
  - Unicode content
  - Edge cases (empty, whitespace-only)
- [ ] `cargo test -p fabryk-content` passes (all tests including milestone 2.1)
- [ ] `cargo clippy -p fabryk-content -- -D warnings` clean

