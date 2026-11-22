# Full workspace and architecture audit

Status: active, explicitly authorized 2026-11-21. Full arc acceptance is not yet claimed; the map and pass01 diagnosis are independently accepted. Project05's remaining feature work is held until
this audit and the subsequent repair/composition gate are complete.

## Consumer, purpose and stop condition

The consumer is Duncan and the project05 foundation-repair planner. The audit
must establish what the actual 0.5.x code does, where contracts and implementations
conflict, and which repairs are required before reusable-server work resumes.
Duncan requested a full audit rather than a sample. The scope is all first-party
workspace members and supporting entrypoints, tests, configuration, build and
release machinery. The audit ends with complete coverage accounting, actionable
findings, synthesis and a repair handoff. Do not turn diagnosis into code changes
or extend it into a separate research programme. Repeated blockers or unavailable
external inputs must be reported, not disguised as completed coverage.
Execution effort/model/contributor allocation is set when the audit is opened;
this planning record does not invent an approved time or token budget.

## Baseline and source discipline

At audit start pin the exact `release/0.5.x` source commit, planning commit,
toolchain, Cargo feature/target inventory, repository instructions and current
architecture pointers. Recheck drift before acceptance. Historical reports are
leads, not findings or verified evidence. Missing/stale architecture is itself
a finding. Preserve distinct failures and observations without duplicate archives.

The audit is diagnosis-only: no source edits, staging or implementation commits.
Any minimal reproducer is isolated and recorded; fixes belong to the later repair
arc. The separate user-authorized privacy cleanup is not an audit repair pass.

## Required method

Use collaboration-framework, project-management, work-verification and the complete
code-auditing guide set. Before auditing Rust, read rust-guidelines and all its
language guides, beginning with its anti-patterns guide. Detect other actual
languages/tools by both manifests and files and load their applicable guidance.
Do not silently drop shell, Make, CI, TOML, JSON, examples or release contracts:
record how they were inspected and any missing specialist guidance. The Operator's
explicit full-workspace scope includes architectural and operational review even
where no language-specific skill is available.

Build the audit map before findings from `Cargo.toml`, module trees, binary
entrypoints, feature definitions, test layouts, public facades and CI/build files.
Every first-party file/target/subsystem receives a coverage disposition; exclude
only identified generated, vendored, dependency and build-output trees.
`.worktrees/` is not a second source population. Cover each of seven scales:
line/function, file/module, logical unit, crate/target, application/service,
whole codebase, and workspace/system boundaries.

## Named coverage passes

These are review passes, not pre-issued implementation slices. Split a pass if
needed to retain context and verification headroom, and keep remaining coverage
explicit in the audit index.

| Pass | Population and primary contracts |
| --- | --- |
| 1 — Core/auth/config | fabryk-core, auth/auth-google/mcp-auth, ACL surface, Redis/GCP utilities; identity versus authorization, state invariants, errors, paths and configuration |
| 2 — Content and search | content parsing, source resolution/validation, FTS, metadata, caches, shared schema ownership and simple-search fallback |
| 3 — Graph and vector | extractors, persistence, graph algorithms, embedding/index lifecycle, hybrid retrieval, shared error policy and ownership |
| 4 — MCP boundary | mcp-core, content/FTS/graph/semantic tools and facades; schemas versus handlers, naming/discovery, resources/subscriptions, transport and runtime behavior |
| 5 — Application assembly | fabryk-cli, builders, orchestration, service slots/probes, default commands versus real handlers, startup/shutdown/recovery and thin-server integration |
| 6 — Existing ECL | every ecl library, pipeline/spec/topology/state crate, adapter, sink, stage and CLI; parsed configuration versus executed behavior, crash resume, side effects and integration wiring |
| 7 — Workspace/release composition | every remaining member including design and textrynum-util; dependency graph, feature combinations, MSRV, manifests, scripts, Make targets, CI, packaging, examples and downstream compatibility |

The actual manifest inventory controls completeness; these names cannot be used
to omit an unlisted crate. Review the generic versus domain-specific boundary by
operations, dependencies and required knowledge rather than vocabulary alone.
Use the existing music-server integration as a bounded consumer check where
available; changes to another repository require their own authorized scope.

## Required hunt list

- Inconsistencies in errors, configuration, IDs, serialization, time, paths,
  logging, async ownership, feature gates and public API conventions.
