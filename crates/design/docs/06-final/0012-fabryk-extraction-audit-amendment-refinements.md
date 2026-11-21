---
number: 12
title: "Fabryk Extraction Audit: Amendment & Refinements"
author: "any knowledge"
component: All
tags: [change-me]
created: 2026-02-03
updated: 2026-02-03
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk Extraction Audit: Amendment & Refinements

**Date:** 2026-02-03
**Reviewer:** Claude (Opus 4.5), in collaborative review with Duncan McGreggor
**Amends:** Doc 0009 — Fabryk Extraction Audit: Music Theory MCP Server
**Purpose:** Formalise adjustments to the Claude Code audit based on architectural review

---

## 1. Summary

Doc 0009 provides a thorough, file-level extraction audit of the music-theory MCP
server codebase. Its core conclusions are sound: 82% of the code is generic or
parameterisable, the `GraphExtractor` trait is the critical abstraction, and the
phased migration strategy is well-ordered.

This amendment records six refinements identified during architectural review. None
of these invalidate the audit; they sharpen classifications, reduce unnecessary
abstraction, and improve reusability for the second and third domain skills.

All six adjustments were agreed unanimously before this document was drafted.

---

## 2. Adjustments

### 2a. Reclassify `tools/concepts.rs` and `tools/sources.rs`

**Audit classification:** Domain-Specific (D) — stays in ai-music-theory.

**Revised classification:** Parameterized (P) — extract to `fabryk-mcp-content`.

**Rationale:**

The *data structures* in these files are domain-specific (`ConceptInfo`,
`SourceInfo`), but the *tool patterns* are universal. Every skill needs:

- List items of a content type, optionally filtered by category
- Get a specific item by ID
- List source materials and their availability
- Retrieve a chapter from a source

Leaving these as domain-specific means every new skill rewrites the same MCP tool
wiring. Instead, introduce a `ContentItemProvider` trait that parallels
`GraphExtractor`:

```rust
// In fabryk-mcp-content

/// Trait for providing domain-specific content item access.
///
/// Each skill implements this to define how its content items
/// are listed, retrieved, and described via MCP tools.
pub trait ContentItemProvider: Send + Sync {
    /// Summary type returned when listing items.
    /// Music theory: ConceptInfo { id, title, category, source }
    /// Math: TheoremInfo { id, name, area, difficulty }
    type ItemSummary: Serialize + Send + Sync;

    /// Detail type returned when getting a single item.
    /// Music theory: full concept card content
    /// Math: full theorem content with proof sketch
    type ItemDetail: Serialize + Send + Sync;

    /// List all items, optionally filtered by category.
    async fn list_items(
        &self,
        category: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Self::ItemSummary>>;

    /// Get a single item by ID.
    async fn get_item(&self, id: &str) -> Result<Self::ItemDetail>;

    /// List available categories with counts.
    async fn list_categories(&self) -> Result<Vec<CategoryInfo>>;
}

/// Trait for providing source material access.
pub trait SourceProvider: Send + Sync {
    /// Summary type for source listings.
    type SourceSummary: Serialize + Send + Sync;

    /// List all source materials with availability status.
    async fn list_sources(&self) -> Result<Vec<Self::SourceSummary>>;

    /// Get a specific chapter from a source.
    async fn get_chapter(
        &self,
        source_id: &str,
        chapter: &str,
        section: Option<&str>,
    ) -> Result<String>;

    /// List chapters for a source.
    async fn list_chapters(&self, source_id: &str) -> Result<Vec<ChapterInfo>>;

    /// Get filesystem path to source PDF/EPUB.
    async fn get_source_path(&self, source_id: &str) -> Result<PathBuf>;
}
```

The MCP tools in `fabryk-mcp-content` then delegate to these traits. Music theory
implements them with its concept card and source structures; math implements them
with theorem and textbook structures. The tool registration, parameter parsing,
and response formatting are shared.

**Impact on audit Section 2 (Classification Matrix):**

