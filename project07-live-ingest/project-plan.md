---
project: project07-live-ingest
status: deferred
depends-on: []
blocks: []
related: ["project06-pipeline"]
---

# Live ingestion and Sygnum

Status: deferred by Duncan on 2026-11-21. No implementation starts while project05
is the active release priority; re-entry requires explicit activation by Duncan.

## Historical direction

Sygnum owns the live/event-driven ingestion direction: event envelopes and typed
routing, publish/subscribe, backend boundaries and application-owned wiring.
ECL remains the workflow/configuration layer and Fabryk the knowledge/retrieval
layer. The existing design conversation and event-bus ecosystem research are
preserved in `artifacts/`; their proposed interfaces and deferred questions have
not been implemented or accepted merely by moving the files.

The old design proposed incubating Sygnum in Keystone before moving stable crates
into Textrynum. Recheck actual repository state and the current intended consumer
at activation; do not assume the historical incubation or implementation occurred.

## Boundaries

Project06 owns pipeline delivery and required authorization/multi-tenancy.
Project08 owns Medys/reasoning. Do not expand Sygnum into workflow policy or domain
semantics because those appear in an example event flow. No new detailed arcs,
implementation prompts, budgets or acceptance results are invented here.

## Version history

- 2026-11-21 v1.0 — Assigned Sygnum/live-ingest evidence to project07 and recorded
  the Operator's execution hold.

## Retained historical documents

- [0020-sygnum-event-driven-architecture-for-keystone.md](artifacts/0020-sygnum-event-driven-architecture-for-keystone.md)
- [0021-building-a-trait-based-event-bus-in-rust-ecosystem-landscape-and-design-patterns.md](artifacts/0021-building-a-trait-based-event-bus-in-rust-ecosystem-landscape-and-design-patterns.md)