- Duplicate functions, algorithms, data models, caches, configuration adapters,
  state machines and policies, including near-duplicates that have drifted.
- Competing or circular abstractions, misplaced responsibilities, inappropriate
  crate dependencies, unnecessary layers and application/library leakage.
- Partial implementations: inert config, no-op commands, stubs, silent fallbacks,
  disconnected handlers, and success returns that do not deliver promised effects.
- MCP schemas/capabilities versus dispatch and observable responses; validate at
  real protocol entrypoints and protect stdio from accidental logging.
- Backend lifecycle: single source of truth, readiness, lock order, cancellation,
  retry, errors, health reporting, resource cleanup and failed/repeated startup.
- Filesystem, network, subprocess and persistence boundaries: validation,
  canonicalization, atomicity, idempotency, crash recovery, trust and secret handling.
- Misleading tests, mocks bypassing production paths, vacuous assertions, ignored
  failures, untested error paths and missing cross-crate composition evidence.
- Documentation, feature lists, examples and package contents that promise more
  than the implemented entrypoints deliver. Distinguish legacy/cancelled scope
  from an active product contract before assigning a defect.

## Validation and evidence

Derive commands from the current repository rather than guessing that old `make`
claims remain accurate. Record baseline build/test/lint/format failures and their
causes. Inspect meaningful default/no-default/feature-specific/all-feature and
supported target/package combinations; do not substitute one green umbrella build
for the matrix. Use discriminating tests and small reproducers for findings.
Where credentials/services prevent a check, retain the gap and concrete re-entry
condition. Do not infer production behavior from mock-only or compilation-only tests.

Every finding has a stable ID, Blocker/High/Medium/Low severity, exact source
commit and file:line, affected scale, observed failure mode, concrete fix direction
and validation oracle. Unconfirmed hypotheses are labeled and not counted as
confirmed defects. At least five concrete negative checks belong in each language
report. Cross-cutting findings point to one root-cause record, not duplicate counts.

## Deliverables — Operator-mandated home

All audit results go directly in `project05-discoverability/artifacts/`.
This explicitly overrides the code-auditing skill's default `workbench/` output
home and the normal per-slice artifact home for these results.
Capture the actual date at execution; `YYYY.MM.DD` below is a naming pattern,
not a claim that an audit occurred on the planning date.

- `artifacts/YYYY.MM.DD-audit-index.md`: pinned baseline, complete map, languages,
  coverage dispositions, findings counts, exclusions, unavailable checks and handoff.
- `artifacts/YYYY.MM.DD-audit-results-<language>.md`: per-language findings,
  scale evidence, coherence observations and negative checks.
- `artifacts/YYYY.MM.DD-audit-modernization-synthesis.md`: root-cause themes,
  ownership, proposed consolidations and compatibility-sensitive decisions.
- Reproducer/evidence files only where a concrete reproduction or decision needs
  them; keep them under this same project artifacts directory with clear names.

Do not create successful reports or verification records before the work happens.

## Acceptance and handoff

Complete all coverage rows in the sibling ledger with inspectable evidence.
Have a separate reviewer check coverage, source evidence and the conclusions.
The synthesis must assign every finding to a repair owner and validation approach,
protect sound existing behavior and distinguish safe refactors from explicit
behavior changes. Then open finding-driven repair slices. All findings must be
fixed before remaining project05 feature work, unless Duncan explicitly approves
a documented alternative disposition; severity alone grants no automatic waiver.
Repair acceptance and higher-scale composition are separate from audit acceptance.

## Version history

- 2026-11-21 v1.0 — Planned the Operator-requested full audit and fixed the
  results home at project05/artifacts. No audit execution or findings claimed.

## Additional Operator prerequisites and ChatGPT UX scope — 2026-11-21

Do not execute this audit until arc00's fresh green CI, latest-dependency update,
and green revalidation have been accepted. Existing instructions that call audit
"first" mean first before remaining feature development, after these newly
specified prerequisites. Pin the resulting source baseline rather than the old
pre-upgrade snapshot.

ChatGPT UX is a major project05 outcome. Audit the Claude-origin assumptions at
the actual client boundary: tool naming/descriptions, schema clarity, discovery
and navigation, guidance/resources/prompts compatibility, next-step affordances,
stable IDs/provenance, filters/pagination, summary/detail payloads, error recovery
and backend readiness. Assess request count and payload costs against complete,
correct, useful tasks. Do not equate compilation, a directory tool, small payloads
or an assistant's subjective preference with proven ChatGPT usability.

