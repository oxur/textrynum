---
number: 11
title: "Fabryk Extraction: Session Bootstrap (Phases 1-7)"
author: "Duncan McGreggor"
component: All
tags: [bootstrap, context, planning]
created: 2026-02-03
updated: 2026-02-03
state: Active
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk Extraction: Session Bootstrap (Phases 1-7)

**Purpose:** Cold-start a new Claude session with full context for planning and
executing any phase of the Fabryk extraction from the music-theory MCP server.

**When to use:** Start a new Claude conversation (web, Claude Code, or project)
and provide this document along with the required companion documents listed below.

---

## Required Documents

Attach these documents (in this order) when starting a new session:

| # | Document | Filename | ECL No. | Music Theory No. | Why It's Needed |
|---|----------|----------|---------|------------------|-----------------|
| 1 | **This bootstrap doc** | `fabryk-extraction-session-bootstrap-phases-1-7.md` | dev 0012 | dev 0018 | The prompt and orientation |
| 2 | Project Plan Overview | `fabryk-extraction-project-plan-overview.md` | 0013 | 0013 | Milestone breakdown for all phases |
| 3 | **Extraction Audit** | `fabryk-extraction-audit-music-theory-mcp-server.md` | 0011 | 0009 | File-level inventory, classifications, trait designs, migration phases |
| 4 | **Audit Amendment** | `fabryk-extraction-audit-amendment.md` | 0012 | 0012 | Six refinements that override parts of the audit |
| 5 | **Unified Ecosystem Vision v2** | `unified-ecosystem-vision-ecl-fabryk-skill-framework.md` | 0009 | NA | Architecture, crate structure, component responsibilities |
| 6 | **Git-tracked files** | `git-tracked-files.txt` | NA | NA | Current codebase file listing (regenerate with `git ls-files` before each session) |

**Optional but helpful:**

| Document | When to Include |
|----------|-----------------|
| Phase-specific source files | When planning or executing a specific phase |
| Previous session transcripts | When continuing interrupted work |

---

## The Prompt

Copy everything below the line into your new Claude session, after attaching the
documents listed above.

---

# PROMPT START

You are helping me plan and execute the Fabryk extraction — a project to refactor
the domain-agnostic infrastructure out of a music-theory MCP server into a set of
reusable Rust crates called Fabryk.

## Project Summary

The music-theory MCP server (~12,000 lines of Rust) has grown into a sophisticated
knowledge management system with 25 MCP tools, Tantivy-based full-text search, a
petgraph-based graph database, and content loading infrastructure. We've determined
that ~87% of this code is domain-agnostic and should be extracted into reusable
Fabryk crates, leaving only ~2,800 lines of music-theory-specific code.

## Key Documents (attached)

I've attached the following documents. Please read them carefully:

1. **Extraction Audit (0009):** The comprehensive file-by-file analysis of the
   music-theory MCP server. Contains the classification matrix (Generic /
   Parameterized / Domain-Specific / Mixed), trait designs, crate-by-crate
   extraction plans, dependency analysis, and migration phases.

2. **Audit Amendment (0010):** Six refinements that override specific parts of
   the audit. **Where the audit and amendment conflict, the amendment takes
   precedence.** The key changes are:
   - 2a: `tools/concepts.rs` and `tools/sources.rs` reclassified from
     Domain-Specific to Parameterized (new `ContentItemProvider` and
     `SourceProvider` traits)
   - 2b: `Relationship` type decided as enum with `Custom(String)` variant
   - 2c: `MetadataExtractor` trait deferred (consolidated with `GraphExtractor`)
   - 2d: `SearchSchemaProvider` trait deferred (default schema for v0.1)
   - 2e: v0.1-alpha checkpoint inserted after Phase 3
   - 2f: Three utilities noted for extraction (`compute_id`,
     `extract_list_from_section`, manual edges support)

3. **Unified Ecosystem Vision v2:** The architectural big picture — how Fabryk
   relates to ECL (workflow orchestration) and the Skill Framework. Contains the
   full crate structure diagram and component responsibilities.

4. **Git-tracked files:** The current file listing of the music-theory repo.

## Fabryk Crate Structure

