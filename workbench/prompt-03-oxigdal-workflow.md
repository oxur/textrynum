# Deep Analysis: oxigdal-workflow

## Your Task

Locate and deeply analyze the `oxigdal-workflow` crate, then produce a
structured analysis document. Save the output as `analysis-oxigdal-workflow.md`
in the current working directory.

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

We are **NOT** adopting any crate wholesale. We are mining them for excellent
concepts, design patterns, trait designs, and architectural decisions that we
can pull into our own `ecl-*` crates. The target weight class is "a task runner
with checkpointing" — no Kubernetes, no external databases, no distributed
orchestration servers.

### Three-Layer Architecture

Our design separates configuration into three layers:

- **Specification** — Static declaration from TOML files, immutable during
  execution (where Configurability lives)
- **Topology** — Derived structure: concrete adapter instances, resolved
  connections. Computed at startup, then immutable (where Composability lives)
- **State** — Runtime accumulation of progress, results, failures. Mutates
  during execution (where Durability, Incrementality, and Observability live)

## Source Repository

This crate may not have a public GitHub repo. Try these approaches in order:

1. Check https://lib.rs/crates/oxigdal-workflow for a source link
2. Check https://crates.io/crates/oxigdal-workflow for a repository URL
3. Check https://docs.rs/oxigdal-workflow for source browsing
4. If source is available, clone it. If only docs are available, analyze from
   the docs.rs documentation and API surface.

## What to Focus On

A DAG workflow engine for geospatial processing with **STATE PERSISTENCE**
(save/restore), retry with exponential backoff, and scheduling. Key areas:

- `WorkflowDag` and `TaskNode` design
- State persistence mechanism (save/restore)
- `RetryPolicy` and failure recovery
- `ResourceRequirements` — resource-aware scheduling
- How the scheduling system works (cron, interval, event-driven)
- The RESTful API (optional `server` feature)

## Output Format

Produce `analysis-oxigdal-workflow.md` with these sections:

### 1. Overview
- What problem does it solve?
- How mature is it? (version, activity, downloads)
- What's the core abstraction?

### 2. Trait Design
- What are the key traits? Quote their signatures.
- How are they composed?
- What's the type parameter strategy (generics vs trait objects vs enums)?
- Are there associated types? How are they used?

### 3. DAG / Execution Model
- How are tasks/steps defined?
- How are dependencies expressed?
- How is execution ordered? (topological sort? layers? manual?)
- How is parallelism handled?
- How are results passed between steps?

### 4. State Management
- Is there any checkpointing or persistence?
- How is inter-step data passed? (channels? shared context? typed outputs?)
- Is state serializable?
- How are errors handled and propagated?

### 5. Configuration
- How are pipelines configured? (code only? config files? both?)
- Is there a builder pattern?
- How flexible is the configuration?

### 6. Relevance to Our Five Pillars
For each of our five pillars (Durability, Configurability, Observability,
Incrementality, Composability), assess:
- Does this crate address it? How?
- What can we learn from its approach?
- What would we do differently?

### 7. Extractable Concepts
A prioritized list of specific concepts, patterns, or code designs we should
consider adopting, with rationale for each.

### 8. Anti-Patterns / Warnings
Anything we should explicitly avoid or that represents a design decision we
disagree with, and why.

## Instructions

1. Locate the source (see Source Repository section above)
2. If source is available: read the full codebase starting from `lib.rs`
3. If only docs are available: analyze the public API surface thoroughly from
   docs.rs, noting where you're inferring from docs vs. reading source
4. Pay special attention to the state persistence mechanism — this is
   high-value for our use case
5. Produce `analysis-oxigdal-workflow.md` following the template above
6. Be thorough but concise — focus on what's transferable to our use case
7. If the crate turns out to be unavailable or trivially thin, say so clearly
   and explain what you were able to find
