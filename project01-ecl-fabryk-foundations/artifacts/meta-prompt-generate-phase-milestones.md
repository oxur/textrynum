
<!-- Historical source: workbench/old/meta-prompt-generate-phase-milestones.md, lines 1-272. Original bytes are recoverable from import commit 2d006a7. -->

---
title: "Meta-Prompt: Generate Fabryk Phase Milestone Prompts"
purpose: "Feed to Claude Code (Opus) to generate CC prompts for any extraction phase"
author: "Claude (Opus 4.5)"
created: 2026-02-03
usage: "Attach governing docs + this prompt, specify phase number"
---

# Meta-Prompt: Generate Fabryk Phase Milestone Prompts

## Instructions for Claude Code

You are generating Claude Code implementation prompts for Phase {PHASE_NUMBER} of
the Fabryk extraction project. Your task is to produce one detailed `.md` prompt
file per milestone, following the established format and conventions.

**Before generating prompts, you must:**

1. Read all attached governing documents carefully
2. Identify the milestones for the requested phase from Doc 0013 (Project Plan)
3. Cross-reference each milestone with the Extraction Audit (Doc 0011) for
   file-level details
4. Check the Audit Amendment (Doc 0012) for any overrides or refinements
5. Read the actual source files in the music-theory repo to understand the code
   being extracted

---

## Governing Documents (must be attached)

| Document | Purpose |
|----------|---------|
| **0012 — Session Bootstrap** | Context, key decisions, crate structure |
| **0013 — Project Plan Overview** | Milestone breakdown with deliverables and audit refs |
| **0011 — Extraction Audit** | File inventory, classifications (G/P/D/M), trait designs |
| **0012 — Audit Amendment** | Six refinements that override parts of the audit |
| **0009 — Ecosystem Vision v2** | Architecture context, crate responsibilities |
| **git-tracked-files.txt** | Current file listings for both repos |

---

## Phase to Generate

**Phase: {PHASE_NUMBER}**

(Replace `{PHASE_NUMBER}` with 2, 3, 4, 5, 6, or 7 when using this prompt)

---

## Output Format

Generate one markdown file per milestone using this template:

```markdown
---
title: "CC Prompt: Fabryk {PHASE}.{MILESTONE} — {Short Name}"
milestone: "{PHASE}.{MILESTONE}"
phase: {PHASE}
author: "Claude (Opus 4.5)"
created: {TODAY'S DATE}
prerequisites: ["list", "of", "prerequisite", "milestones"]
governing-docs: [relevant doc refs]
---

# CC Prompt: Fabryk {PHASE}.{MILESTONE} — {Short Name}

## Context

{2-3 paragraphs explaining:
- Where this fits in the overall extraction
- What was completed in prior milestones (for this phase)
- The classification (G/P/D/M) and what changes are needed}

## Source Files

{List the specific files being extracted or modified, with paths:}

```
~/lab/music-comp/ai-music-theory/crates/server/src/{path}
```

{Include line counts and key types/functions from the Audit §1 inventory}

## Objective

{Numbered list of concrete deliverables for this milestone}

## Implementation Steps

### Step 1: {Action}

{Detailed instructions with code examples, bash commands, or file snippets}

### Step 2: {Action}

{Continue with specific, actionable steps}

...

### Step N: Verify

{Verification commands: cargo check, cargo test, cargo clippy}

## Exit Criteria

- [ ] {Checkbox item 1}
- [ ] {Checkbox item 2}
- [ ] ...
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` clean

## Commit Message

```
{type}({scope}): {description}

{Body explaining what was done}

Ref: Doc 0013 milestone {PHASE}.{MILESTONE}, Audit §{section}
```

## Notes

