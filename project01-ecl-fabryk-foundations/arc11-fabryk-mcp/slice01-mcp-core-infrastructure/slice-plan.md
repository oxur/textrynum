# CC Prompt: Fabryk 5.1 — MCP Core Infrastructure

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Create `fabryk-mcp` crate with:

1. `FabrykMcpServer<C: ConfigProvider>` - Generic MCP server
2. `ToolRegistry` trait - Tool registration abstraction
3. Health check tool (extracted from music-theory)
4. Server lifecycle management

