# ECL Pipeline Redesign — 2026.02.13 Session Bootstrap

## Purpose

This document captures the complete context from a design session (March 2026)
where we identified a fundamental architectural gap in the ECL/Fabryk ecosystem
and designed the approach to fix it. Use this to bootstrap a new Claude session
that can pick up where we left off.

## Who We Are

**Duncan McGreggor** — experienced Rust developer, physicist, longtime
programmer (since age 9), trained in debate with Tibetan monks. Has been
building the ECL/Fabryk ecosystem with Claude since January 2026. Personal
projects include AI skills for music theory, higher mathematics, and advanced
Rust. Work projects use Fabryk for knowledge management.

**Eric Man** — Duncan's colleague and fellow partner in the ECL/Fabryk work.
Present during this session.

## Project Context

### The ECL/Fabryk Ecosystem

- **ECL** ("Extract, Cogitate, Load") — Rust-based AI workflow orchestration
  with managed serialism, bounded iteration, and durable execution
- **Fabryk** — Knowledge fabric framework: 24 crates, 3 search modalities
  (FTS via Tantivy, knowledge graph via petgraph, vector search via LanceDB),
  MCP tool exposure, trait-based extension points
- **Textrynum** — The overall project umbrella name (rebranded from "ecl")
- GitHub: <https://github.com/oxur/ecl> (will be renamed to textrynum)

### What Exists and Works Well

The Fabryk side is substantially realized:

- `fabryk-core` — ConfigProvider trait, AppState, ServiceHandle lifecycle
- `fabryk-content` — Markdown parsing, YAML frontmatter extraction
- `fabryk-fts` — Full-text search with 14-field Tantivy schema
- `fabryk-graph` — Knowledge graph with 10 relationship types, 6 algorithms
- `fabryk-vector` — Vector/semantic search with LanceDB + fastembed
- `fabryk-mcp-*` — MCP tool registries (content, fts, graph, semantic)
- `fabryk-auth` / `fabryk-auth-google` — Token validation, Google OAuth2
- `fabryk-cli` — Extensible CLI with CliExtension trait
- `fabryk-gcp`, `fabryk-redis` — Vendor utility crates

The knowledge synthesis pipeline (PDF → markdown → concept cards → unified
concepts → guides → FTS/graph indexing) works excellently for static sources.

### The Problem Discovered

A recent work project required processing documents from Google Drive, chats
from Slack, and notes from Granola. The team had been jamming ETL
infrastructure (authentication, API client management, pagination, format
normalization, incremental sync) into Fabryk implementations designed for
knowledge synthesis. This caused two weeks of slow, painful, error-prone work.

**Root cause**: The architecture has no extraction/ingestion layer. Everything
from `fabryk-content` upward assumes structured markdown files already exist on
a filesystem. The "E" of ECL was trivial when sources were PDFs on disk, so it
was never built out properly.

**Additional insight**: External data sources (Google Drive, Slack, etc.) are
organized chaotically by humans. There's no amount of AI cleverness that will
divine organizational intent from folders like "Q3 Stuff" or "asdf". The
extraction layer must be configuration-driven, not inference-driven.

## The Design We Arrived At

### Five Pillars

Every design decision must trace back to at least one of these:

1. **Durability** — Checkpoint/resume on failure; long-running pipelines must
   survive crashes without re-executing completed work
2. **Configurability** — Declarative, per-source configuration for source
   selection (include/exclude), format handling, and processing parameters
3. **Observability** — Serializable state that humans or AI can inspect to
   understand exactly what happened, what failed, and why
4. **Incrementality** — Skip unchanged items across runs via content hashing
   or timestamps; efficiency across runs, not just within runs
5. **Composability** — Mix-and-match stages per source type; not every source
   needs every transformation step

### Three Layers

The "configuration" concept was decomposed into three distinct concerns:

1. **Specification** — Static declaration of what the pipeline *is*. Comes from
   TOML files, doesn't change during execution. Where **Configurability** lives.
2. **Topology** — Derived structure of the system once specification is resolved.
   Concrete adapter instances, resolved connections. Computed at startup, then
   immutable. Where **Composability** lives.
3. **State** — Runtime accumulation of what has happened, what's in progress,
   what's left. Mutates during execution. Where **Durability**,
   **Incrementality**, and **Observability** live.

### Key Design Insight: State IS the Pipeline

The pipeline's serialized state struct should be the complete, inspectable,
resumable truth. When handed to Claude, it should contain enough richly-typed,
well-named data that Claude can perform a full conceptual analysis of the
system's state without needing logs, dashboards, or database queries.