Use the project plan's ChatGPT UX acceptance axis to derive a concrete task set
and protocol. Verify actual target-client support from current official guidance
and real trials. Propose measurable tradeoffs and capture limitations; numerical
thresholds and trial scope require agreement before evaluation, not invented
success claims. The post-audit readiness work and release gate must carry these
obligations forward. All audit results remain in project05/artifacts.

## Incidental prerequisite intake — audit still unopened

[Dependency-planning audit intake](../artifacts/dependency-planning-audit-intake.md) preserves a pinned SFTP server-key observation from arc00's routine source inspection. It is source evidence for the later Existing ECL/trust-boundary passes, not a commissioned, severity-graded or accepted finding and not completed audit coverage. Revalidate against the accepted post-upgrade baseline. The audit remains planned and unexecuted; no security redesign is authorized by this link.

- 2026-11-21 v1.1 — Linked the explicitly requested incidental prerequisite observation in the Operator-mandated project artifacts home; no audit execution or acceptance claimed.

## Current authorization and preflight

Duncan's current request is: “let's fast-forward codex/ci-dependencies into
release/0.5.x, push it to all remotes, and then start the foundation audit in
Autonomous Mode with the same CDC/CC models.” The integration/push is fulfilled:
release719 was normally/atomically pushed to github, codeberg and macpro and all
three advertised tips read back exactly719. No main/tag/publication effect.
Arc00 is independently accepted. Current immutable audit source is
`719c58f245fe4a9115c7c0391d3e268c99157e2f`; initial planning is
`10e1253b205ec73eeedc48a98ddd6af1341bc1ba`. The existing accepted CI/native
proof applies to unchanged source; new branch CI44986949945 is additional.

The default endpoint is this entire audit arc, including named coverage passes,
all seven review scales, findings, independent review, synthesis, actionable
repair handoff and closing retrospectives. Do not open arc02 or implement repairs.
All audit results, inventories, evidence and reproducers go directly in
`project05-discoverability/artifacts/`; canonical planning/prompt/close/review/
retrospective files remain in the owning slice root. Actual America/Chicago
execution dates determine output prefixes.

| Responsibility | Selected configuration and actual context | Ownership / gate |
| --- | --- | --- |
| CDC, planning, independent slice review | gpt-6.1-sol High, Codex/OpenAI, `/root/foundation_audit_cdc` | Owns planning/index and source branch operations until explicit CC handoff; no product edits |
| CC, audit author | exact gpt-6-luna High, local Codex CLI | Session receipt checked on dispatch; normal workspace-write/reviewed approvals; no substitution or bypass |
| Final arc composition | fresh gpt-6.1-sol High context, requested through coordinator at readiness | Independent of CC and assembling CDC; acceptance remains pending until real review |

Physical source `/Users/oubiwann/lab/oxur/textrynum` was clean at719 on
`codex/ci-dependencies`; release checkout is separately clean at719.
Reuse the existing source checkout on dedicated `codex/foundation-audit`;
branch selection is the only authorized source-tree mutation. Product files,
source index and build/native outputs are otherwise protected. Planning physical
root is `/Users/oubiwann/lab/oxur/textrynum/.worktrees/planning`, clean initially.
One mutation owner at a time: CDC owns both until named CC handoff, then CDC
reviews read-only until return. No duplicate Cargo/native work. Preserve unknown
outputs; scoped isolated reproducers need explicit output ownership.

Required contracts: current AGENTS/project/arc/ledgers, collaboration-framework,
project-management wayfinder and operation guides, complete code-auditing set,
work-verification, prompt-authoring/intake and AUM contract, agent-coordination,
testing and methodology-stewardship; full Rust language guides beginning with
anti-patterns before semantic auditing, plus detected other-language/tool
coverage. Historical reports remain leads only. Configuration receipts prove
requested settings, not hidden backend identity. Default five refinements per
slice, unchanged by staffing; no volunteered global budget or separate budget
questionnaire is imposed.

Hosted purge stays open/nonblocking; no external messages, provider requests,
repository deletion/recreation, features, service deployment, publication or
outside-repository coding. All customer examples use Acme Inc. Current ECL is
fully audited while project06 development remains held. Real ChatGPT UX evidence
and agreed trial thresholds remain later release obligations; no local audit
claim substitutes for actual client trials.

