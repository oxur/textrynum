---
project: project08-reasoning
status: deferred
depends-on: []
blocks: []
related: ["project05-discoverability", "project06-pipeline", "ai-engineering"]
---

# Reasoning and Medys

Status: deferred. Duncan assigned Medys/reasoning here on 2026-11-21; the current
priority is shipping a solid 0.5.0 through project05. No reasoning implementation
or new research campaign is opened by organizing these documents.

## Historical evidence

The artifacts preserve the abstraction-tooling notes, reasoning/LLM research,
architectural examples and unified reasoning-server proposal. Themes include
structural abstraction checks, external verification, reasoning-session state,
solver integration, retrieval support and reusable reasoning templates.
The earlier theorem-prover survey remains in project04's historical artifacts;
its ownership for prospective reasoning work is now this project.

These documents are proposals and historical research, not validated capability,
approved effort estimates or empirical proof of claims about model behavior.
Reassess concrete usefulness, consumers, source validity, scope and stop conditions
with Duncan before commissioning research or implementation.

## Boundaries

Concept-card development and the collaboration framework are owned by the
ai-engineering repository. Reuse their contracts rather than starting a competing
card framework. Sygnum belongs to project07; end-to-end ingestion and pipeline
capabilities belong to project06. No current arcs/slices are manufactured from
speculative schedules in the preserved proposal.

## Version history

- 2026-11-21 v1.0 — Assigned Medys/reasoning evidence to project08; retained
  proposals without promoting their completion or research claims.

## Retained historical documents

- [0001-abstraction-tooling-design-notes.md](artifacts/0001-abstraction-tooling-design-notes.md)
- [0022-the-limitations-of-llms-with-abstract-reasoning.md](artifacts/0022-the-limitations-of-llms-with-abstract-reasoning.md)
- [0023-augmenting-llm-reasoning-with-external-tools-a-practitioners-landscape.md](artifacts/0023-augmenting-llm-reasoning-with-external-tools-a-practitioners-landscape.md)
- [architectural-blueprints-for-a-rust-mcp-reasoning-server.md](artifacts/architectural-blueprints-for-a-rust-mcp-reasoning-server.md)
- [unified-rust-mcp-server-for-llm-reasoning-augmentation.md](artifacts/unified-rust-mcp-server-for-llm-reasoning-augmentation.md)
