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

The root cause of the turn-level platform interruption is unknown. No specific
Rust/Tantivy command was rejected: no CDC slice03 native replay had started. The
native hold is parent's precautionary restriction, not evidence that ordinary
software tests require another service.

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

## Scoped ordinary-local verification authority — v1.8, 2026-11-22

After read-only inspection of the frozen269-line slice03 driver and its local
case groups, parent explicitly assessed a genuinely unaffected safer task:
ordinary isolated Unicode/query/filter/metadata/index-lifecycle correctness. It
has owned text/index fixtures and no network/provider/auth/credential/traversal/
security payload, service or OS-process test. This is a separately scoped
continuation, not a retry of the unknown turn-level platform interruption.

At the clean docs02 boundary, CC returned d5f19d09c246f85f4e4e4ed99deb511c76b436df
and session58752 exited0. CDC read the complete four-doc diff, verified exact
paths/trailers, unchanged accepted report prefix, original probe/manifest/lock/
raw-log bytes, unchanged coverage/index, clean source719/planningd5f and zero
active Cargo/Rust/probe processes. CDC now solely owns planning/index and the
idle transferred /private/tmp/arc01-slice02-cargo-target.

Parent's precautionary native hold is lifted ONLY for isolated slice03 ordinary
local correctness verification: Unicode paragraph/snippet boundaries and excerpt
normalization; valid-frontmatter title/source-resolution/file-kind metadata;
configured/request query modes, threshold/boost/quote behavior; filtered retrieval
counts/limits/snippet options; rebuild/append-cache coherence, ID collisions and
controlled same-second freshness. Use a unique CDC package/binary, owned offline
lock generation and actual --locked run, owned target/temp fixtures, absolute
flat project artifacts, true exit/full provenance and discriminating negative
controls. Keep original author proof and failures intact; wrapper0 does not
establish all claims. Normal Cargo cache locks/bookkeeping are acknowledged;
protect source/index/global config and unknown outputs. No full baseline rerun.

All SECURITY-SPECIFIC execution, providers, services and later-group native
execution remain held until independently scoped. No Daybreak setup/scan, model
substitution, external service or permission assumption is authorized. If this
limited task is platform-flagged, stop/preserve/report; no variants, new context
or model as a workaround. All SEARCH/coverage acceptance remains pending actual
needed proof. The full audit scope/endpoint stays active and unchanged.

- 2026-11-22 v1.8 — Recorded explicit parent scoped authority and clean single-
  owner entry at 2026-11-22T02:23:49-06:00, before native verification execution.

## Second platform stop and unfinished-work preservation — v1.9

The platform flagged the CDC TURN twice; the root cause remains unknown. After
the second stop, parent required administrative recovery/preservation only and
no execution retry, variants, new context/model, lock fix, provider or service.
The original user-authorized full audit is NOT complete: only core/pass01's
38paths/seven scale cells and findings001–010 are independently accepted. Docs02
d5f19d09 preserves stable011–033 as17unaccepted proposals, four questions and
two cleared leads. All SEARCH/replay/whole-arc gates remain pending, with no
user-requested pause, waived criteria or reduced full-audit scope.

For slice03 CDC, no build, test or probe-run ever started. Toolchain version
reads and owned offline lock generation exited0, then a pre-run Python full-
record comparison failed an unresolved assertion before recording its receipt.
No claim about the new lock's correctness or a specific blocked execution is
made. Four owned partial preparation files are preserved UNVERIFIED under
project05-discoverability/artifacts/2026.11.22-content-search-cdc-:
probe.rs, probe.toml, probe.lock and run.log. The run log's initial probe hash
predates a pre-run source edit and is not a current run attestation; there was
no run. Preserve the original bytes and incomplete provenance for an approved
route. Metadata/hash/private-pattern-count checks prove preservation only.

Recovery found source719 clean, planningc72311d plus exactly those four owned
untracked files, no staging and no active native/probe process. Administrative
preservation appends only this note and the arc ledger and commits those exact
four files unchanged; no source/target/scratch mutation or further validation.
The last preserved prompt is slice03-content-search/cc-prompt-iteration02.md;
2of5 refinements used,3remain. All ownership returns to parent after commit,
with no active owned mutable process. The user Daybreak SETUP-ONLY question
remains unanswered; no service call, setup change or scan was performed.

- 2026-11-22 v1.9 — Recorded second turn-level stop, authoritative recovery and
  administrative preservation only at 2026-11-22T02:34:00-06:00; full audit remains unfinished.

### Retained review-context re-entry pointer

