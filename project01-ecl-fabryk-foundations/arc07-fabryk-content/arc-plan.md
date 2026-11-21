# Fabryk Phase 2 — Content

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 70-94. Original bytes are recoverable from import commit 3a39aea. -->

## Phase 2: fabryk-content (1 week)

**Goal:** Extract markdown parsing, frontmatter extraction, and content helper
utilities.

**Crate:** `fabryk-content`
**Dependency level:** 1 (depends on `fabryk-core`)
**Risk:** Low

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 2.1 | Frontmatter extraction | Extract `markdown/frontmatter.rs` to `fabryk-content::markdown`. Functions: `extract_frontmatter()`, `FrontmatterData`. Already fully generic | Frontmatter tests pass in fabryk-content | §4.2 |
| 2.2 | Markdown parser | Extract `markdown/parser.rs` to `fabryk-content::markdown`. Functions: `parse_markdown()`, `extract_first_heading()`. Already fully generic | Parser tests pass in fabryk-content | §4.2 |
| 2.3 | Content helpers | Create `fabryk-content::markdown::helpers`. Implement `extract_list_from_section()` (Amendment §2f-ii). Extract any other generic markdown utilities from metadata/extraction.rs | Helper function tests pass | Amend §2f-ii |
| 2.4 | Music theory integration | Add `fabryk-content` dependency to ai-music-theory. Update imports. Move domain-specific metadata logic (field names, ConceptMetadata type) to music theory's own modules. Remove extracted markdown files from music-theory repo | `cargo test --all-features` passes in both repos | §4.2 |

**Phase 2 exit criteria:**

- `fabryk-content` compiles and all tests pass
- ai-music-theory uses `fabryk-content` for all markdown/frontmatter operations
- Domain-specific metadata types remain in ai-music-theory
- No `MetadataExtractor` trait (deferred per Amendment §2c)

---


## Reconstructed slice locations

- [CC Prompt: Fabryk 2.1 — Frontmatter Extraction](slice01-frontmatter-extraction/slice-plan.md)
- [CC Prompt: Fabryk 2.2 — Markdown Parser](slice02-markdown-parser/slice-plan.md)
- [CC Prompt: Fabryk 2.3 — Content Helpers](slice03-content-helpers/slice-plan.md)
- [CC Prompt: Fabryk 2.4 — Phase 2 Completion & Integration Notes](slice04-phase-2-completion-integration-notes/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