- 2026-11-21 v1.2 — Activated the user-authorized full arc01 after accepted arc00
  and verified integrated/published719. Recorded exact roles, single ownership,
  endpoint and explicit diagnosis/results-home constraints before audit execution.

## Execution decomposition — v1.3

The seven named passes remain the required population. The following slices
retain review headroom and produce a complete diagnosis, not selected examples.
Only slice01's open set is issued now; later slices are detailed from its actual
map and accepted coverage. Each slice includes its own meaningful evidence,
independent CDC review, ledger, bubble-up and retrospective.

| Slice | Useful result and pass ownership | Dependency |
| --- | --- | --- |
| 01 — audit-map | Exhaustive file/member/target/language/tool/features map, exclusions, architecture pointers and validation applicability | Current integrated719 + canonical authorization |
| 02 — core-auth | Pass1: all core/auth/ACL/Redis/GCP configuration and trust boundaries | Accepted map |
| 03 — content-search | Pass2: content/source/FTS, schema and fallback contracts | Accepted map/core boundary |
| 04 — graph-vector | Pass3: every graph/vector file, algorithms, persistence and retrieval lifecycle | Accepted map/shared boundaries |
| 05 — mcp-runtime | Pass4 infrastructure: mcp-core, transport, registries, resources/prompts/subscriptions and lifecycle | Accepted core/engine boundaries |
| 06 — mcp-tools-ux | Pass4 tool suites/facades: every schema/handler, discoverability, IDs/provenance/payload/recovery and real-client limits | Accepted MCP runtime/engines |
| 07 — application-assembly | Pass5: all CLI/builders/orchestration/startup/shutdown/thin integration | Accepted library and MCP boundaries |
| 08 — ecl-execution | Pass6 execution: ECL core/steps/workflows/pipeline/spec/topology/state/CLI | Accepted map |
| 09 — ecl-io | Pass6 I/O: every adapter/sink/stage/secrets/GCP auth and pipeline integration | Accepted execution contracts |
| 10 — workspace-release | Pass7: remaining design/util/facades, all manifests/Make/shell/CI/config/examples/reference/binary populations and release/package/feature composition | Prior passes + map reconciliation |
| 11 — synthesis-handoff | Complete per-language reports/index, cross-cutting duplicate/root-cause synthesis, finding owners/oracles and actionable repair handoff | Every population inspected; final arc composition remains independent |

Guide loading is itself a prerequisite, not source coverage. Full Rust
SKILL/guide text (anti-patterns first) must be loaded before semantic Rust
review; map/data extraction alone does not constitute that review. Structured
population projections are not replacements for normative guide reading or
full-file semantic review. All first-party files retain a named disposition;
references/binary assets receive explicit method and limits.

- 2026-11-21 v1.3 — Split the seven required passes into eleven reviewable
  increments, including two MCP and two ECL slices for context/reproduction
  headroom; no population or scale is dropped. Issued slice01 map/intake.

## Accepted map clarification — v1.4

Slice01 map is independently accepted at candidate 498834f (CDC verification in
slice01-audit-map). Actual source remains clean 719: 360 tracked paths, 41 actual
member packages, 45 metadata targets. All semantic coverage remains pending.
The seven named passes and eleven-slice decomposition are unchanged.

Four ignored ECL planning files under docs/superpowers/plans are preserved
reference-only pass07 inputs, separately from the 360 tracked source denominator.
They are not current authority, product source or permission to revive held work.
Before any dump/extraction, use the encoded private-name screen; CDC screened
these exact four on Nov 21 with counts 0/0/0/0 and bytes 27873/57160/39508/40791.
Changed/new ignored inputs require a fresh screen. Unknown-owner outputs remain
untouched. Workbench nested archives/vendor snapshots stay excluded as identified
nonproduct reference populations; initial metadata-only traversal is disclosed.

README's 20 refers to Fabryk specifically (metadata 19), while architecture YAML's 24
refers to the workspace (metadata 41: 19 Fabryk, 21 ECL, 1 utility). These are
qualified semantic reconciliation leads for pass07, not accepted findings.

- 2026-11-21 v1.4 — Slice01 accepted complete map with preserved correction
  history. Recorded four ignored-file reference dispositions/privacy screen and
  like-population count qualifiers; no audit population, scale or endpoint change.

## Additional release-ref integration witness — v1.5