| File | Old | New | Target Crate |
|------|-----|-----|--------------|
| `tools/concepts.rs` | D | P | fabryk-mcp-content |
| `tools/sources.rs` | D | P | fabryk-mcp-content |

**Impact on audit Section 5 (Music Theory Remainder):**

Remove `tools/concepts.rs` and `tools/sources.rs` from the "stays as-is" list.
Add `music_theory_content.rs` (implements `ContentItemProvider`) and
`music_theory_sources.rs` (implements `SourceProvider`) to the "new domain
implementation files" list.

Revised post-extraction ai-music-theory line count: ~2,800 lines (down from ~3,500
in the audit, ~12,000 pre-extraction).

---

### 2b. Resolve the `Relationship` Enum Question

**Audit status:** Left open between two options.

**Decision:** Option 1 — enum with `Custom(String)` variant.

```rust
/// Relationship types for graph edges.
///
/// Common relationships are first-class variants for pattern matching
/// and exhaustive checks. Domain-specific relationships use Custom(String).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Relationship {
    /// A must be understood before B.
    Prerequisite,
    /// Understanding A naturally leads to B.
    LeadsTo,
    /// A and B are conceptually related.
    RelatesTo,
    /// A extends or generalises B.
    Extends,
    /// Source A introduces concept B.
    Introduces,
    /// Source A covers concept B.
    Covers,
    /// A is a source-specific variant of canonical concept B.
    VariantOf,
    /// Domain-specific relationship not covered above.
    Custom(String),
}

impl Relationship {
    /// Default weight for this relationship type.
    ///
    /// Used when the extractor doesn't specify an explicit weight.
    pub fn default_weight(&self) -> f32 {
        match self {
            Self::Prerequisite => 1.0,
            Self::LeadsTo => 1.0,
            Self::Extends => 0.9,
            Self::Introduces => 0.8,
            Self::Covers => 0.8,
            Self::VariantOf => 0.9,
            Self::RelatesTo => 0.7,
            Self::Custom(_) => 0.5,
        }
    }
}
```

**Rationale:**

Option 2 (trait-based `RelationshipType`) would make `Edge` generic over
`R: RelationshipType`, which propagates into `GraphData<R>`, `GraphBuilder<E, R>`,
every algorithm signature, persistence, serialisation, and all MCP tools. This is
a substantial type parameter tax for marginal benefit.

The enum approach gives:

- Pattern matching in algorithms (e.g., "follow only Prerequisite edges")
- Trivial serialisation (serde derives work immediately)
- No generic infection of the graph infrastructure
- `Custom(String)` as an escape hatch for `"implies"`, `"enables"`, or any
  domain-specific relationship

If a future domain genuinely needs compile-time relationship type safety, a
domain-specific enum can be used internally and mapped to `Relationship` variants
(including `Custom`) in the `to_graph_edges` implementation.

**Impact on audit Section 4.4 (fabryk-graph):**

Replace the open question with the decided enum. Remove Option 2 discussion.

---

### 2c. Consolidate `MetadataExtractor` with `GraphExtractor`

**Audit proposal:** New `MetadataExtractor` trait in `fabryk-content`.

**Decision:** Defer `MetadataExtractor` as a standalone trait. Consolidate
metadata extraction into `GraphExtractor` for v0.1.

**Rationale:**

Compare the signatures:

```rust
// MetadataExtractor (proposed)
fn extract(&self, base_path, file_path, frontmatter, content) -> Result<Self::Metadata>;

// GraphExtractor::extract_node (already designed)
fn extract_node(&self, base_path, file_path, frontmatter, content) -> Result<Self::NodeData>;
```

These are the same operation. The `NodeData` associated type *is* the extracted
metadata — it carries domain-specific fields (category, source, difficulty, etc.)
that downstream code needs.

Introducing a separate `MetadataExtractor` creates two problems:

1. **Skill author confusion:** "Do I implement `MetadataExtractor`,
   `GraphExtractor`, or both? What if they disagree?"
