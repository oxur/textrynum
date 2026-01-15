---
number: 1
title: "ECL Architecture Proposal"
author: "Flink creators"
component: All
tags: [change-me]
created: 2026-01-15
updated: 2026-01-15
state: Under Review
supersedes: null
superseded-by: null
version: 1.0
---

# ECL Architecture Proposal

## "Extract, Cogitate, Load" — Managed Serialism for AI Agent Workflows

**Version**: 1.0
**Date**: January 2026
**Status**: Propositional Architecture (v1 Target)

---

## Executive Summary

This document proposes a Rust-based architecture for ECL, an AI workflow orchestration system designed around the concept of **managed serialism**—the deliberate, controlled sequencing of AI agent steps with bidirectional feedback, conditional iteration, and durable execution guarantees.

The architecture centers on **Restate** as the orchestration layer, providing durable execution, automatic retry, and crash recovery without requiring custom infrastructure code.

---

## Core Design Principles

### 1. Managed Serialism Over Naive Parallelism

Unlike typical "agentic" frameworks that emphasize parallel tool execution, ECL recognizes that many AI workloads require **deliberate sequencing**:

- Step N must complete and pass validation before Step N+1 begins
- Downstream steps can request upstream revision (feedback loops)
- Iteration counts are bounded and tracked
- Failure at any step can trigger rollback, retry, or escalation

### 2. Durable Execution as Foundation

Every step in an ECL workflow is **durable by default**:

- Progress survives process crashes
- Completed steps are never re-executed on recovery
- External API calls (especially LLM calls) are journaled
- Workflows can pause for hours/days waiting for human input or external events

### 3. Clean Separation of Concerns

The architecture enforces strict boundaries between:

- **Workflow Definition** — What steps exist and how they connect
- **Step Implementation** — What each step actually does
- **LLM Integration** — How we communicate with Claude
- **Persistence** — How state is stored and retrieved
- **Transport** — How data flows between components

---

## Conceptual Model

### The Workflow Graph

An ECL workflow is a **directed graph** where:

```
┌──────────────────────────────────────────────────────────────┐
│                         WORKFLOW                             │
│  ┌─────────┐      ┌─────────┐      ┌─────────┐               │
│  │ Step 1  │─────▶│ Step 2  │─────▶│ Step 3  │─────▶ Output  │
│  │ Extract │      │ Review  │      │ Produce │               │
│  └─────────┘      └────┬────┘      └─────────┘               │
│       ▲                │                                     │
│       │                │ feedback                            │
│       └────────────────┘ (bounded iteration)                 │
└──────────────────────────────────────────────────────────────┘
```

**Nodes** are workflow steps that:

- Accept typed input from upstream
- Perform work (possibly involving LLM calls)
- Produce typed output for downstream
- Can request revision from upstream via feedback channels

**Edges** represent:

- Data flow (forward direction)
- Feedback channels (backward direction)
- Conditional routing based on step results

### The Step Lifecycle

Each step follows a defined lifecycle:

```
                    ┌──────────────┐
                    │   PENDING    │
                    └──────┬───────┘
                           │ input received
                           ▼
                    ┌──────────────┐
              ┌────▶│   RUNNING    │◀────┐
              │     └──────┬───────┘      │
              │            │              │
         retry│            │ complete     │ feedback
      (bounded)            ▼              │ received
              │     ┌──────────────┐      │
              └─────│   REVIEW     │──────┘
                    └──────┬───────┘
                           │ approved
                           ▼
                    ┌──────────────┐
                    │  COMPLETED   │
                    └──────────────┘
```

### Feedback Loop Semantics

When Step N+1 reviews Step N's output and finds issues:

1. Step N+1 constructs a `FeedbackRequest` with specific criteria not met
2. The request is sent to Step N via a **Durable Promise**
3. Step N revises its output (iteration counter incremented)
4. Revised output flows back to Step N+1
5. If iteration limit exceeded, workflow enters `FAILED` state with full context

---

## Architectural Layers

### Layer 1: Orchestration (Restate)

Restate provides the execution backbone:

