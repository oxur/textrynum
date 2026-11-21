---
project: project06-pipeline
status: deferred
depends-on: []
blocks: []
related: ["project04-ecl-pipeline-engine", "project05-discoverability", "project07-live-ingest"]
---

# Pipeline and knowledge ingestion

Status: deferred by Duncan on 2026-11-21. Organizing existing evidence is authorized;
no project06 feature implementation, new slice assignment or detailed prospective
redesign is opened. Re-entry requires Duncan to activate the project after the
current 0.5.0 priority.

## Adopted scope ownership

- End-to-end ingestion into the knowledge fabric: extraction/normalization,
  AI-assisted stages and direct integration into Fabryk retrieval/indexing.
- Pipeline capabilities available through the normal CLI, including existing
  implemented stages/adapters whose integration is incomplete.
- Remaining Acme Inc. pipeline and full-parity milestones recorded in project04.
- Authorization and multi-tenancy needed to support these efforts. The target
  source implementation may be fabryk-acl, but the delivery obligation belongs here.
- The future pipeline direction replaces the original agent-workflow framework,
  which Duncan explicitly cancelled. Do not revive its Restate/SQLx/step-framework
  roadmap by treating old historical documents as current work.

## Historical basis and boundaries

Project04 retains the original release-0.4 plans and evidence; retaining those
records does not mean their entire roadmap was implemented. Its later pipeline
arcs and their unimplemented milestones are the source material for this project.
Do not duplicate or move completed release history into a prospective plan.
The initial inventory is in
[project04/project-plan.md](../project04-ecl-pipeline-engine/project-plan.md).

AI-engineering owns current concept-card methodology. This project may integrate
with its outputs and contracts; it must not create a competing card framework.
Live event routing/Sygnum belongs to project07. Reasoning/Medys belongs to project08.
Do not infer a dependency on completion of the whole of either project; identify
concrete required inputs when this deferred project is activated.

## Future decomposition

Retained capability groups are ingestion-to-Fabryk, runnable CLI pipeline
composition, authorization/tenant boundaries, and the remaining named data
pipeline milestones. They are ownership groups, not issued arcs/slices or a new
approved architecture. Detail only after activation and current-source review.

## Version history

- 2026-11-21 v1.0 — Recorded Duncan's scope split, hold, authorization ownership,
  and explicit cancellation of the former agent-workflow direction.