```
fabryk/
├── fabryk-core/           # Types, traits, errors, utilities
├── fabryk-content/        # Markdown parsing, frontmatter, helpers
├── fabryk-fts/            # Full-text search (Tantivy), default schema
├── fabryk-graph/          # Knowledge graph, GraphExtractor trait, algorithms
├── fabryk-acl/            # Placeholder (v0.2/v0.3)
├── fabryk-mcp/            # Core MCP infra (rmcp), health tool
├── fabryk-mcp-content/    # Content + source MCP tools (ContentItemProvider, SourceProvider)
├── fabryk-mcp-fts/        # FTS MCP tools
├── fabryk-mcp-graph/      # Graph MCP tools
└── fabryk-cli/            # CLI framework
```

## Traits to Implement in v0.1

| Trait | Crate | Purpose |
|-------|-------|---------|
| `GraphExtractor` | fabryk-graph | Extract nodes/edges from content files |
| `ConfigProvider` | fabryk-core | Domain configuration abstraction |
| `ContentItemProvider` | fabryk-mcp-content | Item listing/retrieval for MCP tools |
| `SourceProvider` | fabryk-mcp-content | Source material access for MCP tools |

Traits **deferred** to v0.2+: `MetadataExtractor`, `SearchSchemaProvider`, `AccessControl`.

## Migration Phases

```
Phase 1: fabryk-core               (1–2 weeks)
Phase 2: fabryk-content            (1 week)
Phase 3: fabryk-fts                (2 weeks)
     ── v0.1-alpha CHECKPOINT ──
Phase 4: fabryk-graph              (3 weeks) ← highest risk
Phase 5: fabryk-mcp + fabryk-mcp-* (1–2 weeks)
Phase 6: fabryk-cli                (1 week)
Phase 7: Integration & docs        (1 week)
```

## Current Status

[EDIT THIS SECTION before each new session to reflect current progress]

**Completed phases:** None yet — planning phase.
**Current phase:** Pre-Phase 1 (detailed planning).
**Blockers:** None.
**Open questions:** None.

## Key Decisions Already Made

These decisions are final and should not be revisited:

1. **Extract Fabryk from music-theory** (not build from scratch)
2. **Trait-based abstraction** for domain customisation (not configuration-based)
3. **Associated types** on traits (not generic type parameters)
4. **Enum with Custom(String)** for Relationship type (not trait-based)
5. **Default FTS schema** for v0.1 (SearchSchemaProvider deferred)
6. **MetadataExtractor deferred** (GraphExtractor::NodeData carries metadata)
7. **Content tools are Parameterized** (ContentItemProvider + SourceProvider traits)
8. **v0.1-alpha checkpoint** after Phase 3
9. **Filesystem as source of truth** (editable, git-friendly, portable)
10. **ACL is placeholder** in v0.1 (defer to v0.2/v0.3)

## Your Task

[EDIT THIS SECTION to describe what you need from this session]

Example tasks:

- "Create a detailed Phase 1 implementation plan with specific files, types,
  and test strategies"
- "Design the GraphExtractor trait API with full documentation"
- "Write a Claude Code prompt for executing Phase 3"
- "Review my in-progress Phase 2 code and suggest improvements"
- "Help me resolve a design question about [specific topic]"

**My task for this session:**

[Describe your task here]

## Working Agreements

- When referencing the audit, cite section numbers (e.g., "Audit §4.3")
- When referencing the amendment, cite adjustment numbers (e.g., "Amendment §2a")
- Always check the amendment before relying on audit classifications
- Produce artifacts as markdown documents with frontmatter matching the project's
  numbering convention
- For Claude Code prompts, follow the format established in
  `cc-fabryk-extraction-audit-prompt.md`
- When in doubt about a design decision, check the "Key Decisions Already Made"
  list above before proposing alternatives

# PROMPT END

---

## Maintenance Notes

### Keeping This Document Current

After each work session, update the **Current Status** section:

```markdown
## Current Status

**Completed phases:** Phase 1 (fabryk-core), Phase 2 (fabryk-content).
**Current phase:** Phase 3 (fabryk-fts) — schema extraction in progress.
**Blockers:** Need to decide on stopword list approach.
**Open questions:** Should SimpleSearch be feature-gated?
```

### When to Regenerate git-tracked-files.txt

Run `git ls-files > git-tracked-files.txt` in the music-theory repo before
starting any new session. The file listing changes as extraction progresses
(files move between repos).

### When to Update the Decisions List

Add new decisions as they're made during planning or implementation sessions.
Never remove decisions — if one is reversed, note the reversal and the
replacement decision.

### When This Document Becomes Obsolete

This bootstrap is no longer needed when:

- All 7 phases are complete
- Fabryk v0.1.0 is tagged
- Music theory MCP server depends entirely on fabryk-* crates
- A second domain skill has been built using Fabryk