```
┌─────────────────────────────────────────────────────────────────┐
│                     RESTATE SERVER                              │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Journal-based Durable Execution                          │  │
│  │  • Every ctx.run() call is logged before execution        │  │
│  │  • Results are cached and replayed on recovery            │  │
│  │  • Exactly-once semantics for external calls              │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Workflow Primitives                                      │  │
│  │  • run handler: executes once per workflow instance       │  │
│  │  • shared handlers: query/signal running workflows        │  │
│  │  • Durable Promises: cross-handler communication          │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  State Management                                         │  │
│  │  • Per-workflow K/V state                                 │  │
│  │  • Persisted with execution progress                      │  │
│  │  • Queryable from shared handlers                         │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Why Restate?**

- **Single binary** — no database, message queue, or cluster to manage
- **Rust-native server** — high performance, low latency
- **Official Rust SDK** — first-class Tokio support
- **Built by Flink creators** — production-proven distributed systems expertise
- **Durable Promises** — perfect primitive for feedback loops

### Layer 2: Step Abstraction

A thin trait-based abstraction over Restate's primitives:

```
┌───────────────────────────────────────────────────────────────────┐
│                          STEP ABSTRACTION                         │
│                                                                   │
│  ┌──────────────────┐  ┌─────────────────┐  ┌──────────────────┐  │
│  │   StepConfig     │  │   StepContext   │  │   StepResult     │  │
│  │                  │  │                 │  │                  │  │
│  │ • Input schema   │  │ • Restate ctx   │  │ • Output data    │  │
│  │ • Output schema  │  │ • LLM client    │  │ • Feedback req   │  │
│  │ • Retry policy   │  │ • File access   │  │ • Next step      │  │
│  │ • Timeout        │  │ • State access  │  │ • Metadata       │  │
│  └──────────────────┘  └─────────────────┘  └──────────────────┘  │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  Step Trait                                                 │  │
│  │  async fn execute(&self, ctx, input) -> Result<StepResult>  │  │
│  │  async fn revise(&self, ctx, feedback) -> Result<StepResult>│  │
│  │  fn retry_policy(&self) -> RetryPolicy                      │  │
│  └─────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

### Layer 3: LLM Integration

Clean abstraction over Anthropic's Claude API:

```
┌───────────────────────────────────────────────────────────────────┐
│                     LLM INTEGRATION                               │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  LLMProvider Trait                                          │  │
│  │  • async fn complete(&self, request) -> Result<Response>    │  │
│  │  • async fn complete_stream(&self, request) -> Stream       │  │
│  │  • fn model_info(&self) -> ModelInfo                        │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                    │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  ClaudeProvider (Primary Implementation)                    │  │
│  │  • Anthropic Messages API                                   │  │
│  │  • Streaming support                                        │  │
│  │  • Tool/function calling                                    │  │
│  │  • Automatic retry with backoff                             │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  Future: Additional providers can implement LLMProvider trait     │
└───────────────────────────────────────────────────────────────────┘
```

### Layer 4: Persistence

Application-level persistence for workflow metadata and artifacts:

```
┌──────────────────────────────────────────────────────────────────┐
│                      PERSISTENCE                                 │
│                                                                  │
│  ┌───────────────────────┐    ┌───────────────────────────────┐  │
│  │  Restate (Execution)  │    │  SQLx Repository (Metadata)   │  │
│  │                       │    │                               │  │
│  │  • Workflow progress  │    │  • Workflow definitions       │  │
│  │  • Step journals      │    │  • Execution history          │  │
│  │  • Durable promises   │    │  • Artifact references        │  │
│  │  • K/V state          │    │  • Audit logs                 │  │
│  └───────────────────────┘    └───────────────────────────────┘  │
│                                        │                         │
│                               ┌────────┴────────┐                │
│                               ▼                 ▼                │
│                        ┌──────────┐        ┌──────────┐          │
│                        │  SQLite  │        │ Postgres │          │
│                        │  (Dev)   │        │  (Prod)  │          │
│                        └──────────┘        └──────────┘          │
└──────────────────────────────────────────────────────────────────┘
```

**Division of Responsibility:**

| Concern | Owner |
|---------|-------|
| Workflow execution state | Restate |
| Step progress & journals | Restate |
| Durable promises | Restate |
| Workflow definitions | SQLx Repository |
| Execution history/audit | SQLx Repository |
| Large artifacts (docs, files) | Filesystem + SQLx references |

---

## Data Flow: Concrete Example

Mapping to the toy example from the project brief:

### Workflow Definition

```
Workflow: "Document Review Pipeline"
├── Step 1: Extract
│   ├── Input: file paths, instructions
│   ├── LLM: Claude reads files, follows instructions
│   ├── Output: extracted content + analysis
│   └── Feedback: accepts revision requests from Step 2
│
├── Step 2: Review
│   ├── Input: Step 1 output + review criteria
│   ├── Validation: check against criteria
│   ├── If invalid: send FeedbackRequest to Step 1 (max N iterations)
│   ├── If valid: continue
│   ├── LLM: Claude performs additional processing
│   └── Output: reviewed content + additional files
│
└── Step 3: Produce
    ├── Input: Step 2 output + production instructions
    ├── LLM: Claude generates final deliverables
    └── Output: final documents/artifacts
```

