# Deep Analysis: dagga

## Your Task

Clone and deeply analyze the `dagga` crate, then produce a structured analysis
document. Save the output as `analysis-dagga.md` in the current working
directory.

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

```
git clone https://github.com/Ratysz/dagga
```

## What to Focus On

This is a DAG scheduler with an **architecturally different** dependency model.
Instead of explicit task-to-task edges, dependencies are **implicit via resource
access patterns**. Key areas:

- The **resource-based dependency model** — how do create/read/write/consume
  semantics replace explicit DAG wiring?
- The four resource access patterns: create, read, write, consume
- **Parallel batch scheduling** — how are batches computed? What determines
  which tasks can run concurrently?
- How conflict detection works (write/write, write/read, etc.)
- How this compares to explicit DAG wiring for our use case

Evaluate whether this resource-access model is useful for pipeline stages that
share filesystem paths, API connections, or database handles. Our pipeline
stages often contend on shared resources (e.g., a Google Drive API client with
rate limits, a filesystem output directory, a redb database).

## Output Format

Produce `analysis-dagga.md` with these sections:

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

1. Clone the repo
2. Start with `src/lib.rs` for the public API surface
3. Focus on understanding the resource access semantics — this is the novel
   contribution of this crate
4. Read any examples or tests that show how resource declarations translate
   into execution order
5. Think about how this model maps to our pipeline's shared resources (API
   clients, filesystem, databases)
6. Produce `analysis-dagga.md` following the template above
7. Be thorough but concise — focus on what's transferable to our use case
