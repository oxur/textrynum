# Textrynum

[![][build-badge]][build]
[![][tag-badge]][tag]

[![][logo]][logo-large]

*A Rust workspace for weaving knowledge into searchable, interconnected fabric.*

## What is Textrynum?

Textrynum (from the Roman *textrinum* — a weaving workshop) is a Rust workspace for building **knowledge systems**. It houses two complementary layers:

1. **Fabryk** — A modular knowledge fabric: ingest content, build knowledge graphs, index for full-text and semantic search, and serve it all via MCP tools. Production-ready, 24,000+ lines, 14 crates.

2. **ECL** (Extract, Cogitate, Load) — A workflow orchestration engine for durable AI agent pipelines with feedback loops, validation, and managed serialism. Early stage, building on solid foundations.

The metaphor: Textrynum is the workshop. Fabryk is the fabric it produces. ECL is the loom.

---

## Fabryk — The Knowledge Fabric

Fabryk turns structured content (Markdown with YAML frontmatter) into a multi-modal knowledge store: a **graph** of relationships, a **full-text search** index, and a **vector space** for semantic similarity — all exposed via **MCP tools** and a **CLI**.

### What it does

- **Content ingestion** — Parse Markdown files with YAML frontmatter, extract sections, resolve sources
- **Knowledge graph** — Build and traverse relationship graphs (prerequisites, extensions, related concepts) with petgraph
- **Full-text search** — Tantivy-backed search with category filtering, incremental indexing, and freshness tracking
- **Vector/semantic search** — LanceDB + fastembed for local embedding generation and similarity search
- **Hybrid search** — Reciprocal rank fusion combining FTS and vector results
- **MCP server** — 25+ tools for AI assistants to query the knowledge base via Model Context Protocol
- **CLI** — Command-line interface for graph operations, search, and configuration
- **Auth** — OAuth2 with Google provider, JWT validation, RFC 9728 discovery

### Crate Map

| Tier | Crate | Purpose |
|------|-------|---------|
| Foundation | `fabryk-core` | Shared types, traits, error handling |
| Content | `fabryk-content` | Markdown parsing, frontmatter extraction |
| Search | `fabryk-fts` | Full-text search (Tantivy backend) |
| Search | `fabryk-graph` | Knowledge graph storage & traversal (petgraph) |
| Search | `fabryk-vector` | Vector/semantic search (LanceDB + fastembed) |
| Auth | `fabryk-auth` | Token validation & Tower middleware |
| Auth | `fabryk-auth-google` | Google OAuth2 / JWKS provider |
| Auth | `fabryk-auth-mcp` | RFC 9728 OAuth2 discovery endpoints |
| MCP | `fabryk-mcp` | Core MCP server infrastructure (rmcp) |
| MCP | `fabryk-mcp-content` | Content & source MCP tools |
| MCP | `fabryk-mcp-fts` | Full-text search MCP tools |
| MCP | `fabryk-mcp-graph` | Graph query MCP tools |
| CLI | `fabryk-cli` | CLI framework with graph commands |
| ACL | `fabryk-acl` | Access control (placeholder) |
| Umbrella | `fabryk` | Re-exports everything, feature-gated |

Use the umbrella crate to pull in what you need:

```toml
# Just graph and FTS
fabryk = { version = "0.1", features = ["graph", "fts-tantivy"] }

# Everything including MCP server
fabryk = { version = "0.1", features = ["full"] }
```

---

## ECL — The Loom

ECL addresses **workflows that require deliberate, validated sequencing** — where each step must complete before the next begins, and downstream steps can request revisions from upstream.

### Core Concepts

**Managed Serialism**: Steps execute in defined order with explicit handoffs. Each step validates its input, performs work (often involving LLM calls), and produces typed output for the next step.

**Feedback Loops**: Downstream steps can request revisions from upstream. Iteration is bounded — after N attempts, the workflow fails gracefully with full context.

**Durable Execution**: Every step is journaled. Workflows survive process crashes and resume where they left off.