The push-triggered [run44986949945](https://github.com/oxur/textrynum/actions/runs/44986949945)
completed successfully at exact719 on release/0.5.x. Parent fetched all three
complete job logs/metadata and parsed the real toolchains/steps; CDC independently
read the retained summary and reproduced all log hashes, exact SHA presence,
all85 test summaries per test lane and zero private-pattern counts. Linux
Rust1.100 debug and release each:2440 passed/0 failed/61 existing ignored. Actual
Rust1.94 MSRV job succeeded; its check is not a test-suite claim. All required
steps succeeded; release's dependency-freshness step is intentionally debug-only.

[Concise durable summary](../artifacts/2026.11.21-release-ref-ci-summary.json)
retains URL, jobIDs137099486290/137099486522/137099486523, full-log SHA-256 values
and temporary original evidence paths. Raw2.5MB logs are preserved at the named
original temporary root and not duplicated here. This is additional release-ref
integration evidence, not new semantic audit coverage or a new entry gate; no
local/native suite was repeated. Accepted arc00 limitations remain applicable.

- 2026-11-21 v1.5 — Retained parent-observed successful release-ref CI and CDC's
  full-log digest/test-summary reconciliation; no source, scope or gate change.

## Accepted first semantic pass — v1.6, 2026-11-22

Slice02 core/auth diagnosis is independently accepted at candidate6ae2752 after
preserved review1 and refinement1/numbered missing-input continuation. All five
CORE rows,38tracked paths/sevenmembers and sevenpass01scale cells are supported;
remaining322paths/42cells stay open. Ten findingsFA-RS-001..010 (7Medium/3Low)
have logical repair owners/oracles; two hypotheses remain separate. Source stays
clean719 and no repair occurred. [Independent review](slice02-core-auth/cdc-verification.md)
and [retrospective](slice02-core-auth/cdc-retrospective.md) retain actual runtime,
static-only/provider/client limits, all failures and count/source chronology.

Later complete repair handoff must include path containment across consumers,
auth outage classification, Google cache/deadline/audience policy, lifecyclepanic
state, ADC file selection, mockRedis parity and scheme-case interoperability.
No changes are implemented here. Arc05 CI-efficiency planning was added separately
at3d550d7; its execution is not opened by audit progress. CDC regains planning/
index/source/native authority after returned CC continuation41135exit0, then may
issue the next bounded content/search assignment under the unchanged contract.

- 2026-11-22 v1.6 — Accepted first semantic pass with explicit remaining full
  coverage, preserved correction/failures, repair owners and scoped retro lessons.

## Restricted static continuation — v1.7, 2026-11-22

A platform message flagged the CDC turn for possible cybersecurity risk. Parent
ordered state recovery only; no blocked execution/content was retried. Read-only
recovery found planning clean at8d89b6372d8713b3824b7793bcee5f3ca768a56d,
source clean at719c58f245fe4a9115c7c0391d3e268c99157e2f, no staged changes,
CC session14227 completed exit0, and no active owned Cargo/Rust/probe process.
Existing prompts, candidate, logs and scratch outputs were preserved; no CDC
slice03 native replay had started and no uncommitted audit records existed. A
recovery log-reader initially failed on a non-JSON warning footer; tolerant
read-only recovery then completed. No product or evidence was changed.

Parent subsequently authorized only unaffected static software-correctness,
contract and documentation coordination for slice03. CDC owns planning/index
for that work until an exact documentation-only CC handoff; source/native/probe/
provider/security execution remains on hold. No safeguard bypass, model change,
blocked-content retry or new native/provider test is authorized. The overall arc
remains active with its full original scope; this is not a user-requested pause
or a reduced acceptance contract. Gates requiring unperformed independent replay
remain pending. Accepted001–010 and pass01 remain preserved; the23 slice03
proposals011–033 at8d89b637 remain unaccepted.

The next preserved prompt is slice03-content-search/cc-prompt-iteration01.md,
semantic refinement1of5, limited to analysis/documentation corrections. Static
review may clear or reclassify unsupported proposals with written reasons and
original IDs; it cannot silently manufacture runtime evidence or accept coverage.
The initial provisional24/011–034 progress description is superseded by the
first frozen candidate's23/011–033; explain that draft-to-freeze change rather
than treating the provisional numbering as canonical accepted findings.

- 2026-11-22 v1.7 — Recorded actual platform interruption, clean recovery,
  restricted static continuation and pending independent gates at 2026-11-22T01:44:58-06:00.