2. **Redundant parsing:** Both traits would read the same frontmatter and extract
   overlapping fields.

For v0.1, the `GraphExtractor::NodeData` type serves as the metadata carrier.
If a future use case requires metadata extraction *without* graph building (e.g.,
a skill that uses FTS but not the graph), we can introduce `MetadataExtractor`
at that point — and potentially have `GraphExtractor` provide a blanket
implementation of it.

**Impact on audit Section 4.2 (fabryk-content):**

Remove the `MetadataExtractor` trait definition. `fabryk-content` provides:

- `extract_frontmatter()` — generic frontmatter parsing
- `parse_markdown()` — generic markdown parsing
- Helper utilities (see Adjustment 3b)

The `metadata/extraction.rs` file's domain-specific logic moves to the music
theory `GraphExtractor` and `ContentItemProvider` implementations.

**Impact on audit Section 5 (Music Theory Remainder):**

Remove `music_theory_metadata.rs` from the new files list. Its responsibilities
are absorbed by `music_theory_extractor.rs` (for graph metadata) and
`music_theory_content.rs` (for MCP content tool metadata).

---

### 2d. Defer `SearchSchemaProvider` to v0.2

**Audit proposal:** New `SearchSchemaProvider` trait in `fabryk-fts`.

**Decision:** Ship v0.1 with a sensible default schema. Introduce
`SearchSchemaProvider` in v0.2 only if a domain actually needs custom fields.

**Rationale:**

The current music theory schema fields are:

- **Full-text:** title, description, content
- **Facet:** category, source, tags
- **Stored:** id, path, chapter, part, author, date, content_type, section

These fields aren't music-theory-specific. A math skill searches by title,
content, and category too. A Rust skill has the same needs. The fields are
*knowledge-domain-general* — they describe "a piece of knowledge with metadata,"
not "a music theory concept."

Adding `SearchSchemaProvider` now means:

- Every function in `fabryk-fts` becomes generic over `S: SearchSchemaProvider`
- Skill authors must implement a trait for what is likely identical configuration
- The abstraction exists to solve a problem we don't yet have

For v0.1:

```rust
// fabryk-fts/src/schema.rs

/// Default schema for knowledge items.
///
/// Covers the common fields needed by any knowledge domain.
/// Custom schemas can be added via SearchSchemaProvider (v0.2).
pub fn default_schema() -> Schema {
    let mut builder = Schema::builder();
    // Full-text (with boost weights)
    builder.add_text_field("title", TEXT | STORED);      // boost 3.0
    builder.add_text_field("description", TEXT | STORED); // boost 2.0
    builder.add_text_field("content", TEXT | STORED);     // boost 1.0
    // Facets
    builder.add_text_field("category", STRING | STORED);
    builder.add_text_field("source", STRING | STORED);
    builder.add_text_field("tags", STRING | STORED);
    // Stored metadata
    builder.add_text_field("id", STRING | STORED);
    builder.add_text_field("path", STORED);
    builder.add_text_field("content_type", STRING | STORED);
    // ... etc
    builder.build()
}
```

If a domain genuinely needs custom fields (e.g., a `difficulty` facet for math,
or a `rust_version` facet for Rust), we introduce `SearchSchemaProvider` at that
point. The migration path is clean: add a `with_schema_provider()` builder method
to the FTS backend that falls back to `default_schema()` when not provided.

**Impact on audit Section 4.3 (fabryk-fts):**

- Remove `SearchSchemaProvider` trait definition
- Remove `MusicTheorySchemaProvider` implementation
- `schema.rs` moves to `fabryk-fts` with hardcoded default fields (renamed from
  music-theory-specific to generic names where needed)
- `document.rs` uses the default schema fields directly
- Classification of `schema.rs` changes from P to G (no parameterisation needed)
- Classification of `document.rs` changes from P to G

**Impact on audit Section 5 (Music Theory Remainder):**

