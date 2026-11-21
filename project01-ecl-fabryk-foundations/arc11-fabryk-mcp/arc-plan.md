# Fabryk Phase 5 — MCP tools

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 168-193. Original bytes are recoverable from import commit 2d006a7. -->

## Phase 5: fabryk-mcp + fabryk-mcp-* (1–2 weeks)

**Goal:** Extract MCP server infrastructure and all generic MCP tools.

**Crates:** `fabryk-mcp`, `fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph`
**Dependency level:** 2–3
**Risk:** Medium

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 5.1 | MCP core infrastructure | Extract `server.rs` to `fabryk-mcp`. Parameterise: `FabrykMcpServer<C: ConfigProvider>`, configurable server name. Define `ToolRegistry` trait. Extract `tools/health.rs` | MCP server starts with mock registry | §4.6 |
| 5.2 | Content & source traits | Define `ContentItemProvider` and `SourceProvider` traits in `fabryk-mcp-content` (Amendment §2a). Extract `tools/guides.rs` → `documents.rs`. Generalise guide tools to document tools | Traits defined, document tools compile | Amend §2a, §4.7 |
| 5.3 | Music theory content providers | Implement `ContentItemProvider` for music theory concepts (from `tools/concepts.rs` logic). Implement `SourceProvider` for music theory sources (from `tools/sources.rs` logic). Wire into tool registration | Concept and source tools work via traits | Amend §2a |
| 5.4 | FTS MCP tools | Extract `tools/search.rs` to `fabryk-mcp-fts`. Parameterise response types to use default schema fields | Search tool works via fabryk-mcp-fts | §4.8 |
| 5.5 | Graph MCP tools | Extract `tools/graph.rs` and `tools/graph_query.rs` to `fabryk-mcp-graph`. Parameterise over generic node/edge types | All 15 graph tools work via fabryk-mcp-graph | §4.9 |
| 5.6 | Full MCP integration | Wire all fabryk-mcp-* crates into ai-music-theory. Implement `ToolRegistry` for music theory. Remove extracted tool files. Verify all 25 tools via MCP inspector | All 25 MCP tools functional, inspector confirms | §7 Phase 6 |

**Phase 5 exit criteria:**

- All four MCP crates compile and pass tests
- ai-music-theory registers tools via `ToolRegistry` trait
- MCP inspector shows all 25 tools with correct schemas
- `cargo test --all-features` passes in both repos

---


## Reconstructed slice locations

- [CC Prompt: Fabryk 5.1 — MCP Core Infrastructure](slice01-mcp-core-infrastructure/slice-plan.md)
- [CC Prompt: Fabryk 5.2 — Content & Source Traits](slice02-content-source-traits/slice-plan.md)
- [CC Prompt: Fabryk 5.3 — Music Theory Content Providers](slice03-music-theory-content-providers/slice-plan.md)
- [CC Prompt: Fabryk 5.4 — FTS MCP Tools](slice04-fts-mcp-tools/slice-plan.md)
- [CC Prompt: Fabryk 5.5 — Graph MCP Tools](slice05-graph-mcp-tools/slice-plan.md)
- [CC Prompt: Fabryk 5.6 — Full MCP Integration](slice06-full-mcp-integration/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
