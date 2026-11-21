---
project: project05-discoverability
status: planning
depends-on: []
blocks: []
related: ["project06-pipeline", "project07-live-ingest", "project08-reasoning", "ai-engineering"]
---

# Discoverability and a dependable 0.5.0 MCP foundation

## Outcome and boundaries

Ship a solid 0.5.0 foundation for updating existing domain MCP servers and building
new RAG-supported servers, with ChatGPT UX as a primary delivery outcome. The Operator selected the short slug `discoverability`;
the scope also includes thin application assembly, backend startup and shutdown,
shared infrastructure, and elimination of duplicate logic and competing abstractions.
Consolidation is a substantial release objective, not incidental tidying.

This is the current plan of record, agreed in the 2026-11-21 conversation. It does
not accept historical completion claims or turn the imported plans into fresh
implementation assignments. The implementation baseline is `release/0.5.x`;
use a dedicated non-default task branch when implementation is authorized.

## Mandatory order and release gate

1. Preserve the completed sanitization. Duncan subsequently accepted the current
   coverage and deferred the remaining hosted purge as **open and nonblocking**.
   Macpro is verified purged; GitHub retains old content and Codeberg's private
   request awaits an outcome. Do not delete/recreate repositories or submit
   further GitHub destructive form operations without new authorization.
2. Apply the requested AGENTS.md policies (rebase/fast-forward only and Cayrn's
   two co-author trailers), fast-forward `codex/planning-migration` into
   `release/0.5.x`, then push all five release branches to all three remotes.
3. Get a fresh CI baseline passing. CI was re-enabled by Duncan; the assistant
   dispatched a read-only baseline on sanitized main without changing source.
4. Update all external dependencies to the latest stable versions, including
   applicable major releases, and get checks and CI green again. Record any real
   upstream compatibility constraint instead of silently claiming everything is
   latest. This prerequisite is planned in
   [arc00-ci-dependencies/arc-plan.md](arc00-ci-dependencies/arc-plan.md).
5. Complete the full code and architectural audit in
   [arc01-foundation-audit/arc-plan.md](arc01-foundation-audit/arc-plan.md).
6. Repair the inconsistencies, partial implementations, duplicate code/logic,
   and code and architectural anti-patterns identified by that audit. Reproduce
   the repairs independently and verify composition before feature work.
7. Reconcile and deliver remaining ChatGPT UX, reusable-server and discoverability
   scope against the repaired foundation.
8. Validate the actual 0.5.0 packages and real ChatGPT task workflows, then seek
   the Operator's release acceptance. Publishing 0.5.0 remains separately gated.

Duncan explicitly lifted the prior hosted-purge hold and instructed continuation
of the rest of the work. AGENTS edits, the source-cleanup fast-forward, release
branch pushes and bounded arc00 execution may proceed. The full audit remains
unopened until CI/dependency preparation is accepted.

Use only Acme Inc. in prose and anonymized identifiers in code, fixtures and
paths. Keep the deferred purge visible without misrepresenting retained hosted
objects as removed. Repository deletion/recreation is for Duncan's possible later
decision, not an action authorized by this continuation.

## Arc roadmap

| Arc | Useful result | Dependencies / status |
| --- | --- | --- |
| 00 — CI and dependencies | Fresh green CI, latest external dependencies and green checks again | Integration complete; bounded Autonomous execution authorized; hosted purge deferred |
| 01 — Foundation audit | Complete, severity-graded, evidence-based workspace and architecture findings with an implementation handoff | Detailed plan exists; execution has not started |
| 02 — Foundation repair and consolidation | Audited defects repaired at their root, shared policies centralized, real behavior and integration demonstrated | Requires accepted audit; slices are created from actual findings, not invented in advance |
| 03 — ChatGPT UX and reusable server readiness | Natural ChatGPT workflows, efficient discovery and payloads, consistent thin-server assembly, backend lifecycle and remaining existing extraction obligations | Blocked until audit and repair gates pass; detailed scope will be reconciled then |
| 04 — 0.5.0 release validation | Usable packaged crates, demonstrated downstream migration/new-server construction and real ChatGPT task acceptance | Requires accepted foundation and server-readiness composition; Operator release approval remains explicit |

