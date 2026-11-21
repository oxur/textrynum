# Textrynum historical planning

Planning lives on the orphan `planning` branch at `.worktrees/planning`.
This is a reconstruction of old documents, authorized on 2026-11-21, to make
historical work searchable in the collaboration framework's expected locations.
It is not an audit of the implementation, a new engineering plan, or acceptance
of claims made by the old documents.

## Release and project map

| Release branch | Baseline | Tags grouped on the branch | Historical project |
| --- | --- | --- | --- |
| `release/0.1.x` | `b134862` | `0.1.0`, `0.1.1`, `0.1.2`; ancestral `fabryk-v0.1-alpha` | [ECL and Fabryk foundations](project01-ecl-fabryk-foundations/project-plan.md) |
| `release/0.2.x` | `bffd745` | `0.2.0` | [Fabryk consolidation and lifecycle](project02-fabryk-consolidation/project-plan.md) |
| `release/0.3.x` | `0d5ce41` | `0.3.0`, `v0.3.0` (same commit) | [MCP guidance and resources](project03-mcp-guidance-resources/project-plan.md) |
| `release/0.4.x` | `c481981` | `0.4.0`, `0.4.1`, `0.4.2` | [ECL pipeline engine](project04-ecl-pipeline-engine/project-plan.md) |
| `release/0.5.x` | `856af87` | No 0.5 tag exists in the inspected local repository | [Discoverability and 0.5.0 foundation](project05-discoverability/project-plan.md) |

Each release worktree is `.worktrees/release-0.N.x`. Release branches preserve
their pinned implementation snapshots. `release/0.1.x` was advanced from its
ancestor `2d32858` to `0.1.2`; no existing branch history or tags were discarded.
`release/0.5.x` is the exact `main` snapshot inspected for this migration.
All five release branches are now published to github, codeberg and macpro;
all 15 remote tips were read back successfully. Release/0.5.x subsequently
fast-forwarded to `92c72d7` for planning relocation and AGENTS policies. The
remaining hosted purge is explicitly deferred and nonblocking.

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

The imported sources, including ignored workbench files and superseded copies,
were committed before reorganization as `2d006a7`. For example:

```sh
git show 2d006a7:workbench/TODO.md
git show 2d006a7:crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md
```

This Git history preserves the originals without maintaining a parallel archive
in the current planning tree. The source checkout cleanup is isolated on
`codex/planning-migration`; the release branches and `main` remain pinned.
The 11 distinct post-0.4 sources are now assigned to projects05, 07 and 08.
Their original bytes remain in the import history; lower-precedence copies are
not retained as competing current documents.
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

138 selected source documents are represented in the current projects; 76
lower-precedence copies are retained in import history and mapped to preferred
versions. No original migration source remains unassigned.

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

## Current project split — 2026-11-21

| Project | Responsibility | Current execution status |
| --- | --- | --- |
| [05 — Discoverability](project05-discoverability/project-plan.md) | Solid 0.5.0: audit and repairs first, reusable assembly/startup, consolidation, thin servers; vector improvements tentative | Planning; audit not executed |
| [06 — Pipeline](project06-pipeline/project-plan.md) | End-to-end ingestion, CLI completion, remaining data pipelines, required authorization/multi-tenancy | Deferred |
| [07 — Live ingest](project07-live-ingest/project-plan.md) | Sygnum and event-driven ingestion | Deferred |
| [08 — Reasoning](project08-reasoning/project-plan.md) | Medys and reasoning tools | Deferred |

Concept-card development is owned by ai-engineering. The original agent-workflow
framework is explicitly cancelled in favor of project06. Historical source text
remains evidence and does not override these current scope decisions.
All future project05 audit results must be written to
`project05-discoverability/artifacts/`, per Duncan's explicit instruction.
The complete audit plan is
[arc01-foundation-audit/arc-plan.md](project05-discoverability/arc01-foundation-audit/arc-plan.md).

## Privacy rewrite — 2026-11-21

Duncan required anonymization of a private customer across current source,
fixtures, filenames and Git history. Acme Inc. and anonymous code/path identifiers
replace that identity. The import commit, source revision, content hashes and
migration headers now refer to sanitized history. Historical byte-preservation
statements above describe the initial migration; privacy sanitization explicitly
supersedes the original sensitive bytes. The current source map is reconciled
against the sanitized import and includes a commit-identity translation map.
No unsanitized archive or backup branch is retained.

The [privacy rewrite record](project05-discoverability/artifacts/privacy-rewrite-record.md) distinguishes completed remote ref replacement from the still-pending host-side purge.

## Follow-up execution gates and ChatGPT UX — 2026-11-21

Macpro's host-side purge is verified complete. GitHub and Codeberg still require
provider-side cleanup. Duncan initially chose waiting for both, then explicitly
accepted cleanup so far and directed continuation with that task **open and
nonblocking**. AGENTS policy edits, source-cleanup FF into release/0.5.x and
release-branch pushes can proceed. No repository deletion/recreation is authorized.
All five release branches are to be pushed to all three remotes when ready.
No further privacy-history rewrites are planned; later planning changes are
ordinary new commits.

Before audit/feature development: get fresh CI passing, update all dependencies
to latest stable, then get checks green again. Duncan re-enabled CI and a fresh
read-only baseline run completed with dependency-check and release-build failures
(recorded in arc00). The planned code phase uses Autonomous
Mode with CDC 6.1 Sol High and CC 6 Luna High; the earlier cleanup hold is lifted for the bounded CI/dependency arc. See
[arc00-ci-dependencies](project05-discoverability/arc00-ci-dependencies/arc-plan.md).

ChatGPT UX is primary project05 scope and release acceptance. Its goal, task-level
measures, discovery/request/payload/information tradeoffs and real-client evidence
are recorded in the project plan and audit plan, with post-audit implementation
owned by the ChatGPT UX/reusable-server readiness work.


## Current execution handoff — 2026-11-21

Requested AGENTS changes, source-cleanup FF and all release-branch pushes are
complete; evidence and full branch IDs are in the privacy record. Arc00 is the
bounded active implementation scope, using source branch `codex/ci-dependencies`
at `92c72d7`, source checkout `/Users/oubiwann/lab/oxur/textrynum`, and the separate
planning worktree. CDC 6.1 Sol High coordinates/reviews CC 6 Luna High. Hosted purge
stays open and nonblocking. The full audit and project06/07 features are unopened.
