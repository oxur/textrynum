# Textrynum

[![][build-badge]][build]
[![][tag-badge]][tag]
[![][license badge]][license]

[![][logo]][logo-large]

*A workspace and tools for weaving knowledge into a hyper-connected, searchable fabric*

## What is Textrynum?

Textrynum is a workspace for building **knowledge systems**. It currently houses two complementary layers:

1. **[Fabryk](README-fabryk.md)** — A modular knowledge fabric: ingest content, build knowledge graphs, index for full-text and semantic search, and serve it all via MCP tools. Production-ready, 24,000+ lines, 14 crates.

2. **[ECL](README-ecl.md)** (Extract, Cogitate, Load) — A workflow orchestration engine for durable AI agent pipelines with feedback loops, validation, and managed serialism. Early stage, building on solid foundations.

---

## Architecture

```text
Textrynum
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  ECL                                                             │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                      Workflow Layer                        │  │
│  │          (Steps, Feedback Loops, Journaling)               │  │
│  └────────────────────────────┬───────────────────────────────┘  │
│                               │                                  │
│  Fabryk                       v                                  │
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

[logo]: assets/images/logo/v1-y250.png
[logo-large]: assets/images/logo/v1.png
[build]: https://github.com/oxur/textrynum/actions/workflows/cicd.yml
[build-badge]: https://github.com/oxur/textrynum/actions/workflows/cicd.yml/badge.svg
[tag-badge]: https://img.shields.io/github/tag/oxur/textrynum.svg
[tag]: https://github.com/oxur/textrynum/tags
[license]: LICENSE-APACHE
[license badge]: https://img.shields.io/badge/License-Apache%202.0%2FMIT-blue.svg
