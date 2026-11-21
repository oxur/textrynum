# Textrynum historical planning

Planning lives on the orphan `planning` branch at `.worktrees/planning`.
This is a reconstruction of old documents, authorized on 2026-11-21, to make
historical work searchable in the collaboration framework's expected locations.
It is not an audit of the implementation, a new engineering plan, or acceptance
of claims made by the old documents.

## Release and project map

| Release branch | Baseline | Tags grouped on the branch | Historical project |
| --- | --- | --- | --- |
| `release/0.1.x` | `ef30512` | `0.1.0`, `0.1.1`, `0.1.2`; ancestral `fabryk-v0.1-alpha` | [ECL and Fabryk foundations](project01-ecl-fabryk-foundations/project-plan.md) |
| `release/0.2.x` | `0850343` | `0.2.0` | [Fabryk consolidation and lifecycle](project02-fabryk-consolidation/project-plan.md) |
| `release/0.3.x` | `007bc76` | `0.3.0`, `v0.3.0` (same commit) | [MCP guidance and resources](project03-mcp-guidance-resources/project-plan.md) |
| `release/0.4.x` | `1a248c4` | `0.4.0`, `0.4.1`, `0.4.2` | [ECL pipeline engine](project04-ecl-pipeline-engine/project-plan.md) |
| `release/0.5.x` | `f19bb3b` | No 0.5 tag exists in the inspected local repository | Project intentionally deferred for Operator review |

Each release worktree is `.worktrees/release-0.N.x`. Release branches preserve
their pinned implementation snapshots. `release/0.1.x` was advanced from its
ancestor `2d32858` to `0.1.2`; no existing branch history or tags were discarded.
`release/0.5.x` is the exact `main` snapshot inspected for this migration.
These are local changes; no remote publication was requested or performed.

## How the history was reconstructed

Sources were selected in the Operator's precedence order: high-level
`crates/design/docs/*/*.md`, top-level implementation documents in
`crates/design/dev/*.md`, grouped implementation documents in
`crates/design/dev/*/*.md`, then `workbench/*.md`, then `workbench/old/*.md`.
Different versions are resolved by that precedence; complementary reports,
prompts and implementation summaries remain distinct records. In particular,
the six pipeline-crate analysis reports are retained separately from their
similarly titled research prompts. Duplicate bodies were compared directly;
shared titles alone do not establish a duplicate.

Release membership follows adjacent tag ranges and document content/history.
Original workbench modification times are supplementary evidence. They are
recorded as nanoseconds and America/Chicago timestamps, rather than silently
replaced with migration dates. A document title, a release-note date and a
commit date sometimes disagree; those facts are preserved, not harmonized.
For example, the 0.2 release notes use a `0.1.0...0.2.0` scope and therefore
repeat work already present in the separate 0.1 project. Project release
boundaries use the actual adjacent-tag ranges instead.

The original phase and milestone names determine the hierarchy. ECL's initial
phases, Fabryk's extraction phases, the pipeline runner phases and the later
Banyan phases remain distinguishable. Historical future proposals stay marked
as proposals. In particular, ECL Phases 3–5 have no reconstructed detailed slice
documents, and project03 has no inferred arcs or slices. The crate hierarchy
refactor is a direct project-level slice. Numbered directories are navigation,
not a new dependency graph or an execution order.

Original project and arc text was split at Markdown headings outside code
fences. Existing acceptance sections were placed in `ledger.md` where useful;
the original wording and checkbox states remain historical assertions. Existing
CC prompts are preserved intact in slice-root `cc-prompt.md`, with small
source-derived navigation excerpts in the slice plan. No new prompts are issued.
No missing criteria, independent review, completion or verification records are
invented. Supporting reference documents live in the owning project's or arc's
`artifacts/` directory. Original internal paths and links are unchanged.

## Provenance and preservation

[migration-map.json](migration-map.json) records all 214 inspected documents:
original path, SHA-256, size, mtime, tracked history, disposition, preferred
version where applicable, and exact original/destination line ranges for splits.
The map serves recovery and completeness checks; it does not establish code
correctness. Every selected source is covered completely by its recorded
segments, and every lower-precedence copy has an explicit preferred source.

Original input bytes, including ignored workbench files and superseded copies,
were committed before reorganization as `3a39aea`. For example:

```sh
git show 3a39aea:workbench/TODO.md
git show 3a39aea:crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md
```

This Git history preserves the originals without maintaining a parallel archive
in the current planning tree. The source checkout cleanup is isolated on
`codex/planning-migration`; the release branches and `main` remain pinned.
The 11 distinct post-0.4 sources and their lower-precedence versions stay at
their original source paths pending the 0.5 review. No `project05` exists.
The generated design index, the nested `.dustbin/overwritten` document, and
non-Markdown workbench content are outside the requested migration globs and
remain in the source checkout. Existing links in those records are not repaired.

## Reconstruction inventory

| Project | Historical arcs | Historical slices |
| --- | ---: | ---: |
| `project01-ecl-fabryk-foundations` | 14 | 59 |
| `project02-fabryk-consolidation` | 0 | 1 |
| `project03-mcp-guidance-resources` | 0 | 0 |
| `project04-ecl-pipeline-engine` | 9 | 46 |

127 selected source documents are represented in the current projects; 76
lower-precedence copies are retained in the import commit and mapped to their
preferred source; 11 distinct post-0.4 sources await review. Some of the 76
copies belong to those held sources, so they also remain in the source checkout.

## Validation boundary

Validation checks document identity, full split coverage, generated navigation,
duplicate dispositions, held-file preservation, release/tag ancestry, worktree
branches and the orphan planning root. They are self-checks of this migration.
No Rust implementation tests, dependency upgrades or behavioral audits were
performed as part of this organizational work. Historical test counts and
completion statements inside source documents have not been independently
verified.

Migration self-checks on 2026-11-21 passed: all 214 imported originals matched
their SHA-256 and size records; all 127 selected sources had complete,
non-overlapping, exact line coverage at their destinations; 295 generated
navigation links resolved; all 76 duplicate relationships were checked against
the actual bodies and document roles. All five release worktrees were clean
and checked out their named branches; all applicable tags were ancestors of
their release tips, and `release/0.5.x` equaled `main`. The planning branch had
one independent, initially empty root. These are author self-checks.
