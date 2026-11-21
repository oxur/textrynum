# Pipeline Crate Synthesis

## Your Task

You have six analysis documents produced by deep-reading Rust workflow/DAG
crates. Read all six, then produce a synthesis document that combines their
best ideas into actionable recommendations for our pipeline runner design.
Save the output as `pipeline-crate-synthesis.md` in the current working
directory.

## Input Files

Read all of these before starting the synthesis:

- `analysis-dagrs.md` — Flow-Based Programming, async DAG orchestration,
  config file parsing, channel-based communication
- `analysis-erio-workflow.md` — AI agent workflows with checkpointing,
  typed context passing, conditional execution
- `analysis-oxigdal-workflow.md` — Geospatial DAG workflows with state
  persistence, retry policies, resource-aware scheduling
- `analysis-dagx.md` — Minimal type-safe DAG executor with compile-time
  cycle prevention via type-state patterns
- `analysis-dagga.md` — Resource-based implicit dependency scheduling with
  create/read/write/consume semantics
- `analysis-rettle.md` — Keras-inspired ETL framework with Fill/Transform/Pour
  ingredient model

## Project Context

We are designing a **lightweight CLI pipeline runner in Rust** for the
ECL/Fabryk ecosystem — a Rust-based AI workflow orchestration and knowledge
fabric framework. The pipeline must deliver five properties:

1. **Durability** — Checkpoint/resume on failure; survive crashes without
   re-executing completed work
2. **Configurability** — Declarative, per-source TOML-based configuration for
   source selection, format handling, and processing parameters
3. **Observability** — Serializable state that humans or AI can inspect to
   understand exactly what happened, what failed, and why
4. **Incrementality** — Skip unchanged items across runs via content hashing
   or timestamps
5. **Composability** — Mix-and-match stages per source type; not every source
   needs every transformation step

We are **NOT** adopting any crate wholesale. We built this analysis to mine
excellent concepts, design patterns, trait designs, and architectural decisions
for our own `ecl-*` crates.

### Target Weight Class

"A task runner with checkpointing." Run on a beefy local machine. Configure,
point at a data source, run, monitor, iterate. No Kubernetes, no external
databases (we use redb for embedded persistence), no distributed orchestration
servers.

### Three-Layer Architecture

Our design separates configuration into three layers:

- **Specification** — Static declaration from TOML files, immutable during
  execution (where Configurability lives)
- **Topology** — Derived structure: concrete adapter instances, resolved
  connections. Computed at startup, then immutable (where Composability lives)
- **State** — Runtime accumulation of progress, results, failures. Mutates
  during execution (where Durability, Incrementality, and Observability live)

### Key Design Principle: State IS the Pipeline

The pipeline's serialized state struct should be the complete, inspectable,
resumable truth. When handed to an AI (or a human), it should contain enough
richly-typed, well-named data that a full conceptual analysis of the system's
state is possible without needing logs, dashboards, or database queries.

### Concrete Use Case

Extracting documents from Google Drive, chats from Slack, notes from Granola →
normalizing to structured markdown → feeding into a knowledge synthesis pipeline
(concept cards → unified concepts → guides → FTS/graph indexing). The extraction
and normalization layers are the biggest gaps.

### Component Stack Already Decided

| Pillar | Crate(s) |
|--------|----------|
| Durability | `redb` (v3.1.0) — pure Rust, single-file, ACID |
| Configurability | `toml` + `serde` + custom PipelineConfig structs |
| Observability | Serializable state in redb + `tracing` |
| Incrementality | `blake3` (v1.8.2) — content hashing, stored in redb |
| Composability | Custom `Stage<I, O>` trait (design TBD — this synthesis informs it) |

## Output Format

Produce `pipeline-crate-synthesis.md` with these sections:

### 1. Concept Inventory
A table of every extractable concept across all six crates, deduplicated,
with:
- Concept name
- Source crate(s)
- Brief description
- Relevance rating (HIGH / MEDIUM / LOW) for our five pillars
- Which pillar(s) it serves

### 2. Trait Design Recommendations
Based on the best patterns seen across all crates, recommend the core trait
signatures for our pipeline runner. Consider:
- A `Stage<I, O>` or `Step` trait
- A `SourceAdapter` trait for external data services
- A `Pipeline` or `PipelineRunner` orchestrator
- A `Checkpoint` or `StateStore` persistence trait
- A `Normalizer` trait for format conversion

Address: type safety, composability, serializable state, async support, and
error handling. Show actual Rust trait signatures where possible.

### 3. Execution Model Recommendation
Which execution model (or combination) best serves our five pillars?
Consider: topological sort with layers, type-state safety, resource-based
scheduling, conditional execution, checkpointed resume.

Be specific about what we should build vs. what we should skip.

### 4. State Design Recommendation
How should pipeline state be structured to serve as both checkpoint data
AND inspectable system state? Address the three-layer model:
- Specification (immutable after load)
- Topology (immutable after init)
- State (mutates during execution)

Show example struct layouts if useful.

### 5. What We Should NOT Do
Explicit anti-patterns and over-engineering traps identified across the
six crates. For each, explain what the crate did, why it's wrong for us,
and what to do instead.

### 6. Recommended Architecture Sketch
A high-level architecture for the `ecl-*` pipeline runner crates, informed
by the best of all six analyses. Include:
- Proposed crate names and responsibilities
- Key traits and their relationships
- Dependency relationships between crates
- How the three layers (spec/topology/state) map to code structure
- How a Google Drive → markdown pipeline would be expressed

## Instructions

1. Read all six analysis files thoroughly
2. Cross-reference concepts — the same idea may appear in different forms
   across multiple crates
3. Prioritize ruthlessly — we want the best 10-15 concepts, not an
   exhaustive catalog of everything every crate does
4. Keep recommendations concrete — show Rust code where it helps
5. Remember the weight class: CLI tool, not distributed system
6. Produce `pipeline-crate-synthesis.md` following the format above