ECL is early stage. The core types and critique-loop workflow are implemented; the CLI, step library, and Restate integration are planned.

---

## Architecture

```text
Textrynum
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  ECL (the loom)                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                      Workflow Layer                        │  │
│  │          (Steps, Feedback Loops, Journaling)               │  │
│  └────────────────────────────┬───────────────────────────────┘  │
│                               │                                  │
│  Fabryk (the fabric)          v                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    Knowledge Fabric                        │  │
│  │                                                            │  │
│  │   ┌──────────┐    ┌──────────┐    ┌──────────┐             │  │
│  │   │  Graph   │    │   FTS    │    │  Vector  │             │  │
│  │   │(petgraph)│    │(Tantivy) │    │(LanceDB) │             │  │
│  │   └──────────┘    └──────────┘    └──────────┘             │  │
│  │         │               │               │                  │  │
│  │         └───────────────┼───────────────┘                  │  │
│  │                         v                                  │  │
│  │              ┌────────────────────┐                        │  │
│  │              │    MCP Server      │                        │  │
│  │              │  (Tool Interface)  │                        │  │
│  │              └────────────────────┘                        │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │   LLM    │  │   Auth   │  │  Config  │  │       CLI        │  │
│  │ (Claude) │  │(OAuth2)  │  │ (confyg) │  │     (clap)       │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### Key Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| Knowledge Graph | [petgraph](https://crates.io/crates/petgraph) | Graph data structures & algorithms |
| Full-Text Search | [Tantivy](https://crates.io/crates/tantivy) | Rust-native search engine |
| Vector Search | [LanceDB](https://lancedb.com/) | Embedded vector database |
| Embeddings | [fastembed](https://crates.io/crates/fastembed) | Local embedding generation |
| MCP Server | [rmcp](https://crates.io/crates/rmcp) | Model Context Protocol |
| Auth | [jsonwebtoken](https://crates.io/crates/jsonwebtoken) | JWT validation |
| CLI | [clap](https://crates.io/crates/clap) | Command-line parsing |
| Configuration | [confyg](https://crates.io/crates/confyg) | Hierarchical config |
| LLM Integration | [llm](https://crates.io/crates/llm) | Claude API abstraction |

---

## Project Status

**v0.1.0** — Fabryk is functional; ECL is in progress.

### Completed

- [x] Knowledge graph with traversal algorithms (fabryk-graph)
- [x] Full-text search with Tantivy backend (fabryk-fts)
- [x] Vector/semantic search with LanceDB (fabryk-vector)
- [x] Markdown content parsing and frontmatter extraction (fabryk-content)
- [x] MCP server infrastructure and tool suites (fabryk-mcp-*)
- [x] OAuth2 authentication with Google provider (fabryk-auth-*)
- [x] CLI framework with graph commands (fabryk-cli)
- [x] Configuration infrastructure with TOML support
- [x] CI/CD pipeline

### In Progress

- [ ] ECL workflow primitives
- [ ] Step abstraction layer with feedback loops
- [ ] LLM integration
- [ ] Connecting ECL workflows to Fabryk persistence

### Planned

- [ ] Access control layer (fabryk-acl)
- [ ] Additional MCP tool suites
- [ ] Example workflows
- [ ] Published crate documentation

---

## Getting Started

### Prerequisites

- Rust 1.75+

### Building

```bash
git clone https://github.com/oxur/textrynum
cd textrynum
cargo build
```

### Testing

```bash
cargo test --workspace --all-features
```

---

## Contributing

We're not yet accepting external contributions, but will open the project once the core architecture stabilizes.

---

## License

Apache-2.0

---

[//]: ---Named-Links---

[logo]: assets/images/logo/v1-x250.png
[logo-large]: assets/images/logo/v1.png
[build]: https://github.com/oxur/textrynum/actions/workflows/cicd.yml
[build-badge]: https://github.com/oxur/textrynum/actions/workflows/cicd.yml/badge.svg
[tag-badge]: https://img.shields.io/github/tag/oxur/textrynum.svg
[tag]: https://github.com/oxur/textrynum/tags
