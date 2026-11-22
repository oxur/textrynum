# Slice12 — static Cargo features and public API consistency

Complete this small, independently reviewable declaration check, then return
ownership. Read declared Cargo features, map public module/export availability,
and compare exact current README entrypoint claims. Source is immutable719;
this is separate permitted work while the earlier SEARCH verification stays
open. Initial assignment:0of5 refinements used;03 remains2of5used/3left.

## Terminology and stop contract

Use accurate ordinary software engineering terms in new prose/prompts—“code quality assessment”, “behavioral check”, or “API consistency review”—when equally precise. Keep necessary technical facts, exact code identifiers and historical records unchanged. Do not construct keyword blacklists, obscure or relabel the work, or use wording/context/model changes to evade platform restrictions; we do not know which words triggered prior flags. If a task is platform-flagged, stop that affected work and report its evidence without variants or retries.

Propagate this paragraph to any later authorized contributor assignment. This
assignment has one CC author and independentCDC; no extra contributor is needed.

## Roles, baseline and ownership

Retained exact CC gpt-6-luna High through localCodexCLI session
01a2019f-73de-7de1-831f-53d55b3da247; CDC gpt-6.1-sol High. Coordinator already
verified requested configuration in your own-session turn_context at
/Users/oubiwann/Library/Application Support/ChatGPT-Profiles/personal/codex-home/sessions/2026/11/21/rollout-2026-11-21T22-59-17-01a2019f-73de-7de1-831f-53d55b3da247.jsonl.
Acknowledge retained receipt; hidden backend unreported. If the retained session
is unavailable, disclose and return rather than silently substitute a model.

Source /Users/oubiwann/lab/oxur/textrynum on codex/foundation-audit remains
719c58f245fe4a9115c7c0391d3e268c99157e2f, READ-ONLY including its index.
Planning /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning is orphanplanning.
Use ABSOLUTE paths for ALL reads/writes and explicit physical planning cwd for
Git mutations; resume tool cwd may retain source despite the launch --cd.

You own ONLY these five paths during this assignment:
1. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-api-feature-consistency.md (new complete matrix/intake/evidence)
2. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-audit-results-rust.md (append this bounded section only)
3. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.21-audit-index.md (one bounded-status/navigation note)
4. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice12-api-feature-consistency/ledger.md
5. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice12-api-feature-consistency/closing-report.md

Perform static file/text/TOML inspection and scoped documentation bookkeeping
only. Preserve source, existing report bytes, coverageJSON, issued prompts and
all SEARCH records/partial files. Do not inspect/reconcile/run/reassign that
blocked driver; metadata-only preservation hashes are allowed. No Cargo/Rust/
build/test/runtime command, service/provider, repair, CI
edit, push, publication, Daybreak, feedback resubmission or external message.
Parent already submitted feedback17c6ecb with confirmed reference
no-active-thread-01a203eb-d26f-73a2-ae62-a9350da65b07; no logs/attachments/chat
association. This task neither retries that operation nor resolves its gates.

## Required reading and readback before edits

Read current arc-planv1.14/ledger, this slice plan/ledger/prompt and current
project/AGENTS instructions. Retain the previously completed full collaboration
framework/project-management wayfinder+operation guides, Autonomous contract,
work-verification, code-auditing guide set, engineering prompt/intake discipline
and methodology-stewardship. Retain full Rust skill/all36existingguides with
anti-patterns first, including all Cargo/public-API/cfg guidance; absentCLI09 is
a known real gap. Re-read relevant uncertain rules, not the whole corpus merely
for a new slice name. Do not substitute hashes/receipts for actual new reading.

Parse all41 member manifests for explicit/implicit optional features. For EACH
of these9members read COMPLETE Cargo.toml and src/lib.rs, including crate docs:
fabryk, fabryk-fts, fabryk-graph, fabryk-vector, fabryk-mcp, fabryk-mcp-core,
fabryk-mcp-fts, fabryk-mcp-graph, fabryk-cli. These18files total1049lines. Read
root /Users/oubiwann/lab/oxur/textrynum/Cargo.toml for workspace inheritance.
Read all7existing memberREADMEs: fabryk150lines, fabryk-mcp125, fabryk-fts56,
fabryk-graph18, fabryk-cli18, fabryk-mcp-fts16, fabryk-mcp-graph16. There is NO
README in fabryk-vector or fabryk-mcp-core; use actual crate docs, not invented
reads. These absences alone are not findings.