{Any additional context, warnings, design decisions, or alternatives to consider}
```

---

## Content Guidelines

### For Each Milestone Prompt, Include:

1. **Actual source file paths** — Read the git-tracked-files.txt and verify paths
2. **Specific code examples** — Show the "before" (current) and "after" (extracted)
   patterns where relevant
3. **Trait implementations** — If the milestone introduces or implements a trait,
   show the full trait definition and example implementation
4. **Import migration** — Show exactly which imports change and to what
5. **Test strategy** — How to verify the extraction worked (unit tests, integration
   tests, manual verification)
6. **Risk flags** — Call out anything that might go wrong or need special attention

### Cross-Reference Requirements:

- Cite Audit sections: "Audit §4.3" for fabryk-fts details
- Cite Amendment sections: "Amendment §2a" for reclassifications
- Cite Project Plan: "Doc 0013 milestone X.Y" for deliverables
- Check Amendment before relying on Audit classifications — Amendment takes precedence

### Key Constraints (apply to ALL prompts):

1. **Music-theory server must remain functional** after every milestone
2. **All 25 MCP tools must work** — no regressions
3. **`cargo test --all-features`** must pass in both repos
4. **No duplicate code** between repos for extracted functionality
5. **Feature flags** for optional dependencies (tantivy, rkyv-cache)

### Phase-Specific Guidance:

**Phase 2 (fabryk-content):**
- Focus on markdown parsing and frontmatter extraction
- No MetadataExtractor trait (deferred per Amendment §2c)
- Add `extract_list_from_section()` helper (Amendment §2f-ii)
- Domain-specific metadata logic stays in ai-music-theory

**Phase 3 (fabryk-fts):**
- Default schema only — no SearchSchemaProvider trait (Amendment §2d)
- Feature-gate Tantivy behind `tantivy` feature flag
- Verify search results are identical pre/post extraction
- This phase ends at the **v0.1-alpha checkpoint**

**Phase 4 (fabryk-graph):**
- **Highest risk phase** — most complex trait design
- `GraphExtractor` trait is the critical abstraction (Audit §3, §9)
- `Relationship` enum with `Custom(String)` variant (Amendment §2b)
- Add `with_manual_edges()` support (Amendment §2f-iii)
- `MusicTheoryExtractor` implementation stays in ai-music-theory
- Verify graph build produces identical node/edge counts

**Phase 5 (fabryk-mcp + fabryk-mcp-*):**
- Four crates: fabryk-mcp, fabryk-mcp-content, fabryk-mcp-fts, fabryk-mcp-graph
- `ContentItemProvider` and `SourceProvider` traits (Amendment §2a)
- Tools reclassified from D to P: concepts.rs, sources.rs
- guides.rs renamed to documents.rs for generality
- Verify all 25 MCP tools via MCP inspector

**Phase 6 (fabryk-cli):**
- Parameterise over `C: ConfigProvider` and `E: GraphExtractor`
- Support domain-specific subcommand registration
- Relatively low risk after Phase 4/5

**Phase 7 (Integration & Docs):**
- No new code extraction — cleanup, testing, benchmarking, documentation
- Performance verification against pre-extraction baselines
- Tag `fabryk v0.1.0`

---

## Before You Begin

Read these source files to understand the extraction targets for Phase {PHASE_NUMBER}:

```bash
# List the files relevant to this phase
# (Adjust based on the phase — this example is for Phase 3)
ls -la ~/lab/music-comp/ai-music-theory/crates/server/src/search/

# Read key files
cat ~/lab/music-comp/ai-music-theory/crates/server/src/search/backend.rs
cat ~/lab/music-comp/ai-music-theory/crates/server/src/search/schema.rs
# ... etc
```

Cross-reference with the Audit §4.{X} section for that phase's crate to understand:
- Which files are Generic (G) — copy as-is
- Which are Parameterized (P) — need trait abstraction or config
- Which are Mixed (M) — need splitting
- Which are Domain-Specific (D) — stay in ai-music-theory

---

## Output

Generate the following files (adjust count based on actual milestones in Doc 0013):

1. `cc-fabryk-{PHASE}.1-{short-name}.md`
2. `cc-fabryk-{PHASE}.2-{short-name}.md`
3. ... (one per milestone)
4. `cc-fabryk-{PHASE}.{N}-{short-name}.md` (integration milestone)

Save all files to `/mnt/user-data/outputs/` and present them when complete.

---

## Example: Generating Phase 2 Prompts

If the phase is 2 (fabryk-content), Doc 0013 shows milestones 2.1–2.4:

| # | Milestone | Description |
|---|-----------|-------------|
| 2.1 | Frontmatter extraction | Extract markdown/frontmatter.rs |
| 2.2 | Markdown parser | Extract markdown/parser.rs |
| 2.3 | Content helpers | Create helpers module, add extract_list_from_section() |
| 2.4 | Music theory integration | Wire fabryk-content into ai-music-theory |

You would generate four files:
- `cc-fabryk-2.1-frontmatter-extraction.md`
- `cc-fabryk-2.2-markdown-parser.md`
- `cc-fabryk-2.3-content-helpers.md`
- `cc-fabryk-2.4-music-theory-integration.md`

Each file follows the template above with specific details from the source code,
audit, and amendment.

---

## Final Checklist Before Submitting Prompts

For each generated prompt, verify:

- [ ] Source file paths are correct (cross-checked with git-tracked-files.txt)
- [ ] Classification matches Audit (with Amendment overrides applied)
- [ ] Prerequisites list is accurate (prior milestones in dependency order)
- [ ] Code examples are syntactically valid Rust
- [ ] Exit criteria are specific and verifiable
- [ ] Commit message follows conventional commits format
- [ ] Audit and Amendment sections are cited where relevant
- [ ] Key constraint reminder included (25 tools must work, tests must pass)

---

**Now generate the prompts for Phase {PHASE_NUMBER}.**
