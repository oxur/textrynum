# ECL

[![][crate-badge]][crate]
[![][docs-badge]][docs]

[![][logo]][logo-large]

*Extract, Cogitate, Load*  • Part of the [Textrynum](README.md) project

ECL addresses **workflows that require deliberate, validated sequencing** — where each step must complete before the next begins, and downstream steps can request revisions from upstream.

## Core Concepts

**Managed Serialism**: Steps execute in defined order with explicit handoffs. Each step validates its input, performs work (often involving LLM calls), and produces typed output for the next step.

**Feedback Loops**: Downstream steps can request revisions from upstream. Iteration is bounded — after N attempts, the workflow fails gracefully with full context.

**Durable Execution**: Every step is journaled. Workflows survive process crashes and resume where they left off.

## Crates

| Crate | Purpose |
|-------|---------|
| `ecl-core` | Core types, traits, error handling, LLM abstractions |
| `ecl-steps` | Step execution framework (in progress) |
| `ecl-workflows` | Workflow orchestration with critique loops |
| `ecl-cli` | Command-line interface (planned) |
| `ecl` | Umbrella re-export crate |

## Key Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| LLM Integration | [llm](https://crates.io/crates/llm) | Claude API abstraction |
| Resilience | [backon](https://crates.io/crates/backon) | Exponential backoff & retry |
| Database | [sqlx](https://crates.io/crates/sqlx) | Async database access |
| Observability | [tracing](https://crates.io/crates/tracing) | Structured logging |

## Status

ECL is early stage. The core types and critique-loop workflow are implemented; the CLI, step library, and durable execution integration are planned.

[//]: ---Named-Links---

[logo]: assets/images/ecl/v2-y250.png
[logo-large]: assets/images/ecl/v2.png
[crate]: https://crates.io/crates/ecl
[crate-badge]: https://img.shields.io/crates/v/ecl.svg
[docs]: https://docs.rs/ecl/
[docs-badge]: https://img.shields.io/badge/rust-documentation-blue.svg
