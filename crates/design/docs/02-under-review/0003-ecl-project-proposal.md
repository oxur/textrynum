---
number: 3
title: "ECL Project Proposal"
author: "the original"
component: All
tags: [change-me]
created: 2026-01-15
updated: 2026-01-15
state: Under Review
supersedes: null
superseded-by: null
version: 1.0
---

# ECL Project Proposal

## Path Forward for Rust-Based AI Workflow Orchestration

**Version**: 1.0
**Date**: January 2026
**Status**: Proposal for Review

---

## Executive Summary

After comprehensive research into the Rust ecosystem for building an AI agent workflow orchestration system, we recommend proceeding with **Restate** as the orchestration backbone, supported by a carefully selected stack of production-ready libraries.

This proposal outlines:

1. The strategic rationale for the chosen approach
2. Key findings from ecosystem research
3. Identified risks and mitigations
4. Recommended next steps

---

## Strategic Rationale

### The Problem We're Solving

The project tagline captures it precisely:

> *"Extract, Cogitate, Load" — Far more than agent parallelism, we've found a deep need for "managed serialism" or agent workflow management*

Current AI agent frameworks optimize for **parallelism**—running multiple tools or LLM calls concurrently. But real-world AI workloads for decision making, planning, and document creation require something different:

1. **Sequential Validation**: Step N+1 must validate Step N's output before proceeding
2. **Bounded Iteration**: Feedback loops must have hard limits to prevent infinite revision cycles
3. **Durability**: Long-running workflows (hours, days) must survive failures
4. **Auditability**: Every decision and revision must be traceable

These requirements point toward **workflow orchestration** rather than **agent frameworks**.

### Why Rust?

Given the requirements above, Rust offers:

- **Performance**: LLM calls are I/O-bound, but orchestration overhead matters at scale
- **Reliability**: Memory safety without garbage collection pauses
- **Ecosystem Maturity**: Tokio provides a battle-tested async foundation
- **Type Safety**: Catch workflow definition errors at compile time

### Why Not Existing Solutions?

| Solution | Limitation |
|----------|------------|
| Python agent frameworks (LangChain, etc.) | Parallelism-focused; weak durability |
| Temporal | Rust SDK in alpha; Python/Go SDKs mature but wrong language |
| Custom Actor System | Would require 1000+ lines of durability code |

---

## Key Research Findings

### Finding 1: Restate Provides the Core Primitives We Need

Restate is a durable execution engine built by the original creators of Apache Flink. Its Rust SDK provides exactly the primitives required for managed serialism:

**Durable Execution**: Every operation wrapped in `ctx.run()` is journaled before execution. On recovery, completed operations replay from the journal without re-execution. This is critical for expensive LLM calls.

**Workflows as First-Class Citizens**: Restate's `#[workflow]` macro provides:

- Run-once semantics per workflow instance
- Shared handlers for querying and signaling
- Built-in K/V state isolated to each workflow

**Durable Promises**: The perfect primitive for feedback loops:

```
Step 1 waits on: ctx.promise("revision_request")
Step 2 signals:  ctx.resolve_promise("revision_request", feedback)
```

**Operational Simplicity**: Single binary with no external dependencies. Can run on a developer laptop or scale to production clusters.

### Finding 2: The LLM Integration Gap is Manageable

There is no official Anthropic Rust SDK, but the community has filled the gap:

- **llm crate**: Multi-provider abstraction with Claude support, streaming, tool calling
- **async-anthropic**: Anthropic-specific client with full API coverage

Both are actively maintained and production-viable. We recommend the `llm` crate for its provider abstraction, which aligns with the principle of clean separation—even though we're Claude-focused for v1.

### Finding 3: SQLx is the Clear Choice for Persistence

SQLx provides:

- Compile-time query validation
- Native async Tokio support
- Same API across SQLite (dev) and PostgreSQL (prod)
- In-memory SQLite for testing

Alternatives like Diesel lack async SQLite support; SeaORM adds unnecessary abstraction for our needs.

### Finding 4: The Supporting Ecosystem is Mature

Every other component has well-established solutions:

| Need | Solution | Maturity |
|------|----------|----------|
| Retry with backoff | backon | Part of OpenDAL project |
| Circuit breaker | failsafe | Production-ready |
| Configuration | figment | Used by Rocket framework |
| Observability | tracing | Standard in Tokio ecosystem |

---

## Risk Assessment

### Risk 1: Restate Rust SDK Maturity

**Risk**: The Rust SDK (v0.7) is newer than TypeScript/Java SDKs.

**Mitigation**:

- SDK is actively maintained by Restate team
- Core concepts are identical across SDKs
- Step implementations remain behind trait abstractions for future migration
- Start with simpler workflows, validate before complex use cases

**Assessment**: Low-Medium risk. The SDK covers all our required features.

### Risk 2: BSL License

**Risk**: Restate server uses Business Source License 1.1.

**Mitigation**:

- BSL converts to Apache 2.0 after 4 years
- Only restricts offering Restate-as-a-service (competing with their cloud)
- Our use case (internal orchestration) is explicitly permitted
- SDKs are MIT licensed

**Assessment**: Low risk for our use case.

### Risk 3: External Process Dependency

**Risk**: Restate requires running `restate-server` alongside our application.

**Mitigation**:

- Single binary with no dependencies
- Well-documented deployment patterns
- Can containerize alongside our service
- Alternative (building custom durability) would be 10x the work

**Assessment**: Acceptable tradeoff.

### Risk 4: LLM Library Stability

**Risk**: Community LLM libraries may have breaking changes or abandonment.