The deferred vector-search improvements belong here tentatively. After the audit,
choose an explicit disposition for hosted embedding providers, long-document
chunking, and index maintenance: include in 0.5.0, retain a bounded later item,
or cancel with the Operator. Their tentative assignment does not silently expand
the release or waive the audit gate.

## Scope ownership

- Project06 owns end-to-end knowledge ingestion, pipeline CLI completion, the
  Acme Inc./full-parity pipeline work, and authorization/multi-tenancy required
  by those pipelines. Its implementation is on hold.
- Project07 owns Sygnum and live ingestion; implementation is on hold.
- Project08 owns Medys/reasoning; it is a later project, not part of shipping 0.5.0.
- Concept-card methodology and ongoing concept-card development are owned by the
  `ai-engineering` repository, which also houses the collaboration framework.
  Existing Textrynum card readers/providers remain an audit and compatibility
  surface; do not create a competing methodology or new extraction programme here.
- Existing ECL crates are included in the full workspace audit because they ship
  from this repository. Auditing/fixing current defects does not open project06's
  deferred feature roadmap.

## Historical basis

The `artifacts/` directory contains the remaining extraction plan, the later music
server extraction audit, the duplicate-code audit, and its consolidation plan.
These are evidence to reconcile with code, not a fresh assertion that their
requirements have been delivered. Do not repeat consolidation phases already
implemented simply because an old document remains present.

## Authority and acceptance

Planning, privacy cleanup and read-only CI inspection are active. Duncan requested
CI and dependency preparation before audit or feature coding, and explicitly chose
Autonomous Mode when implementation is ready. Duncan has now explicitly lifted
the hosted-purge entry gate while retaining that task open. At this authorization
update no contributor has been dispatched and no source coding/audit has started.

| Role | Approved model and reasoning | Deployment and actual assignment |
| --- | --- | --- |
| CDC — coordination/planning and independent review | `gpt-6.1-sol`, High | Codex/OpenAI runner; requested explicitly by Duncan; CDC dispatched as `/root/ci_dependency_cdc`, CC exact CLI dispatch pending |
| CC — implementation | `gpt-6-luna`, High | Codex/OpenAI runner; requested explicitly by Duncan; not yet dispatched |

Both IDs and High reasoning were found in the local Codex model catalog. This is
catalog verification, not a runtime identity claim. Apply the exact choices at
dispatch; no silent substitution, inheritance or downgrade is authorized. Carry
these unchanged selections forward without asking again. The current preparation
context does not claim to be the selected CDC runtime; handoff to that context
must happen before Autonomous implementation work.

The first bounded implementation scope is arc00 CI/dependency preparation, with
one CC and CDC's independent review, dedicated non-default source branch, one
worktree mutation owner, preserved prompts, meaningful validation and normal
ledger/retrospective duties. Do not infer whole-project execution or open the audit
from this planning record. No project06/07 work, unrelated external messages,
publication or 0.5.0 release is authorized by that mode selection.

## Version history

- 2026-11-21 v1.0 — Duncan adopted projects05–08 and made full audit and repair
  prerequisites for further project05 feature work. Audit outputs explicitly
  belong in `project05-discoverability/artifacts/`.

## Retained historical documents

- [0014-complete-the-fabryk-extraction-implementation-plan.md](artifacts/0014-complete-the-fabryk-extraction-implementation-plan.md)
- [0015-plan-cross-crate-duplicate-code-consolidation.md](artifacts/0015-plan-cross-crate-duplicate-code-consolidation.md)
- [0024-code-audit-report-poc-mcp-server-final-fabryk-extraction.md](artifacts/0024-code-audit-report-poc-mcp-server-final-fabryk-extraction.md)
- [duplicate-code-audit.md](artifacts/duplicate-code-audit.md)

Privacy ref rewrite and validation: [privacy-rewrite-record.md](artifacts/privacy-rewrite-record.md). Full hosted purge remains open because old commits are still served by hash.

## ChatGPT UX — core project05 acceptance axis

Duncan made ChatGPT UX a major project outcome on 2026-11-21. The original
Claude-oriented conventions must be examined against actual ChatGPT use. The
result should be easy to find, understand and use naturally, with capabilities
and information where the consuming client/model expects them.