After an approved supported route is resolved, parent can reactivate the SAME
/root/foundation_composition_review chat/context. At source719 it completed
360tracked dispositions (344full-text files, structured Cargo.lock and15binary
assets) plus8ignored dispositions. Its full source/guidance/static-candidate
notes remain in that retained context; they are not accepted semantic coverage
or a whole-arc verdict. No duplicate archive, new source analysis or execution
is commissioned by this pointer. CDC/CC selected contexts and issued prompts
remain preserved; no context/model change is an authorized filter workaround.

## User-requested single local retry — v1.10, 2026-11-22

The user reports similar apparent false-positive alerts in other projects and
says they often persist until a GPT Desktop restart. The user explicitly asks
to try again. No restart is assumed to have happened, and the platform flags'
root cause remains unknown. Daybreak was declined/unavailable: no setup/data/
service/scan call is authorized or performed. Parent separately checks supported
reporting channels; CDC must not send provider/support messages.

The user/parent authorize ONE retry of the SAME previously assessed benign
ordinary local Rust framework content/search verification in the SAME selected
gpt-6.1-sol High CDC context. No new context/model, workaround or relabeling.
Re-entry verified planning4c3a50e clean, source719 clean, no staged changes and
zero active native/probe processes. CDC owns planning/index and the idle owned
target for this attempt. The four partial files' authoritative recovery hashes
match exactly; their old incomplete provenance remains UNVERIFIED and preserved.
The unresolved pre-run full-lock-record assertion and stale initial probe hash
need ordinary diagnostic metadata reconciliation before any locked run. Keep
those prior bytes/failures; a separate retry log will record the current input
hashes, reconciliation, actual commands/exits and any new evidence.

Only the already assessed Acme TempDir Unicode/query/filter/metadata/index-
lifecycle groups are allowed, with the unique CDC package/binary, owned offline
lock generation and actual --locked Cargo execution, owned target/fixtures and
absolute flat project artifact outputs. No auth, network/provider, credential,
traversal/security payload, service or later-group extension; global config,
source/index and unknown outputs stay protected. This is ordinary framework
software correctness work; the broader full audit does include security
observations, and this entry does not erase or relabel them.

If this limited retry is platform-flagged again: STOP, perform administrative
recovery only, preserve exact available error/context/step and execution state,
return ownership, and do not try variants, models/contexts or another execution
retry. Successful execution may support the allowed03 close with independent
review, but no SEARCH row is accepted before actual necessary proof. Full audit
scope, models, finding history and endpoint remain intact and uncompleted.

- 2026-11-22 v1.10 — Recorded the user's single-retry request and bounded current
  authority at 2026-11-22T08:51:55-06:00, before metadata reconciliation or execution.

## Consumed retry, third stop and local feedback draft — v1.11

The user's ONE same-context retry was consumed by a third platform TURN flag:
“This content was flagged for possible cybersecurity risk.” The root cause is
unknown; no individual failed execution command is identified. In THIS retry,
only read-only re-entry checks and the three-document authorization commit
84f37c6debf699c46fcebfe83c36e21e323a323c completed. No pre-run metadata
reconciliation, new version read, offline lock generation, build/test/probe or
new retry log started. Earlier version/offline-lock0 results belong to the
previous attempt and are not latest-retry evidence. No further retry, variant,
model/context change, provider/service or diagnostic lock fix is authorized.

Administrative recovery found planning84f37c6 clean/unstaged, source719 clean/
unstaged and no active native/probe process. The four previously preserved
partial files remain byte-unchanged and UNVERIFIED; the initial logged hash
still predates a pre-run edit and is not run attestation. All SEARCH gates stay
pending. Only core/pass01's38paths/7cells and10findings are accepted; docs02d5f
retains17unaccepted proposals,4questions and2cleared leads. The full audit is
unfinished; no user pause, scope reduction or criteria waiver occurred. Last
prompt remains iteration02,2of5 refinements used/3remaining.

Daybreak is unavailable/declined per the user; no setup/data/service/scan calls
were made. Parent authorized administrative notes and a LOCAL DRAFT ONLY at
[platform feedback draft](../artifacts/2026.11.22-platform-false-positive-feedback.md).
It copies no private customer identity, proprietary source or raw conversation,
and has NOT been submitted. Supported feedback-route investigation belongs to
parent; CDC sent no provider/support message. All ownership returns to parent
after the exact administrative commit; source/target/scratch stay unchanged
and idle. The retained reviewer re-entry pointer remains applicable.

- 2026-11-22 v1.11 — Recorded consumed retry/third-stop recovery and local
  unsubmitted feedback draft at 2026-11-22T09:01:29-06:00; no execution or acceptance.

## User-reported restart and same-context resumption — v1.12