Remove `music_theory_schema.rs` from the new files list.

---

### 2e. Establish a v0.1 Checkpoint at Phase 3

**Audit timeline:** 8–12 weeks, linear through 8 phases.

**Refinement:** Insert a formal v0.1-alpha checkpoint after Phase 3.

**Rationale:**

Phases 1–3 (core + content + FTS) deliver a working Fabryk that handles content
loading and full-text search — roughly 60% of the extraction value. This is a
natural milestone:

```
Phase 1: fabryk-core       (1–2 weeks)  ─┐
Phase 2: fabryk-content    (1 week)      ├── v0.1-alpha checkpoint
Phase 3: fabryk-fts        (2 weeks)     ─┘
Phase 4: fabryk-graph      (3 weeks)     ── highest risk, highest value
Phase 5: fabryk-mcp-*      (1–2 weeks)
Phase 6: fabryk-cli        (1 week)
Phase 7: integration       (1 week)
```

At the v0.1-alpha checkpoint:

- Music theory skill works with Fabryk core + content + FTS
- Graph still uses the existing in-repo code (not yet extracted)
- You can validate the extraction pattern before tackling the hardest piece
- If something's wrong with the approach, you find out at week 4, not week 10

**Concrete v0.1-alpha success criteria:**

1. `fabryk-core`, `fabryk-content`, `fabryk-fts` compile and pass tests
2. Music theory MCP server depends on these crates for content + search
3. All 10 non-graph MCP tools work identically
4. Graph tools still work via in-repo code (unchanged)
5. `cargo test --all-features` passes in both repos

---

### 2f. Note Additional Items for fabryk-core and fabryk-content

The audit doesn't explicitly address three utilities that should be part of
the shared infrastructure. Recording them here for the extraction plan.

#### 2f-i. `compute_id()` → `fabryk-core`

The `compute_id(base_path, file_path)` function (strips base path, removes
extension, normalises to kebab-case ID) appears in every `GraphExtractor`
example in the audit. It's called from domain code but contains no domain
logic.

**Decision:** Extract to `fabryk-core::util::ids` as a public utility.

```rust
// fabryk-core/src/util/ids.rs

/// Compute a stable, human-readable ID from a file path.
///
/// Given base_path `/data/concepts` and file_path
/// `/data/concepts/harmony/picardy-third.md`, produces `"picardy-third"`.
///
/// Rules:
/// - Strip base_path prefix
/// - Remove file extension
/// - Use filename (not full relative path) as ID
/// - Normalise to lowercase kebab-case
pub fn compute_id(base_path: &Path, file_path: &Path) -> Result<String> {
    // ...
}
```

Extractors call `fabryk_core::util::compute_id()` rather than reimplementing.

#### 2f-ii. `extract_list_from_section()` → `fabryk-content`

The Math and Rust `GraphExtractor` examples both use
`extract_list_from_section(content, section_heading, keyword)` to parse
relationship lists from markdown body sections. This is a generic markdown
parsing utility.

**Decision:** Extract to `fabryk-content::markdown::helpers` as a public utility.