This is the same principle as Fabryk's self-describing MCP services and
SKILL.md pattern: make the system structurally transparent to AI.

### Weight Class Decision

We explicitly rejected heavy infrastructure:

- **No Restate** — Too heavy; a separate server process for distributed
  workflow orchestration when we need a CLI pipeline tool
- **No external database** — Filesystem-based state persistence
- **No Kubernetes** — This is a tool, not a service

The right weight class: "a task runner with checkpointing." Run on a beefy
local machine. Configure, point at a data source, run, monitor, iterate.

### PoC Scope

One source adapter (Google Drive) + minimal pipeline infrastructure:

- Validate the specification layer (Drive config with include/exclude)
- Validate the topology layer (adapter → normalize → output)
- Validate the state layer (checkpointing, resume, progress tracking)
- No AI stages, no concept extraction, no graph building
- Adding the second adapter (Slack/Granola) tests whether abstractions hold

### Relationship to Existing ECL

The original ECL concepts (managed serialism, bounded iteration, durable
execution) are sound. The *weight class* was off. The new pipeline runner
should deliver the same five properties with ~500-1000 lines of custom Rust
atop proven crates, not via Restate + Postgres + Kubernetes.

Doc 0004 (ECL Project Plan) will be superseded by whatever we create next.

## Research Results: Rust Ecosystem Assessment

### Conclusion: Build a Custom Pipeline Runner

No single Rust crate delivers all five properties in a lightweight CLI package.
The two closest (erio-workflow, oxigdal-workflow) are both v0.1.0 with narrow
domain focus. The recommendation: buy the components, build the glue.

### Recommended Component Stack

| Pillar | Crate(s) |
|--------|----------|
| Durability | `redb` (v3.1.0) — pure Rust, single-file, ACID, crash recovery |
| Configurability | `toml` + `serde` + custom PipelineConfig structs |
| Observability | Serializable state in redb + `tracing` for structured logging |
| Incrementality | `blake3` (v1.8.2) — content hashing, stored in redb |
| Composability | Custom `Stage<I, O>` trait inspired by Tower's Service pattern |

### Source Adapter Crates

| Service | Crate(s) |
|---------|----------|
| Google Drive | `google-drive3` (v7.0) + `yup-oauth2` (v12.1) |
| Google Sheets | `google-sheets4` (v7.0) |
| Slack | `slack-morphism` (v2.17) + `oauth2` (v5.0) |
| HTTP resilience | `reqwest-middleware` + `reqwest-retry` or `backon` |

## Next Step: Deep Analysis of Existing Workflow Crates

Before building, we want to extract every excellent concept and design decision
from six Rust workflow/DAG crates that could inform our `ecl-*` crate design.

### Repos to Analyze

| Crate | GitHub URL |
|-------|-----------|
| dagrs | <https://github.com/dagrs-dev/dagrs> |
| erio-workflow | <https://github.com/NomanworkGroup/erio> (crates/workflow) |
| oxigdal-workflow | (published on crates.io, check lib.rs for source) |
| dagx | <https://github.com/swaits/dagx> |
| dagga | <https://github.com/Ratysz/dagga> |
| rettle | <https://github.com/slaterb1/rettle> |

### Analysis Approach

1. Create per-crate analysis prompts for Claude Code (clone repos, deep read)
2. Create a synthesis prompt that combines all six analyses
3. Bring results back to design the `ecl-*` pipeline runner

See companion file: `cc-pipeline-crate-analysis-prompts.md`

## Key Documents

| Doc | Description | Status |
|-----|-------------|--------|
| 0004 | ECL Project Plan | Will be superseded |
| 0009 | Unified Ecosystem Vision | Still valid for Fabryk; missing extraction layer |
| architecture.yaml | Current 24-crate architecture | Accurate; shows the gap |

## Key Past Conversations (in this Claude project)

- "Rust AI workflow framework project planning" — Original ECL/Fabryk design
- "Converging knowledge frameworks and AI skill architecture" — Music theory → Fabryk extraction
- "Designing MCP servers for cross-domain knowledge integration" — Hub/spoke, discoverability
- "Service orchestrator for Fabryk MCP discovery" — Most recent (March 2026)
- "Rebranding ECL project and crates" — Textrynum naming, visual identity
- "Fabryk knowledge graph pipeline architecture" — Academic pipeline formalization
- "Knowledge pipeline design and ontological foundations" — Literature review

---

*This bootstrap doc was created during the session where the five pillars, three
layers, and PoC scope were established. The next session should begin with this
document, then proceed to crate analysis or directly to pipeline design.*
