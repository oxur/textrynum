# Pipeline Crate Deep Analysis — Claude Code Prompts

## Context for All Prompts

We are designing a lightweight CLI pipeline runner in Rust for the ECL/Fabryk
ecosystem. The pipeline must deliver five properties: **Durability** (checkpoint/
resume), **Configurability** (declarative TOML-based), **Observability**
(serializable inspectable state), **Incrementality** (content-hash skip),
**Composability** (mix-and-match stages).

We are NOT adopting any of these crates wholesale. We are mining them for
excellent concepts, design patterns, trait designs, and architectural decisions
that we can pull into our own `ecl-*` crates.

For each crate, produce a structured analysis covering the sections below. Be
thorough but concise — focus on what's transferable to our use case.

---

## Per-Crate Analysis Template

For each crate, produce a markdown file named `analysis-{crate-name}.md` with
these sections:

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

For each of our five pillars, assess:

- Does this crate address it? How?
- What can we learn from its approach?
- What would we do differently?

### 7. Extractable Concepts

A prioritized list of specific concepts, patterns, or code designs we should
consider adopting, with rationale for each.

### 8. Anti-Patterns / Warnings

Anything we should explicitly avoid or that represents a design decision we
disagree with, and why.

---

## Prompt 1: dagrs

```
Clone: https://github.com/dagrs-dev/dagrs

Read the entire codebase. This is a Flow-Based Programming framework for async
DAG task orchestration. Key areas of interest:

- The `Action` / `Node` / `Graph` type hierarchy
- The `Parser` trait and its TOML/YAML/JSON support
- The `Condition` trait for conditional execution
- The `InChannels` / `OutChannels` inter-task communication
- The `EnvVar` shared environment
- Loop DAG support (cyclic dependencies — unusual!)
- The derive macro (`dagrs-derive`)

Note: The repo was archived January 2026 but crate publishing continues from
a new location. Assess the code quality regardless of maintenance status.

Produce: analysis-dagrs.md (following the template above)
```

## Prompt 2: erio-workflow

```
Clone: https://github.com/NomanworkGroup/erio
Focus on: crates/workflow/

This is a DAG workflow engine for AI agent orchestration with CHECKPOINTING —
the only crate we found with explicit checkpoint/resume support. Key areas:

- The `Checkpoint` module — how does it persist and restore?
- The `WorkflowContext` — how is typed data passed between steps?
- The `Step` trait and `StepOutput`
- The `Dag` module — dependency resolution
- The `WorkflowEngine` — parallel step execution
- The `Workflow` builder pattern
- The `conditional` module — runtime predicates

Also briefly review crates/core/ for shared types (Message, RetryConfig,
error handling) as these inform the broader ecosystem design.

Produce: analysis-erio-workflow.md (following the template above)
```

## Prompt 3: oxigdal-workflow

```
This crate may not have a public GitHub repo — check crates.io and lib.rs for
source links. If source is available, clone it. Otherwise, analyze from docs.rs
documentation.

Focus on: https://lib.rs/crates/oxigdal-workflow

A DAG workflow engine for geospatial processing with STATE PERSISTENCE
(save/restore), retry with exponential backoff, and scheduling. Key areas:

- `WorkflowDag` and `TaskNode` design
- State persistence mechanism (save/restore)
- `RetryPolicy` and failure recovery
- `ResourceRequirements` — resource-aware scheduling
- How the scheduling system works (cron, interval, event-driven)
- The RESTful API (optional `server` feature)

Produce: analysis-oxigdal-workflow.md (following the template above)
```

## Prompt 4: dagx

```
Clone: https://github.com/swaits/dagx

This is a minimal, type-safe DAG executor with COMPILE-TIME cycle prevention
via type-state patterns. Key areas of interest:

- The type-state pattern for cycle prevention — how does it work?
- The `#[task]` proc macro — what does it generate?
- The `DagRunner` and `TaskHandle<T>` design
- Three task patterns: Stateless, Read-only (&self), Mutable (&mut self)
- The `depends_on()` API with tuple-based multi-dependency
- Runtime-agnostic execution (works with any async runtime via spawner fn)
- Zero-cost tracing behind feature flags

This crate has some of the most elegant Rust API design in the set. Pay
special attention to how it achieves type safety without runtime overhead.

Produce: analysis-dagx.md (following the template above)
```

## Prompt 5: dagga

```
Clone: https://github.com/Ratysz/dagga

A DAG scheduler focused on create/read/write/consume resource semantics with
parallel batch scheduling. Key areas:

- Resource-based dependency model (not explicit task-to-task deps)
- The create/read/write/consume resource access patterns
- Parallel batch scheduling — how are batches computed?
- How does it differ from explicit DAG wiring?

This is architecturally different from the others — dependencies are implicit
via resource access patterns rather than explicit edges. Evaluate whether this
model is useful for pipeline stages that share filesystem or API resources.

Produce: analysis-dagga.md (following the template above)
```

## Prompt 6: rettle

```
Clone: https://github.com/slaterb1/rettle

A Keras-inspired ETL framework with Fill/Transform/Pour ingredient types and
multithreaded execution. Key areas:

- The "ingredient" abstraction (Fill, Transform, Pour)
- The "pot" / "brewer" / "rettle" metaphor and how it maps to ETL
- Multithreaded execution model
- How data flows through the pipeline
- Whether there's any persistence or checkpointing

This is the most ETL-specific crate in the set. Even if the implementation
is dated, the domain modeling may have useful insights for our extraction
pipeline.

Produce: analysis-rettle.md (following the template above)
```

---

## Synthesis Prompt

```
You have six analysis documents:
- analysis-dagrs.md
- analysis-erio-workflow.md
- analysis-oxigdal-workflow.md
- analysis-dagx.md
- analysis-dagga.md
- analysis-rettle.md

Produce a synthesis document: pipeline-crate-synthesis.md

Structure:

## 1. Concept Inventory
A table of every extractable concept across all six crates, deduplicated,
with source crate(s) and relevance rating (HIGH / MEDIUM / LOW) for our
five pillars.

## 2. Trait Design Recommendations
Based on the best patterns seen across all crates, recommend the core trait
signatures for our pipeline runner. Consider:
- A `Stage<I, O>` or `Step` trait
- A `SourceAdapter` trait for external data services
- A `Pipeline` or `PipelineRunner` orchestrator
- A `Checkpoint` or `StateStore` persistence trait
Address type safety, composability, and serializable state.

## 3. Execution Model Recommendation
Which execution model (or combination) best serves our five pillars?
Consider: topological sort with layers, type-state safety, resource-based
scheduling, conditional execution, checkpointed resume.

## 4. State Design Recommendation
How should pipeline state be structured to serve as both checkpoint data
AND inspectable system state? Address the three-layer model:
- Specification (immutable after load)
- Topology (immutable after init)
- State (mutates during execution)

## 5. What We Should NOT Do
Explicit anti-patterns and over-engineering traps identified across the
six crates.

## 6. Recommended Architecture Sketch
A high-level architecture for the `ecl-*` pipeline runner crates,
informed by the best of all six analyses. Include proposed crate names,
key traits, and dependency relationships.
```

---

## Execution Instructions

1. Clone all repos to a working directory
2. Run prompts 1-6 (can be parallelized across CC sessions)
3. Gather all six `analysis-*.md` files
4. Run the synthesis prompt with all six as input
5. Bring `pipeline-crate-synthesis.md` back to the design session
