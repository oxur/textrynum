---
title: "CC Prompt: Fabryk 2.4 — Phase 2 Completion & Integration Notes"
milestone: "2.4"
phase: 2
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["2.1 Frontmatter extraction", "2.2 Markdown parser", "2.3 Content helpers"]
governing-docs: [0011-audit §4.2, 0012-amendment §2e, 0013-project-plan]
---

# CC Prompt: Fabryk 2.4 — Phase 2 Completion & Integration Notes

## Context

This milestone verifies Phase 2 completion and documents the music-theory
integration that will occur at the **v0.1-alpha checkpoint** (after Phase 3).

**Key from Amendment §2e:**

> Insert a formal v0.1-alpha checkpoint after Phase 3. At the checkpoint:
> - Music theory skill works with Fabryk core + content + FTS
> - Graph still uses the existing in-repo code (not yet extracted)
> - You can validate the extraction pattern before tackling the hardest piece

**Music-Theory Migration**: Per the checkpoint-based approach, music-theory
continues using its local copy of all modules until the v0.1-alpha checkpoint.
This milestone documents what that integration will look like but **does not
implement it** — that happens after Phase 3 completion.

## Objective

1. Verify all Phase 2 milestones (2.1-2.3) are complete
2. Document the music-theory integration plan for v0.1-alpha
3. Verify: `cargo test -p fabryk-content` passes with full coverage
4. Prepare for Phase 3 (fabryk-fts)

## Verification Steps

### Step 1: Verify module structure

```bash
cd ~/lab/oxur/ecl/crates/fabryk-content

# Expected structure:
# fabryk-content/src/
# ├── lib.rs
# └── markdown/
#     ├── mod.rs
#     ├── frontmatter.rs  (milestone 2.1)
#     ├── parser.rs       (milestone 2.2)
#     └── helpers.rs      (milestone 2.3)

tree src/
```

### Step 2: Verify public exports

Check `lib.rs` exports:

```rust
// Expected exports (all re-exported at crate root):

// From frontmatter (2.1)
pub use markdown::{extract_frontmatter, strip_frontmatter, FrontmatterResult};

// From parser (2.2)
pub use markdown::{extract_first_heading, extract_first_paragraph, extract_text_content};
pub use pulldown_cmark::HeadingLevel;

// From helpers (2.3)
pub use markdown::{
    extract_all_list_items, extract_list_from_section, extract_section_content,
    normalize_id, parse_comma_list, parse_keyword_list,
};
```

### Step 3: Run tests

```bash
cd ~/lab/oxur/ecl
cargo test -p fabryk-content
```

Expected: All tests pass (frontmatter + parser + helpers).

### Step 4: Run clippy

```bash
cargo clippy -p fabryk-content -- -D warnings
```

Expected: No warnings.

### Step 5: Generate documentation

```bash
cargo doc -p fabryk-content --no-deps --open
```

Review:
- [ ] Crate-level docs present
- [ ] All public types and functions documented
- [ ] Examples in doc comments compile

### Step 6: Verify doc tests

```bash
cargo test --doc -p fabryk-content
```

Expected: All doc tests pass.

## Phase 2 Summary

After this milestone, fabryk-content provides:

| Module | Contents |
|--------|----------|
| `markdown::frontmatter` | `FrontmatterResult`, `extract_frontmatter()`, `strip_frontmatter()` |
| `markdown::parser` | `extract_first_heading()`, `extract_first_paragraph()`, `extract_text_content()` |
| `markdown::helpers` | `extract_list_from_section()`, `extract_section_content()`, `parse_keyword_list()`, `parse_comma_list()`, `extract_all_list_items()`, `normalize_id()` |

**What's NOT in fabryk-content:**

- Domain-specific `Frontmatter` structs (stay in each domain)
- `MetadataExtractor` trait (deferred per Amendment §2c)
- Content type enums (domain-specific)
- Metadata extraction orchestration (domain-specific)

## Music-Theory Integration Plan (v0.1-alpha Checkpoint)

**This integration happens after Phase 3 completion, not now.**

When the v0.1-alpha checkpoint is reached, music-theory will:

### A. Add Fabryk dependencies

```toml
# ai-music-theory/mcp-server/crates/server/Cargo.toml

[dependencies]
fabryk-core = { path = "../../../../oxur/ecl/crates/fabryk-core" }
fabryk-content = { path = "../../../../oxur/ecl/crates/fabryk-content" }
fabryk-fts = { path = "../../../../oxur/ecl/crates/fabryk-fts", features = ["fts-tantivy"] }
```

### B. Replace markdown imports

```rust
// Before (local modules):
use crate::markdown::{extract_frontmatter, extract_first_heading};
use crate::markdown::frontmatter::Frontmatter;

// After (Fabryk + local):
use fabryk_content::{extract_frontmatter, extract_first_heading, FrontmatterResult};
use crate::types::Frontmatter;  // Domain-specific struct stays local
```

### C. Define domain-specific Frontmatter

Music-theory keeps its own `Frontmatter` struct and deserializes from Fabryk's
generic `FrontmatterResult`:

```rust
// ai-music-theory/mcp-server/crates/server/src/types/frontmatter.rs

use serde::Deserialize;

/// Music-theory-specific frontmatter fields.
#[derive(Debug, Clone, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub date: Option<String>,

    // Music-theory-specific fields
    pub concept: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub chapter: Option<String>,
    pub section: Option<String>,
    pub part: Option<String>,
}

impl Frontmatter {
    /// Extract from content using Fabryk's generic parser.
    pub fn extract(content: &str) -> fabryk_core::Result<Option<Self>> {
        let result = fabryk_content::extract_frontmatter(content)?;
        result.deserialize()
    }
}
```