```rust
// fabryk-content/src/markdown/helpers.rs

/// Extract a list of items from a named section of a markdown document.
///
/// Looks for a heading matching `section_heading`, then within that section
/// finds a list item matching `keyword`, and returns the comma-separated
/// values.
///
/// # Example
///
/// Given markdown:
/// ```markdown
/// ## Related Concepts
///
/// - **Prerequisite**: major-triad, minor-key
/// - **See also**: borrowed-chords
/// ```
///
/// `extract_list_from_section(content, "Related Concepts", "Prerequisite")`
/// returns `vec!["major-triad", "minor-key"]`.
pub fn extract_list_from_section(
    content: &str,
    section_heading: &str,
    keyword: &str,
) -> Vec<String> {
    // ...
}
```

#### 2f-iii. `manual_edges.json` support → `fabryk-graph`

The audit doesn't mention `mcp-server/data/graphs/manual_edges.json`, which
contains human-curated supplementary edges. The graph builder must support
loading additional edges beyond what the `GraphExtractor` produces.

**Decision:** `GraphBuilder` accepts an optional supplementary edge source:

```rust
impl<E: GraphExtractor> GraphBuilder<E> {
    /// Add manually curated edges from a JSON file.
    ///
    /// These edges supplement (and can override) extracted edges.
    /// Useful for human-curated corrections, cross-source links,
    /// or edges that can't be derived from frontmatter.
    pub fn with_manual_edges(mut self, path: &Path) -> Result<Self> {
        let edges: Vec<Edge> = serde_json::from_reader(
            std::fs::File::open(path)?
        )?;
        self.manual_edges = edges;
        Ok(self)
    }
}
```

This keeps the `GraphExtractor` trait focused on automated extraction while
preserving the ability to supplement with hand-curated knowledge.

---

## 3. Revised Classification Summary

Incorporating all adjustments:

| Classification | Count | Percentage | Change from Audit |
|----------------|-------|------------|-------------------|
| Generic (G) | 22 | 48% | +4 (schema.rs, document.rs reclassified) |
| Parameterized (P) | 18 | 39% | +0 (concepts.rs, sources.rs in; schema.rs, document.rs out) |
| Domain-Specific (D) | 1 | 2% | −2 (only config.rs remains) |
| Mixed (M) | 5 | 11% | unchanged |

**Revised extractable percentage: 87%** (up from 82%).

---

## 4. Revised Trait Inventory

Traits to define in Fabryk v0.1:

| Trait | Crate | Purpose | Status |
|-------|-------|---------|--------|
| `GraphExtractor` | fabryk-graph | Node/edge extraction from content | Confirmed (audit Section 3) |
| `ConfigProvider` | fabryk-core | Domain configuration abstraction | Confirmed (audit Section 4.1) |
| `ContentItemProvider` | fabryk-mcp-content | Item listing/retrieval for MCP | **New** (this amendment, 2a) |
| `SourceProvider` | fabryk-mcp-content | Source material access for MCP | **New** (this amendment, 2a) |

Traits deferred to v0.2+:

| Trait | Crate | Purpose | Deferred Because |
|-------|-------|---------|------------------|
| `MetadataExtractor` | fabryk-content | Metadata from frontmatter | Redundant with GraphExtractor (2c) |
| `SearchSchemaProvider` | fabryk-fts | Custom FTS field definitions | Default schema sufficient (2d) |
| `AccessControl` | fabryk-acl | Permission checks | ACL deferred to v0.2/v0.3 |

---

## 5. Revised Migration Phases

```
Phase 1: fabryk-core               (1–2 weeks)
         Extract: error.rs, util/files.rs, util/paths.rs, state.rs, resources/
         New: compute_id() utility
         Verify: ai-music-theory compiles and passes tests

Phase 2: fabryk-content            (1 week)
         Extract: markdown/frontmatter.rs, markdown/parser.rs
         New: extract_list_from_section() helper
         Move domain metadata logic to ai-music-theory
         Verify: ai-music-theory compiles and passes tests