### Execution Flow

```
1. Client submits workflow with input files and instructions
                │
                ▼
2. Restate receives workflow, assigns workflow_id
                │
                ▼
3. Step 1 (Extract) runs
   └── ctx.run(|| claude.complete(extraction_prompt))
   └── Result journaled in Restate
                │
                ▼
4. Step 2 (Review) receives Step 1 output
   └── Validates against criteria
   └── If invalid:
       └── ctx.resolve_promise("step1_feedback", revision_request)
       └── Waits on ctx.promise("step1_revised")
       └── Step 1 receives feedback, revises, resolves promise
       └── Loop until valid or iteration limit
   └── If valid: continues with LLM processing
                │
                ▼
5. Step 3 (Produce) receives Step 2 output
   └── ctx.run(|| claude.complete(production_prompt))
   └── Final output stored
                │
                ▼
6. Workflow completes, client retrieves results
```

---

## Failure Handling

### Transient Failures (Network, Rate Limits)

Handled automatically by:

1. Restate's built-in retry with exponential backoff
2. `backon` library for LLM-specific retry policies
3. Circuit breaker (`failsafe`) for API protection

### Step Failures

When a step fails after retries exhausted:

1. Restate converts to `TerminalError`
2. Workflow state captured for debugging
3. Compensating actions triggered if defined
4. Workflow marked as `FAILED` with full context

### Feedback Loop Exhaustion

When iteration limit reached:

1. Last attempt's output preserved
2. All feedback history preserved
3. Workflow enters `FAILED_FEEDBACK_EXHAUSTED` state
4. Human intervention can resume or override

---

## Observability

### Built-in Restate Capabilities

- **Invocation journal inspection** — see exactly what each step did
- **K/V state queries** — inspect workflow state at any point
- **SQL interface** — query invocations, state, and progress
- **HTTP endpoints** — attach to running workflows, query output

### Application-Level

- **Tracing** — structured logging via `tracing` crate
- **Metrics** — step durations, LLM token usage, retry counts
- **Audit trail** — all executions logged to SQLx repository

---

## Deployment Model

### Development

```
┌───────────────────────────────────────────────────┐
│  Developer Machine                                │
│                                                   │
│  ┌─────────────────┐     ┌─────────────────┐      │
│  │ restate-server  │────▶│  ECL Service    │      │
│  │ (single binary) │     │  (Tokio app)    │      │
│  └─────────────────┘     └────────┬────────┘      │
│                                   │               │
│                          ┌────────┴────────┐      │
│                          ▼                 ▼      │
│                   ┌──────────┐      ┌──────────┐  │
│                   │  SQLite  │      │  Claude  │  │
│                   │  (local) │      │   API    │  │
│                   └──────────┘      └──────────┘  │
└───────────────────────────────────────────────────┘
```

### Production

```
┌───────────────────────────────────────────────────┐
│  Production Environment                           │
│                                                   │
│  ┌─────────────────┐     ┌─────────────────┐      │
│  │ restate-server  │────▶│  ECL Service    │      │
│  │ (HA cluster)    │     │  (container)    │      │
│  └─────────────────┘     └────────┬────────┘      │
│                                   │               │
│                          ┌────────┴────────┐      │
│                          ▼                 ▼      │
│                   ┌──────────┐      ┌──────────┐  │
│                   │ Postgres │      │  Claude  │  │
│                   │  (RDS)   │      │   API    │  │
│                   └──────────┘      └──────────┘  │
└───────────────────────────────────────────────────┘
```

---

## Open Questions for v1

1. **Workflow Definition Format**: Code-only (Rust traits) vs. hybrid (code + YAML/TOML)?
2. **Artifact Storage**: Local filesystem sufficient for v1, or need S3-compatible from start?
3. **Multi-tenancy**: Single-tenant v1, or design for multi-tenant from beginning?
4. **Human-in-the-Loop**: How do we expose "waiting for approval" workflows to humans?

---

## Next Steps

1. **Library Research**: Validate component library choices (see companion document)
2. **Prototype**: Build minimal 3-step workflow with feedback loop
3. **Benchmarking**: Measure Restate overhead for LLM-bound workloads
4. **API Design**: Define public API for workflow submission and management