### D. Replace helper usage

```rust
// Before (inline parsing):
fn extract_prerequisites(content: &str) -> Vec<String> {
    // 30+ lines of custom parsing...
}

// After (Fabryk helper):
use fabryk_content::extract_list_from_section;

fn extract_prerequisites(content: &str) -> Vec<String> {
    extract_list_from_section(content, "Related Concepts", "Prerequisite")
}
```

### E. Remove extracted files

After migration, remove from music-theory:

```
mcp-server/crates/server/src/markdown/frontmatter.rs  → use fabryk_content
mcp-server/crates/server/src/markdown/parser.rs       → use fabryk_content
```

Keep in music-theory (domain-specific):

```
mcp-server/crates/server/src/types/frontmatter.rs     # Domain struct
mcp-server/crates/server/src/metadata/extraction.rs   # Orchestration
```

### F. v0.1-alpha success criteria

Per Amendment §2e:

1. `fabryk-core`, `fabryk-content`, `fabryk-fts` compile and pass tests
2. Music theory MCP server depends on these crates for content + search
3. All 10 non-graph MCP tools work identically
4. Graph tools still work via in-repo code (unchanged)
5. `cargo test --all-features` passes in both repos

## Exit Criteria (Phase 2 Completion)

- [ ] fabryk-content module structure matches specification
- [ ] All public exports in lib.rs verified
- [ ] `cargo test -p fabryk-content` passes (all tests)
- [ ] `cargo clippy -p fabryk-content -- -D warnings` clean
- [ ] `cargo doc -p fabryk-content --no-deps` builds without warnings
- [ ] Doc tests pass (`cargo test --doc -p fabryk-content`)
- [ ] Frontmatter parsing returns generic `FrontmatterResult`
- [ ] Parser functions extracted from music-theory
- [ ] Helper functions per Amendment §2f-ii implemented
- [ ] Music-theory integration plan documented (not implemented)

## Next Steps

Phase 2 complete. Proceed to Phase 3:

- **Milestone 3.1**: fabryk-fts crate scaffold with feature flags
- **Milestone 3.2**: Default schema (per Amendment §2d)
- **Milestone 3.3**: Document indexing
- **Milestone 3.4**: Search query API
- **Milestone 3.5**: Tantivy backend integration
- **Milestone 3.6**: Phase 3 completion → **v0.1-alpha checkpoint**

After Phase 3, the v0.1-alpha checkpoint integrates music-theory with
`fabryk-core`, `fabryk-content`, and `fabryk-fts`.

## Commit Message

```
chore(content): verify Phase 2 completion

All Phase 2 milestones (2.1-2.3) complete. fabryk-content provides:
- Generic frontmatter extraction with FrontmatterResult
- Markdown structure parsing (headings, paragraphs, text)
- Content helpers (extract_list_from_section, etc.)

Music-theory integration documented for v0.1-alpha checkpoint.
Integration will occur after Phase 3 (fabryk-fts) completion.

Note: Domain-specific Frontmatter structs stay in each domain.
MetadataExtractor trait deferred per Amendment §2c.

Ref: Doc 0013 Phase 2 completion

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

## Appendix: fabryk-content API Summary

### Frontmatter (Milestone 2.1)

```rust
/// Result of frontmatter extraction.
pub struct FrontmatterResult<'a> {
    // Private fields
}

impl<'a> FrontmatterResult<'a> {
    pub fn has_frontmatter(&self) -> bool;
    pub fn had_delimiters(&self) -> bool;
    pub fn value(&self) -> Option<&serde_yaml::Value>;
    pub fn into_value(self) -> Option<serde_yaml::Value>;
    pub fn body(&self) -> &'a str;
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<Option<T>>;
    pub fn get_str(&self, key: &str) -> Option<&str>;
    pub fn get_string_list(&self, key: &str) -> Vec<String>;
}

pub fn extract_frontmatter(content: &str) -> Result<FrontmatterResult<'_>>;
pub fn strip_frontmatter(content: &str) -> &str;
```

### Parser (Milestone 2.2)

```rust
pub fn extract_first_heading(content: &str) -> Option<(HeadingLevel, String)>;
pub fn extract_first_paragraph(content: &str, max_chars: usize) -> Option<String>;
pub fn extract_text_content(content: &str) -> String;
```

### Helpers (Milestone 2.3)

```rust
pub fn extract_list_from_section(
    content: &str,
    section_heading: &str,
    keyword: &str,
) -> Vec<String>;

pub fn extract_section_content(
    content: &str,
    section_heading: &str,
) -> Option<String>;

pub fn parse_keyword_list(content: &str, keyword: &str) -> Vec<String>;
pub fn parse_comma_list(input: &str) -> Vec<String>;
pub fn extract_all_list_items(content: &str, section_heading: &str) -> Vec<String>;
pub fn normalize_id(id: &str) -> String;
```

### Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `fabryk-core` | workspace | Error types, Result |
| `serde` | workspace | Deserialization |
| `serde_yaml` | workspace | YAML parsing |
| `pulldown-cmark` | workspace | Markdown parsing |
| `regex` | workspace | Content extraction patterns |
| `log` | workspace | Warning logging |