Phase 3: fabryk-fts                (2 weeks)
         Extract: all search/* files
         Use default schema (no SearchSchemaProvider)
         Verify: search tools work identically

     ┌── v0.1-alpha CHECKPOINT ──────────────────────────────────┐
     │ • fabryk-core, fabryk-content, fabryk-fts all working     │
     │ • 10 non-graph MCP tools functional                       │
     │ • Graph still in-repo (unchanged)                         │
     │ • cargo test --all-features passes both repos             │
     └──────────────────────────────────────────────────────────┘

Phase 4: fabryk-graph              (3 weeks)  ← highest risk
         Extract: all graph/* files except parser.rs
         Implement GraphExtractor trait
         Create MusicTheoryExtractor in ai-music-theory
         Add manual_edges.json support
         Verify: graph build produces identical output

Phase 5: fabryk-mcp + fabryk-mcp-* (1–2 weeks)
         Extract: server.rs → fabryk-mcp
         Extract: tools/health.rs → fabryk-mcp
         Extract: tools/guides.rs → fabryk-mcp-content
         Extract: tools/concepts.rs, sources.rs → fabryk-mcp-content (via traits)
         Extract: tools/search.rs → fabryk-mcp-fts
         Extract: tools/graph.rs, graph_query.rs → fabryk-mcp-graph
         Verify: all 25 MCP tools work identically

Phase 6: fabryk-cli                (1 week)
         Extract: main.rs, lib.rs, cli.rs
         Verify: all CLI commands work

Phase 7: Integration & docs        (1 week)
         Full test suite, performance benchmarks, documentation
         Tag fabryk v0.1.0
```

**Estimated total: 10–14 weeks** (side project pace).
**v0.1-alpha at ~4 weeks** (early validation).

---

## 6. Revised Post-Extraction Music Theory Structure

```
ai-music-theory/mcp-server/
├── src/
│   ├── main.rs                     # Entry point, wires Fabryk components
│   ├── lib.rs                      # Re-exports, tool registration
│   ├── config.rs                   # Music theory configuration (unchanged)
│   ├── music_theory_extractor.rs   # impl GraphExtractor
│   ├── music_theory_content.rs     # impl ContentItemProvider
│   ├── music_theory_sources.rs     # impl SourceProvider
│   └── prompts.rs                  # Domain-specific MCP prompts (optional)
├── config/
│   └── default.toml
├── data/
│   └── graphs/
│       └── manual_edges.json       # Human-curated supplementary edges
├── Cargo.toml                      # Depends on fabryk-* crates
└── tests/
    ├── search_qa_integration.rs
    └── tantivy_integration.rs
```

**Estimated post-extraction size:** ~2,800 lines (down from ~12,000).

Domain-specific implementations:

| File | Implements | Approximate Lines |
|------|-----------|-------------------|
| `config.rs` | `ConfigProvider` | 753 (mostly unchanged) |
| `music_theory_extractor.rs` | `GraphExtractor` | ~300 |
| `music_theory_content.rs` | `ContentItemProvider` | ~500 |
| `music_theory_sources.rs` | `SourceProvider` | ~800 |
| `main.rs` + `lib.rs` | Wiring | ~200 |
| `prompts.rs` | Optional | ~50 |

---

## 7. Decisions Log

| # | Decision | Alternatives Considered | Rationale |
|---|----------|------------------------|-----------|
| A1 | Content tools are Parameterized, not Domain-Specific | Leave as D | Pattern is universal; every skill needs list/get |
| A2 | Relationship as enum with Custom(String) | Trait-based RelationshipType | Avoids type parameter infection across graph infra |
| A3 | Defer MetadataExtractor to v0.2+ | Ship in v0.1 | Redundant with GraphExtractor::NodeData |
| A4 | Defer SearchSchemaProvider to v0.2+ | Ship in v0.1 | Default schema covers all known domains |
| A5 | v0.1-alpha checkpoint at Phase 3 | Linear 8-phase execution | Early validation before highest-risk phase |
| A6a | `compute_id()` in fabryk-core | Each extractor reimplements | Zero domain logic, universally needed |
| A6b | `extract_list_from_section()` in fabryk-content | Each extractor reimplements | Generic markdown parsing utility |
| A6c | Manual edges support in GraphBuilder | Extractor handles everything | Separation: automated extraction vs. human curation |

---

## 8. Relationship to Other Documents

| Document | Relationship |
|----------|-------------|
| 0009 — Fabryk Extraction Audit | **Amended by this document** |
| 0009 — Unified Ecosystem Vision v2 | Architectural context (crate structure, trait design) |
| CC Audit Prompt | Generated the audit that this document refines |

When reading the extraction audit (0009), this amendment should be read alongside
it. Where they conflict, this amendment takes precedence.

---

**End of Amendment**