Follow immediate source declarations needed for exact availability/name claims:
graph extractor.rs mock gate99 and persistence.rs rkyv_cache172–224; mcp-core
server.rs HttpService30 and http-method cfgs242/248/268; CLI cli.rs74/182/191,
app.rs113 and vectordb_handlers.rs71lines; FTS/graph tool SLOT constants, default
name mapping, tools() registrations and with_names. These are declaration/name
traces, not SEARCH/algorithm behavior assessment. Record exact read ranges; no
whole-file coverage claim for range-only follow-up traces.

Give a compact source-cited readback BEFORE dependent edits: actual population,
relevant Cargo/cfg rules, activation/availability examples, current-vs-historical
documentation, exact outputs/static limit and stop contract.

## Matrix and concrete source-guided questions

1. Map all26explicit feature keys across9members, including9emptydefaults/full
   alias, dep: activation and dependency-feature forwarding. The32othermembers
   have no explicit feature table or optional declaration changing this feature-
   bearing population (independently confirm). Ten optional dependency entries
   are in the selected9. Include workspace-inherited dependency features.
2. Map each feature to public modules/re-exports and immediate gated definitions.
   FtsTools/GraphTools always being available is compatible with forwarding-only
   features. CLIvector-fastembed activates an optional dependency whose manifest
   already selects the leaf vector-fastembed feature. Graphmock cfg(any(test,
   feature=test-utils)) has two intended availability contexts. HTTP facade
   forwarding and leaf-gated exports should align. Empty local defaults do not
   imply all transitive defaults are off or prove compile/runtime behavior.
3. Compare exact README claims with those public definitions. Source leads, not
   preaccepted findings: fabrykREADME89–90 says default search/graph provide only
   traits/types, while their public entries expose more; small MCP READMEs name
   search_items and graph_stats/find_related/find_path, while current slots and
   crate docs use other defaults. Account for with_names overrides; do not claim
   no customization exists. The rkyv feature's public module exposes hashing
   helpers; distinguish entrypoint documentation from later full implementation
   assessment. Do not commission a cache feature or revive cancelled scope.
4. Preserve sound controls: always-available default APIs, aligned optional leaf
   modules, forwarding features, test-utils, HTTP exports and historical0.2
   examples explicitly labelled by mcpREADME. Do not infer registry/publication
   failures or real client usability from source. No finding-count target.

## Deliverable and coverage boundary

The new matrix artifact contains full bounded intake, all26rows/9members, exact
source719file:line links, declaration/activation/export/doc dispositions,
commands/statuses for static bookkeeping, at least5concrete negative checks,
questions and limitations. Interpret declarations manually as needed; a regex
projection alone is not semantic proof.

Append a bounded section to the ONE cumulative Rust report without altering any
existing byte. New supported findings start at unassigned FA-RS-034; preserve
001–033 and their statuses/history. Each new finding needs severity/rationale,
pinnedsource, precise static failure/consumer impact, confidence, repair owner,
fix/issue-specific compatibility choice and validation oracle. No new finding
is also a valid result. Keep hypotheses separate. Detailed finding narratives
belong in that report; the matrix companion links instead of duplicating them.

Coverage unit is26feature declarations/public claims, not18fully assessed code
files or completecrates. Do not change360path owners/statuses,49pass-scale cells
or pendingSEARCHcriteria. Index gets one nonduplicated bounded-result note.
Close/ledger mark author-proposed only; CDC independently checks the matrix and
conclusions. No static-only conclusion becomes a compiled/runtime attestation.

## Privacy, exact commit and handback

Before staging screen exact five owned paths/content in memory with:

```python
import re
pattern = re.compile(bytes([87,97,108,103,114,101,101,110,115]), re.I)
```

Output COUNTS ONLY; only Acme Inc. new examples. Preserve unknown files. Verify
exact26-key set, evidence links, prior report prefix/coverageJSON/SEARCH partial
hashes/source unchanged and scoped diffs. Reconcile semantic IDs/status/claims;
do not build literal-prose assertions as acceptance gates. No new executable
artifact/test or passing-baseline rerun for documentation.

Commit only the five owned paths, ordinary commit with BOTH actual trailers
after blank body, no amend/squash/fixup/drop/merge/push:

Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>

Verify actual paths/trailers/owned clean state. Return exact commit, bounded
source-backed result/questions/limits/privacycounts, source719state and explicit
five-path/planning-index ownership return with no active mutable command.