**Mitigation**:

- Wrap behind trait abstraction
- Pin specific versions
- Monitor library health
- Alternative libraries available if needed

**Assessment**: Low-Medium risk.

---

## Proposed Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                          ECL System                             │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                     Workflow Layer                        │  │
│  │         (Restate Workflows + Custom Step Traits)          │  │
│  └───────────────────────────────────────────────────────────┘  │
│                             │                                   │
│         ┌───────────────────┼──────────────────┐                │
│         ▼                   ▼                  ▼                │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐        │
│  │ LLM Layer   │     │ Persistence │     │ Resilience  │        │
│  │ (llm crate) │     │   (SQLx)    │     │  (backon)   │        │
│  └─────────────┘     └─────────────┘     └─────────────┘        │
│         │                   │                   │               │
│         └───────────────────┴───────────────────┘               │
│                             │                                   │
│                    ┌────────┴────────┐                          │
│                    ▼                 ▼                          │
│              ┌──────────┐      ┌──────────┐                     │
│              │  Claude  │      │ SQLite/  │                     │
│              │   API    │      │ Postgres │                     │
│              └──────────┘      └──────────┘                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │  Restate Server  │
                    │  (orchestration) │
                    └──────────────────┘
```

---

## Recommended Path Forward

### Phase 1: Foundation (Weeks 1-2)

**Objective**: Validate core architecture with minimal implementation.

1. Set up project structure with proposed dependencies
2. Implement basic Restate workflow with 2 steps
3. Integrate Claude API via llm crate
4. Demonstrate durable execution (kill process, verify recovery)
5. Demonstrate feedback loop via Durable Promises

**Deliverable**: Working prototype that exercises all core patterns.

### Phase 2: Step Abstraction (Weeks 3-4)

**Objective**: Build reusable step framework.

1. Define `Step` trait with execute/revise methods
2. Implement retry policy configuration
3. Add SQLx persistence for workflow metadata
4. Build 3-step workflow matching the toy example
5. Add comprehensive tracing/observability

**Deliverable**: Reusable step framework with example workflow.

### Phase 3: API & Tooling (Weeks 5-6)

**Objective**: Make it usable.

1. Define workflow submission API (HTTP or CLI)
2. Implement workflow status querying
3. Add configuration via figment
4. Build basic documentation
5. Create developer setup guide

**Deliverable**: Functional system ready for internal use.

### Phase 4: Hardening (Weeks 7-8)

**Objective**: Production readiness.

1. Add comprehensive error handling
2. Implement circuit breaker for Claude API
3. Add integration tests with in-memory SQLite
4. Performance testing and optimization
5. Document deployment patterns

**Deliverable**: Production-ready v1.

---

## Success Criteria for v1

1. **Durability**: Workflow survives process restart mid-execution
2. **Feedback Loops**: Step N+1 can request revision from Step N with bounded iteration
3. **Observability**: Full trace of all steps, LLM calls, and decisions
4. **Clean Separation**: LLM provider swappable via trait implementation
5. **Developer Experience**: Simple workflow definition in Rust code

---

## Open Questions Requiring Decision

### Q1: Workflow Definition Format

**Option A**: Pure Rust (traits + structs)

- Pro: Type safety, IDE support, refactoring tools
- Con: Requires recompilation for workflow changes

**Option B**: Hybrid (Rust steps + YAML/TOML workflow graphs)

- Pro: Change workflows without recompilation
- Con: Runtime errors, weaker type safety

**Recommendation**: Start with Option A for v1. Add Option B later if needed.

### Q2: Multi-Tenancy

**Option A**: Single-tenant (one workflow namespace)

- Pro: Simpler implementation
- Con: Limits future use cases

**Option B**: Multi-tenant from start

- Pro: Future-proof
- Con: Added complexity

**Recommendation**: Design for multi-tenancy (separate workflow IDs by tenant), implement single-tenant for v1.

### Q3: Artifact Storage

**Option A**: Local filesystem

- Pro: Simple, no additional infrastructure
- Con: Doesn't scale, no redundancy

**Option B**: S3-compatible storage

- Pro: Scalable, redundant
- Con: Additional dependency

**Recommendation**: Local filesystem for v1 with abstraction layer for future S3 support.

---

## Conclusion

The Rust ecosystem provides all the components needed to build ECL. Restate's durable execution model is an excellent fit for managed serialism, eliminating the need for custom orchestration infrastructure. The supporting libraries (SQLx, llm, backon, figment, tracing) are mature and well-maintained.

The primary risks are manageable: Restate's Rust SDK maturity is acceptable given active maintenance and our ability to abstract, and the BSL license is permissive for our use case.

We recommend proceeding with the proposed architecture and phased implementation plan.

---

## References

### Primary Documentation

- Restate Documentation: <https://docs.restate.dev>
- Restate Rust SDK: <https://docs.rs/restate-sdk>
- SQLx Documentation: <https://docs.rs/sqlx>

### Library Repositories

- Restate: github.com/restatedev/restate
- Restate Rust SDK: github.com/restatedev/sdk-rust
- llm crate: github.com/graniet/llm
- SQLx: github.com/launchbadge/sqlx
- backon: github.com/Xuanwo/backon
- figment: github.com/SergioBenitez/Figment

### Supporting Research

- "Building a Modern Durable Execution Engine" (Restate blog)
- "What is Durable Execution?" (Restate documentation)
- Rust Actor Library Comparison (community analysis)

---

## Appendices

See companion documents:

- **01_architecture_proposal.md**: Detailed architecture design
- **02_library_research.md**: Comprehensive library analysis
