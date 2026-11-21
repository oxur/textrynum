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
new RAG-supported servers. The Operator selected the short slug `discoverability`;
the scope also includes thin application assembly, backend startup and shutdown,
shared infrastructure, and elimination of duplicate logic and competing abstractions.
Consolidation is a substantial release objective, not incidental tidying.

This is the current plan of record, agreed in the 2026-11-21 conversation. It does
not accept historical completion claims or turn the imported plans into fresh
implementation assignments. The implementation baseline is `release/0.5.x`;
use a dedicated non-default task branch when implementation is authorized.

## Mandatory order and release gate

1. Complete the full code and architectural audit in
   [arc01-foundation-audit/arc-plan.md](arc01-foundation-audit/arc-plan.md).
2. Repair the inconsistencies, partial implementations, duplicate code/logic,
   and code and architectural anti-patterns identified by that audit. Reproduce
   the repairs independently and verify their composition before feature work.
3. Reconcile the remaining reusable-server/discoverability scope against the
   repaired foundation. Resume only work that still has a demonstrated consumer.
4. Validate the actual 0.5.0 packages and downstream use, then seek the Operator's
   release acceptance. This plan does not authorize publishing a 0.5.0 release.

Privacy cleanup of the repository and its affected remote history is an immediate
prerequisite. Use only Acme Inc. in prose and anonymized identifiers in code,
fixtures and paths. Keep all other project scope frozen during that cleanup.

## Arc roadmap

| Arc | Useful result | Dependencies / status |
| --- | --- | --- |
| 01 — Foundation audit | Complete, severity-graded, evidence-based workspace and architecture findings with an implementation handoff | Detailed plan exists; execution has not started |
| 02 — Foundation repair and consolidation | Audited defects repaired at their root, shared policies centralized, real behavior and integration demonstrated | Requires accepted audit; slices are created from actual findings, not invented in advance |
| 03 — Reusable server readiness | Consistent thin-server assembly, backend lifecycle and observability, tool discovery and remaining existing extraction obligations | Blocked until audit and repair gates pass; detailed scope will be reconciled then |
| 04 — 0.5.0 release validation | Usable packaged crates and demonstrated downstream migration/new-server construction | Requires accepted foundation and server-readiness composition; Operator release approval remains explicit |

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

Current authorization covers planning, historical-document organization and the
explicit privacy cleanup. The audit is planned; no code-audit pass, remediation
slice or feature assignment is issued by this document. Default implementation
workflow is CDC + CC when opened, with a structurally independent reviewer and
`cdc-verification.md` at slice closure. No contributor accepts their own repair.
No Autonomous Mode, project06/07 execution, publication or release is inferred.
Project composition and Operator acceptance remain explicit gates.

## Version history

- 2026-11-21 v1.0 — Duncan adopted projects05–08 and made full audit and repair
  prerequisites for further project05 feature work. Audit outputs explicitly
  belong in `project05-discoverability/artifacts/`.

## Retained historical documents

- [0014-complete-the-fabryk-extraction-implementation-plan.md](artifacts/0014-complete-the-fabryk-extraction-implementation-plan.md)
- [0015-plan-cross-crate-duplicate-code-consolidation.md](artifacts/0015-plan-cross-crate-duplicate-code-consolidation.md)
- [0024-code-audit-report-poc-mcp-server-final-fabryk-extraction.md](artifacts/0024-code-audit-report-poc-mcp-server-final-fabryk-extraction.md)
- [duplicate-code-audit.md](artifacts/duplicate-code-audit.md)
