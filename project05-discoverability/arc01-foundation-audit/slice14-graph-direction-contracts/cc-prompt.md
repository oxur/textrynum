# Slice14 — graph direction contract across public APIs

Read the prerequisite relationship producers and consumers, map exact edge
orientation, and assess whether the public queries compose consistently. This
is a separate bounded code/architecture consistency check, not a documentation-
only sweep. Source719 is read-only. Initial0/5 refinements;03 remains2/5used.

## Terminology and stop contract

Use accurate ordinary software engineering terms in new prose/prompts—“code quality assessment”, “behavioral check”, or “API consistency review”—when equally precise. Keep necessary technical facts, exact code identifiers and historical records unchanged. Do not construct keyword blacklists, obscure or relabel the work, or use wording/context/model changes to evade platform restrictions; we do not know which words triggered prior flags. If a task is platform-flagged, stop that affected work and report its evidence without variants or retries.

Propagate this paragraph to any later authorized contributor assignment.

## Roles, ownership and method

Same retained CC gpt-6-luna High session01a2019f-73de-7de1-831f-53d55b3da247;
CDC gpt-6.1-sol High independently reviews. Acknowledge the already checked
requested-model receipt; hidden backend unreported. Disclose unavailable session
without substitution. Source /Users/oubiwann/lab/oxur/textrynum is read-only,
including index, at719c58f245fe4a9115c7c0391d3e268c99157e2f, codex/foundation-audit.
Planning /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning is orphanplanning.
Use ABSOLUTE reads/writes and explicit physical planning cwd/Git -C; source is
that exact root, NOT a guessed .worktrees/source directory.

You own ONLY these five paths/index during the assignment:
1. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-graph-direction-contracts.md
2. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-audit-results-rust.md
3. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.21-audit-index.md
4. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice14-graph-direction-contracts/ledger.md
5. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice14-graph-direction-contracts/closing-report.md


Static source/manual reasoning and scoped Markdown bookkeeping only. No Rust/
Cargo/build/test/runtime, fixture execution or source repair is assigned. Keep
SEARCH operation/driver/partial files untouched and uninspected except preservation
hashes; no reconciliation/reassignment/retry. CoverageJSON and original full-pass
criteria remain pending. No provider/service, CI edit, push/publication, Daybreak
or external message. Final independent arc composition remains pending.

## Required reading and readback before edits

Read current arc-plan/ledger, this slice plan/ledger/prompt and current applicable
AGENTS/project instructions. Retain unchanged full framework, PM wayfinder/
operation guides, Autonomous contract, work-verification, code-auditing full set,
engineering prompt/intake and methodology-stewardship; complete Rust skill/all36
existing guides with anti-patterns first, API/error/data-model/collection guidance.
CLI09 missing is known. Re-read any relevant uncertain rule; no corpus restart
merely for a new slice name or hash-only substitute for new source reading.

Read COMPLETE files under ABSOLUTE source root:
- crates/fabryk-graph/src/types.rs969lines
- crates/fabryk-graph/src/algorithms.rs1377
- crates/fabryk-graph/src/concept_card_extractor.rs550
- crates/fabryk-graph/src/extractor.rs374
- crates/fabryk-graph/src/builder.rs830
- crates/fabryk-mcp-graph/src/tools.rs2100
- crates/fabryk-graph/src/lib.rs75
- crates/fabryk-mcp-graph/src/lib.rs46
Eight full files6321lines; follow graph persistence.rs1–145 for serialized edge
carriage only. Record actual read extents/recover truncations. No full semantic
acceptance of unrelated algorithms/implementation follows from this intake.

Before edits give compact source-cited readback: exact producer/store/consumer
chain, governing graph/API rules, opposite source tests, manual-versus-executed
limits, outputs/ownership/stop contract. CDC's own current ranges are in plan;
do not rely on the unavailable fresh reviewer's historical intake as your read.

## Source-grounded questions and deliverable

1. Relationship::Prerequisite says A understood before B (types38). Concept-card
   and mock extractors emit current→listed prerequisite (concept203–207,
   extractor210–216). GraphData add_edge preserves from→to (types475–489), and
   builder phase2 inserts extracted edges unchanged (247–273). Map current code
   and claims without silently choosing which convention should become canonical.
2. prerequisites_sorted traverses Incoming (algorithms278–299), free dependents
   also Incoming despite advertising the reverse question (415–457), direct
   GraphData::prerequisites uses Outgoing and direct dependents Incoming (658–678).
   get_related's direction-to-neighbor mapping must be checked, not blamed by
   assumption. learning_path wraps sorted prerequisites (569–607). MCP tools
   delegate these three queries (tools737–762/895–918/1000–1024); defaults have no
   node filter and custom naming is orthogonal. Trace the actual code boundaries.
3. Preserve competing fixtures/assertions: algorithms700–730/847–878/982–1073/
   1258–1289, concept-card452–475, MCPfixture1063–1090 and tests1507–1523/
   1810–1829. Do not treat passing historical CI or unrun tests as new proof.
4. Produce manual source-derivation tables for a single edge and a three-node
   acyclic prerequisite chain, with explicit input convention and exact direction
   followed/expected set per API. Use both conventions; distinguish set membership
   from topo/BFS order and depth-limited versus direct helpers. No executable
   model/reproducer is needed or authorized. Show correct empty/missing-node/
   non-prerequisite/depth/default-filter controls from source. A static direction
   contradiction can be supported without claiming actual Rust/client execution.
5. If one root inconsistency is confirmed, start FA-RS-038 and consolidate its
   extractor/query/MCP symptoms. Severity must follow concrete current consumer
   impact, not a count target. Specify pinned source, Scale, mechanism/confidence,
   repair owner, validation oracle and compatibility decision. Existing manual/
   persisted edges carry direction (builder345–410,persistence1–145): require an
   explicit canonical-orientation/version/migration decision in later repair;
   do not select/reverse it now. Other graph algorithms/caches/vector behavior
   remain original-pass work, not extra findings from this bounded assignment.

Companion artifact contains full intake/ranges, direction and manual-chain maps,
at least five concrete negative checks, questions and actual static commands/
exits/limits. Detailed diagnoses go ONCE in appended cumulative Rust report; link
from the companion. Preserve all001–037 history/dispositions, including037question.
Index receives one bounded navigation note. CC ledger/close author-proposed;
CDC independently reviews. Units are this cross-API direction contract, not eight
whole-file owners, a full graph/MCP pass, runtime or the full arc.

## Concrete byte preservation, privacy and scoped commit

For report/index retain their original bytes, append the new section with only
needed separator bytes, then verify the result starts with the exact retained
prefix. Do NOT assert pre-existing newline shape or cosmetic status phrasing as
semantic gates. Check meaningful IDs/source links/ownership and protected bytes.
No other old results, coverageJSON, issued prompt or SEARCH preparation may change.

Before staging screen exact five owned path/content bytes in memory with:

```python
import re
pattern = re.compile(bytes([87,97,108,103,114,101,101,110,115]), re.I)
```

COUNTS ONLY; Acme Inc. examples. Preserve unknown outputs. Verify exact five-path
diff, source719 clean, old prefixes/coverage/SEARCH hashes and meaningful matrix
relations; record real failures proportionately. Ordinary scoped commit with BOTH
trailers after blank body:

Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>

No amend/squash/fixup/drop/merge/push. Return actual commit/paths/trailers, source-
backed conclusions/limits/questions, counts-only privacy result and explicit
five-path/planning-index ownership handback with no active mutable command.