This is an API/tool-interaction requirement spanning discovery, naming, schemas,
metadata, responses, resources, errors, documentation and server composition.
It is not a separate frontend project. Preserve useful cross-client behavior
while removing unsupported assumptions about a particular model/client.

### Useful capabilities and candidate design questions

- A caller can discover the relevant capability from a natural domain task,
  understand its inputs and invoke it without guessing or loading the whole suite.
- Tool names, descriptions, categories and argument schemas provide consistent,
  specific routing cues with enough examples to disambiguate likely confusion.
- Discovery and result responses expose stable IDs/locators, provenance and an
  actionable next step; detail can be requested progressively when useful.
- Search/list results, pagination, filters and summary/detail boundaries balance
  request count, payload size, useful information and completeness.
- Errors make recovery clear; unavailable services and unsupported operations
  are distinguished from empty results. Avoid repeated blind rediscovery.
- Evaluate server guidance, directory tools, resources and prompts against the
  actual supported ChatGPT surface and current documented MCP behavior. Do not
  assume Claude-specific resources/instructions work the same way in ChatGPT.

These are acceptance concerns and candidate patterns, not a premature selection
of a new endpoint, directory schema or response envelope before the audit.

### Evaluation and release evidence

Use scientific-methods for the task-level evaluation protocol. Before trials,
record the target ChatGPT client/deployment and MCP setup, source/package revision,
corpus, instructions, model/settings where selectable, task set, comparison
conditions, success oracle, measures, budget and stopping/re-entry conditions.
Agree the concrete trial scope and thresholds with Duncan; no invented numerical
budgets or unrun trial claims appear in this plan.

| Axis | Observable evidence |
| --- | --- |
| Task success and correctness | Expected task completed with correct grounded results and no silently omitted required information |
| Discoverability | Calls and elapsed steps to identify the right capability, wrong-tool choices, schema misunderstandings and manual hints required |
| Request efficiency | Total calls and repeated discovery/detail requests needed for a completed task |
| Payload efficiency | Request/response bytes; tokens only where measured or clearly labeled estimates; irrelevant bulk and truncation/pagination costs |
| Useful information | Required answer fields, relevant context, source/provenance and sufficient stable IDs for follow-up |
| Natural progression | Successful handoffs from discovery to search, source detail, graph exploration and recovery from empty/unavailable/error cases |

Candidate representative tasks are cold-start discovery, focused retrieval with
citation, broad exploration followed by narrowing, content/source follow-up,
graph relationship exploration, pagination and backend-unavailable recovery.
Use real ChatGPT task trials for release evidence; local protocol/unit checks
support that evidence but do not establish the client UX by themselves. Preserve
raw traces and model/client limits. A reduction in calls or bytes is accepted only
with preserved task correctness and useful information. Do not optimize a single
count in isolation or treat a single successful anecdote as general acceptance.

Record this requirement in the audit and post-audit roadmap, then derive concrete
implementation slices after foundation repair. Audit results retain their mandated
home in `project05-discoverability/artifacts/`.

### Follow-up decision history

- 2026-11-21 v1.1 — Wait for all hosted purges; queue rebase/FF-only and Cayrn
  co-author policies, source-cleanup FF into release/0.5.x and release-branch pushes.
- 2026-11-21 v1.2 — Require green CI, latest dependencies and green checks before
  audit; select Autonomous CDC 6.1 Sol High and CC 6 Luna High for ready implementation.
- 2026-11-21 v1.3 — Make actual ChatGPT UX, discovery and balanced request/payload/
  information efficiency primary scope and release acceptance requirements.

- 2026-11-21 v1.4 — Duncan paused the GitHub purge attempt, accepted cleanup so
  far and directed continuation while keeping hosted purge open. This supersedes
  the earlier wait-for-all-purges gate; no repository deletion/recreation is
  authorized. Proceed with AGENTS policies, FF, release pushes and bounded arc00.

- 2026-11-21 v1.5 — AGENTS policies committed, source-cleanup fast-forward
  completed at `92c72d7`, and all five release branches pushed normally/atomically
  to all three remotes with tip verification. Arc00 source branch is now
  `codex/ci-dependencies`; contributor dispatch follows the recorded selection.

- 2026-11-21 v1.6 — arc00 CDC dispatch/ownership accepted; slice01 minimal baseline repair opened from current run 44952335686. Exact CC dispatch and independent results remain pending.