The user now explicitly reports restarting the app and requests resuming the
foundation audit with the same CDC/CC models. This is new authorization for the
pending previously assessed ordinary-local03 verification, superseding the
prior consumed-retry hold for that task. It is not evidence that a restart
caused or cured the unknown platform classification. Daybreak remains declined;
no setup/data/scan/service or support message is authorized.

Independent re-entry confirmed planning5961f3b9aa4e81913a59aa5cb486cf59b82f6aea
and source719c58f245fe4a9115c7c0391d3e268c99157e2f clean/unstaged. The four
partial preparation hashes remain exactly preserved and UNVERIFIED. Direct
physical planning reads now succeed in the normal sandbox. A process metadata
read alone failed with macOS ps EPERM, then ordinary reviewed escalation
confirmed zero Cargo/Rust/probe processes; this permission event is distinct
from the earlier turn-level platform flags. No global setting or symlink changed.
CDC now owns planning/index and the idle owned target; CC has no active command.

Retained CDC gpt-6.1-sol High and exact CC gpt-6-luna High/session
01a2019f-73de-7de1-831f-53d55b3da247 remain selected. Full source/guidance intake
and completed gates are retained, not repeated solely for restart. Last prompt
is03iteration02;2of5 refinements used/3remain. Corrected d5f diagnosis retains
17unaccepted proposals,4questions014/016/028/029,2cleared018/031 and stable
011–033. Accepted001–010/core38paths/7cells remain preserved.

Before any locked run, reconcile the earlier incomplete lock-record assertion
and stale initial hash as ordinary diagnostic bookkeeping. Keep all prior
bytes/failures and use distinct current-attempt witnesses/hashes; do not turn
the old pre-edit hash into run attestation. The only native verification now
authorized is the already inspected local03 Unicode/query/filter/metadata/index-
lifecycle driver with synthetic Acme TempDirs, unique CDC package/binary, owned
offline lock and actual --locked run, true exits/full provenance, owned target
and absolute flat project artifacts. No auth/network/provider/credential/
traversal/security payload/service/OS-process test or later-group native
extension. Preserve source/index/global config and unknown files. No source
repair, full baseline/CI rerun or publication.

If this same limited resumption is platform-flagged again: STOP and recover
administratively, preserve facts/outputs/available step identifiers, return
ownership and do not try variants, models/contexts or automatic retries. If it
succeeds, complete03 review/close/bubble-up/CDC retro and continue the full arc
under its intact contract and specifically scoped safe subsequent assignments.
Security-specific/provider/exploit/service execution remains held; static
diagnoses retain their actual confidence. No SEARCH or later gate is accepted
before its needed proof; the endpoint remains the entire accepted audit and
actionable repair handoff, not one probe, arc02 repairs or arc05 implementation.

- 2026-11-22 v1.12 — Recorded reported restart/resumption, independently checked
  ownership/state and precise current authority at 2026-11-22T09:31:43-06:00, before execution.

## Fourth turn-level stop after reported restart — v1.13

The platform flag returned after the user explicitly reported restarting the
app and resumed the same selected CDC/CC models. This is the FOURTH turn-level
flag; its root cause and exact blocked request ID remain unknown. Do not infer
a particular rejected Rust command or restart causality. After authorization
commit00b3d62046c0c24e0704e740890755bac585786d and before the flag, no further
tool command was issued. No pre-run reconciliation, version read, offline lock
generation, build/test/probe, provider/service or new attempt file/log started
in this resumption. The earlier re-entry ps EPERM was an ordinary sandbox
permission event resolved by reviewed metadata-only escalation, not this flag.

Administrative recovery8df182 exited0: planning00b3d62/source719 clean/unstaged,
no new attempt files and native/probe processes0. Four original partial files
retain exact prior hashes/bytes and remain UNVERIFIED, with stale initial hash
not run attestation and prior lock assertion unresolved. No scratch/target or
source mutation, diagnostic interpretation or execution retry is authorized.
Only administrative arc notes and the existing local NOTSUBMITTED feedback
draft are updated to retain these facts; no support submission or Daybreak
call/scan occurs. If an administrative action is itself flagged, stop without
variants. All ownership returns to parent after this exact administrative commit.

Only core38paths/7cells and001–010 are independently accepted. Current03 retains
17unaccepted proposals,4questions and2cleared leads; all SEARCH/remainingarc
gates remain pending. Last CC prompt is iteration02,2of5 refinements used/3left.
No user pause, criteria waiver, source repair, scope/model/context change or
whole-audit completion is claimed. No further retry is authorized by this
recovery; a supported route is still needed.