At that point, Fabryk's own README and documentation should be sufficient
onboarding for new sessions.

---

## Appendix: Quick Reference

### MCP Tool → Destination Crate Mapping

| Tool | Current File | Destination |
|------|-------------|-------------|
| `health` | tools/health.rs | fabryk-mcp |
| `list_sources` | tools/sources.rs | fabryk-mcp-content (SourceProvider) |
| `get_source_chapter` | tools/sources.rs | fabryk-mcp-content (SourceProvider) |
| `get_source_pdf_path` | tools/sources.rs | fabryk-mcp-content (SourceProvider) |
| `list_source_chapters` | tools/sources.rs | fabryk-mcp-content (SourceProvider) |
| `list_concepts` | tools/concepts.rs | fabryk-mcp-content (ContentItemProvider) |
| `list_categories` | tools/concepts.rs | fabryk-mcp-content (ContentItemProvider) |
| `get_concept` | tools/concepts.rs | fabryk-mcp-content (ContentItemProvider) |
| `search_concepts` | tools/search.rs | fabryk-mcp-fts |
| `list_guides` | tools/guides.rs | fabryk-mcp-content |
| `get_guide` | tools/guides.rs | fabryk-mcp-content |
| `graph_status` | tools/graph.rs | fabryk-mcp-graph |
| `graph_stats` | tools/graph.rs | fabryk-mcp-graph |
| `graph_validate` | tools/graph.rs | fabryk-mcp-graph |
| `get_node` | tools/graph.rs | fabryk-mcp-graph |
| `get_node_edges` | tools/graph.rs | fabryk-mcp-graph |
| `get_related_concepts` | tools/graph_query.rs | fabryk-mcp-graph |
| `find_concept_path` | tools/graph_query.rs | fabryk-mcp-graph |
| `get_prerequisites` | tools/graph_query.rs | fabryk-mcp-graph |
| `get_concept_neighborhood` | tools/graph_query.rs | fabryk-mcp-graph |
| `get_dependents` | tools/graph_query.rs | fabryk-mcp-graph |
| `get_central_concepts` | tools/graph_query.rs | fabryk-mcp-graph |
| `get_concept_sources` | tools/graph_query.rs | fabryk-mcp-graph |
| `get_concept_variants` | tools/graph_query.rs | fabryk-mcp-graph |
| `find_bridge_concepts` | tools/graph_query.rs | fabryk-mcp-graph |
| `get_source_coverage` | tools/graph_query.rs | fabryk-mcp-graph |

### Source File → Destination Crate Mapping

| Source File | Destination | Classification |
|------------|-------------|----------------|
| error.rs | fabryk-core | G |
| util/files.rs | fabryk-core | G |
| util/paths.rs | fabryk-core | P |
| state.rs | fabryk-core | P |
| resources/mod.rs | fabryk-core | G |
| markdown/frontmatter.rs | fabryk-content | G |
| markdown/parser.rs | fabryk-content | G |
| metadata/extraction.rs | ai-music-theory | P (domain logic stays) |
| search/* (all 10 files) | fabryk-fts | G/P |
| graph/types.rs | fabryk-graph | P |
| graph/builder.rs | fabryk-graph | P |
| graph/algorithms.rs | fabryk-graph | G |
| graph/persistence.rs | fabryk-graph | G |
| graph/loader.rs | fabryk-graph | P |
| graph/query.rs | fabryk-graph | G |
| graph/stats.rs | fabryk-graph | G |
| graph/validation.rs | fabryk-graph | G |
| graph/parser.rs | **SPLIT** | M (trait → fabryk-graph, impl → ai-music-theory) |
| graph/cli.rs | fabryk-cli | P |
| server.rs | fabryk-mcp | P |
| config.rs | ai-music-theory | D |
| tools/health.rs | fabryk-mcp | G |
| tools/concepts.rs | fabryk-mcp-content | P |
| tools/sources.rs | fabryk-mcp-content | P |
| tools/guides.rs | fabryk-mcp-content | G |
| tools/search.rs | fabryk-mcp-fts | P |
| tools/graph.rs | fabryk-mcp-graph | P |
| tools/graph_query.rs | fabryk-mcp-graph | P |

### Dependency Hierarchy

```
Level 0: fabryk-core
Level 1: fabryk-content, fabryk-fts
Level 2: fabryk-graph, fabryk-mcp
Level 3: fabryk-mcp-content, fabryk-mcp-fts, fabryk-mcp-graph, fabryk-cli
Level 4: ai-music-theory (domain skill)
```
