# ECL — Extract, Cogitate, Load

[![][build-badge]][build]
[![][crate-badge]][crate]
[![][tag-badge]][tag]
[![][docs-badge]][docs]

[![][logo]][logo-large]

*Far more than agent parallelism, we've found a deep need for "managed serialism" or agent workflow management.*

## What is ECL?

ECL is a Rust-based framework for building **durable AI agent workflows** with explicit control over sequencing, validation, and feedback loops.

While most AI agent frameworks optimize for parallelism—running multiple tools or LLM calls concurrently—ECL addresses a different problem: **workflows that require deliberate, validated sequencing** where each step must complete successfully before the next begins, and downstream steps can request revisions from upstream.

### Core Concepts

**Managed Serialism**: Steps execute in defined order with explicit handoffs. Each step validates its input, performs work (often involving LLM calls), and produces typed output for the next step.

**Feedback Loops**: Downstream steps can request revisions from upstream steps. Iteration is bounded—after N attempts, the workflow fails gracefully with full context.

**Durable Execution**: Every step is journaled. Workflows survive process crashes and resume exactly where they left off without re-executing completed steps.

---

## Why ECL?

### The Problem

Consider this workflow:

```
Step 1: Extract information from documents following specific instructions
Step 2: Review extraction, request revisions if criteria not met (max 3 attempts)
Step 3: Use validated extraction to produce final deliverables
```

This pattern appears everywhere in AI-assisted decision making, planning, and document creation. But existing tools fall short:

- **Agent frameworks** (LangChain, etc.): Optimized for parallelism, not sequential validation
- **Workflow engines** (Airflow, etc.): Designed for data pipelines, not LLM interactions
- **Custom solutions**: Require extensive infrastructure code for durability and state management

### The Solution

ECL provides:

1. **Workflow primitives** built on Restate's durable execution engine
2. **Step abstractions** with built-in retry, feedback, and validation patterns
3. **Clean LLM integration** focused on Anthropic's Claude with provider abstraction
4. **Persistence layer** supporting SQLite (dev) and PostgreSQL (prod)

---

## Architecture Overview

```text
┌───────────────────────────────────────────────────────────┐
│                            ECL                            │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐  │
│  │                   Workflow Layer                    │  │
│  │            (Restate + Step Abstractions)            │  │
│  └───────────────────────────┬─────────────────────────┘  │
│                              │                            │
│         ┌────────────────────┼──────────────────┐         │
│         ▼                    ▼                  ▼         │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐  │
│  │     LLM     │     │ Persistence │     │ Resilience  │  │
│  │  (Claude)   │     │   (SQLx)    │     │  (backon)   │  │
│  └─────────────┘     └─────────────┘     └─────────────┘  │
└───────────────────────────────────────────────────────────┘
```

### Key Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| Orchestration | [Restate](https://restate.dev) | Durable workflow execution |
| LLM Integration | [llm](https://crates.io/crates/llm) | Claude API abstraction |
| Database | [SQLx](https://crates.io/crates/sqlx) | Async SQL with compile-time checks |
| Retry Logic | [backon](https://crates.io/crates/backon) | Exponential backoff |
| Configuration | [figment](https://crates.io/crates/figment) | Hierarchical config |
| Observability | [tracing](https://crates.io/crates/tracing) | Structured logging |

---

## Example: Document Review Pipeline

```rust
#[restate_sdk::workflow]
pub trait DocumentReviewWorkflow {
    /// Main workflow execution — runs exactly once per workflow instance
    async fn run(input: WorkflowInput) -> Result<WorkflowOutput, HandlerError>;

    /// Signal handler for feedback — can be called multiple times
    #[shared]
    async fn submit_feedback(feedback: ReviewFeedback) -> Result<(), HandlerError>;

    /// Query handler for status — can be called anytime
    #[shared]
    async fn get_status() -> Result<WorkflowStatus, HandlerError>;
}

impl DocumentReviewWorkflow for DocumentReviewWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        input: WorkflowInput,
    ) -> Result<WorkflowOutput, HandlerError> {
        // Step 1: Extract — durable, won't re-execute on recovery
        let extraction = ctx.run(|| {
            self.llm.extract(&input.files, &input.instructions)
        }).await?;

        ctx.set("status", "Extraction complete, awaiting review");

        // Step 2: Review with feedback loop
        let mut attempts = 0;
        let validated = loop {
            // Wait for review feedback (durable promise)
            let feedback = ctx.promise::<ReviewFeedback>("review").await?;

            if feedback.approved {
                break extraction.clone();
            }

            attempts += 1;
            if attempts >= input.max_iterations {
                return Err(HandlerError::terminal("Max iterations exceeded"));
            }

            // Revise based on feedback — also durable
            extraction = ctx.run(|| {
                self.llm.revise(&extraction, &feedback.comments)
            }).await?;

            ctx.set("status", format!("Revision {} complete", attempts));
        };

        // Step 3: Produce final output
        let output = ctx.run(|| {
            self.llm.produce(&validated, &input.output_instructions)
        }).await?;

        ctx.set("status", "Complete");
        Ok(output)
    }
}
```

---

## Project Status

🚧 **Early Development** — Architecture validated, implementation in progress.

### Completed

- [x] Architecture design
- [x] Library research and selection
- [x] Dependency validation

### In Progress

- [ ] Core workflow primitives
- [ ] Step abstraction layer
- [ ] LLM integration
- [ ] Persistence layer

### Planned

- [ ] CLI tooling
- [ ] HTTP API
- [ ] Documentation
- [ ] Example workflows

---

## Getting Started

> ⚠️ **Note**: ECL is not yet ready for use. This section will be updated as development progresses.

### Prerequisites

- Rust 1.75+
- [Restate Server](https://docs.restate.dev/get_started/)
- Anthropic API key

### Installation

```bash
# Install Restate server
brew install restatedev/tap/restate-server

# Clone ECL
git clone https://github.com/yourorg/ecl
cd ecl

# Build
cargo build

# Run Restate server
restate-server &

# Run ECL service
cargo run
```

---

## Documentation

- [Architecture Proposal](docs/01_architecture_proposal.md) — System design and conceptual model
- [Library Research](docs/02_library_research.md) — Detailed analysis of chosen libraries
- [Project Proposal](docs/03_proposal_and_justification.md) — Strategic rationale and implementation plan

---

## Contributing

We're not yet accepting external contributions, but will open the project once the core architecture stabilizes.

---

## License

TBD

---

[//]: ---Named-Links---

[logo]: assets/images/logo/v1-x250.png
[logo-large]: assets/images/logo/v1.png
[build]: https://github.com/oxur/ecl/actions/workflows/ci.yml
[build-badge]: https://github.com/oxur/ecl/actions/workflows/ci.yml/badge.svg
[crate]: https://crates.io/crates/ecl
[crate-badge]: https://img.shields.io/crates/v/ecl.svg
[docs]: https://docs.rs/ecl/
[docs-badge]: https://img.shields.io/badge/rust-documentation-blue.svg
[tag-badge]: https://img.shields.io/github/tag/oxur/ecl.svg
[tag]: https://github.com/oxur/ecl/tags