- 2026-11-22 v1.13 — Recorded fourth stop after reported restart and exact
  administrative recovery at 2026-11-22T09:35:59-06:00; unfinished full audit preserved.

## Separate bounded static API consistency work — v1.14

Duncan explicitly resumes the code quality assessment in Autonomous Mode with
the same CDC gpt-6.1-sol High and CC exact gpt-6-luna High. This authorization
opens separate bounded work on public API consistency, feature declarations
versus module availability, duplicate configuration handling and documentation
versus implemented entrypoints. It does NOT retry, reconcile, run or reassign
the interrupted SEARCH operation. Its partial files remain untouched/unverified
and every SEARCH gate remains pending. Full arc scope and evidence standards
remain intact; no repair, feature/CI edit, publication or external message.

Parent independently submitted authorized platform feedback at17c6ecb, confirmed
by the existing report/receipt: no-active-thread-01a203eb-d26f-73a2-ae62-a9350da65b07,
no logs/attachments/chat association. Do not resubmit. Daybreak remains declined.
Current planning17c6ecb314a4860beddbabc8da86ea9338c182a0 and source719 are clean;
CDC owns planning/index, source remains read-only. Retained source/full guidance
and exact CC session01a2019f-73de-7de1-831f-53d55b3da247 are reused. Unavailable
retained context must be disclosed, without silent model substitution.

### Bounded decomposition and first deliverable

Add slice12-api-feature-consistency as an additional bounded increment executed
ahead of the existing04–11 sequence while03 remains pending; do not rename or
replace any existing slice/owner. First scope: all41 member-manifest feature/
optional-dependency projections, then complete18 manifest/lib entrypoint files
for9 feature-declaring members (26explicit keys/1049lines), all7existing member
READMEs plus root manifest and immediate declaration/gate/name traces. The32
other members have no explicit feature table or optional-dependency feature
requiring another feature-bearing member. No full-file or whole-pass semantic
acceptance follows from this feature facet. Coverage JSON owner/status rows and
49cell totals remain unchanged; index records a separately bounded static result.

Trace local aliases, dep: activation, dependency features, cfg modules/re-exports
and exact current README availability/name claims. Distinguish declared static
availability from compiled/runtime proof, optional feature forwarding from
missing gates, and current source from explicitly historical0.2examples. The
output is a complete26-key/public-entrypoint matrix with actionable supported
findings or concrete negative checks. All results remain directly in project05
artifacts. CC authors, CDC independently reviews; defaultfive refinements for
this new bounded increment, no reset of03's2/5used or any accepted result.

Later separate increments may address duplicate configuration and additional
entrypoint documentation only after a concrete source-guided scope is recorded.
This decomposition does not authorize a runtime check or blocked-operation retry.

### Terminology and stop contract

Use accurate ordinary software engineering terms in new prose/prompts—“code quality assessment”, “behavioral check”, or “API consistency review”—when equally precise. Keep necessary technical facts, exact code identifiers and historical records unchanged. Do not construct keyword blacklists, obscure or relabel the work, or use wording/context/model changes to evade platform restrictions; we do not know which words triggered prior flags. If a task is platform-flagged, stop that affected work and report its evidence without variants or retries.

Accepted001–010/core38paths/7cells and stable011–033 (17proposals/4questions/
2cleared, not accepted) remain preserved. Source719, whole-arc endpoint, seven
passes/scales and independent final composition are unchanged. No complete-arc
claim is made by this bounded static result.

- 2026-11-22 v1.14 — Recorded separate user-authorized static work, narrow
  decomposition, roles/ownership and terminology contract at 2026-11-22T10:17:13-06:00, before CC.

## Accepted bounded static feature/API review — v1.15

Slice12 candidateb382da16 is independently accepted by assignedCDC for all five
API criteria: complete26feature-key relationships/nine members and exact README
claims. TwoLow documentation findings034/035 join accepted001–010; pending
011–033 stay unchanged. No gate mismatch was found in this static facet, and no
compile/runtime claim was made. Whole-file coverage remains38accepted/31proposed/
291pending;49scale cells stay7accepted/7proposed/35pending. No original pass or
SEARCH gate is closed by this increment. Verification and retro are in slice12.

The selected CCretained session authored and the selectedCDC independently
reviewed source. The earlier independent whole-arc reviewer is unavailable now
and did not review this new candidate; parent confirmed the two-contributor
slice arrangement. Final independent arc composition remains pending. No extra
context/model was created for this checkpoint. The next separate configuration
API/entrypoint scope must be recorded before contributor work.

- 2026-11-22 v1.15 — Accepted bounded static review, twoLow findings, exact
  preservation/coverage limits and proportionate process retrospective.
