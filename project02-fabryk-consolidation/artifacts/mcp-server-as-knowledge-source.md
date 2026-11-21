
<!-- Historical source: workbench/mcp-server-as-knowledge-source.md, lines 1-134. Original bytes are recoverable from import commit 3a39aea. -->

# Making an MCP Server the "Official" Knowledge Source for Claude Code

## The Question

> I would like to make an MCP server (that is equipped not only with tools,
> but also with an extensive ~3000-4000-node knowledge graph, FTS capability,
> and in-memory vector DB, all with supporting MCP tools) the "official"
> source of knowledge for Claude Code ... so instead of a SKILL.md, it would
> use this MCP server. Is that possible? If so, what's the best way to do
> that?

## Short Answer

**Yes, this is absolutely possible.** There are complementary mechanisms in
Claude Code to wire it up, and the architecture replaces static
markdown-based knowledge (like SKILL.md files) with a dynamic, queryable
knowledge system exposed over MCP.

---

## The Approach

### 1. The MCP Server Itself

The server exposes tools like `search_knowledge_graph`, `semantic_search`,
`full_text_search`, `get_concept`, `query_relationships`, and so on. This
is the same pattern proven by existing MCP server implementations that
combine knowledge graphs, FTS, and semantic search tools. The architecture
is sound — the question is how to make Claude Code *prefer* it.

### 2. Telling Claude Code to Prefer This Server as Its Knowledge Source

This is the key challenge. There are several mechanisms that work together.

#### CLAUDE.md — The Behavioral Contract

The most powerful lever is the project's `CLAUDE.md` file. This is Claude
Code's equivalent of a system prompt for a project. You put instructions
like:

```markdown
# CLAUDE.md

## Knowledge & Documentation

Before writing code, generating configurations, or answering domain
questions in this project, ALWAYS query the knowledge MCP server first:

- Use `search_knowledge_graph` for understanding entity relationships
  and architecture decisions
- Use `semantic_search` for conceptual/fuzzy lookups
- Use `full_text_search` for exact terms, error messages, or specific APIs
- Use `get_concept` when you already know the concept slug

Do NOT rely on training data for project-specific patterns, conventions,
or architectural decisions. The knowledge server is authoritative.
```

This is the most direct equivalent of what SKILL.md does — it's
instructions that Claude Code reads at the start of every session. The
difference is that instead of containing the knowledge inline, it tells
Claude *how to fetch it* from your MCP tools.

### 3. Making the Knowledge Graph Self-Describing

A really effective pattern is to give your MCP server a `get_instructions`
or `list_capabilities` tool that returns a manifest of what knowledge
domains it covers and when to query it. Claude Code will see this in the
tool list and understand the server's purpose.

You could even have a top-level `get_guidance` tool that returns
context-appropriate instructions — similar to how a SKILL.md would be
read, but dynamic.

### 4. Scope Configuration for the Team

Use `--scope project` so the MCP server config lives in `.mcp.json` at the
project root. Combine that with the `CLAUDE.md` instructions, and anyone
who clones the repo gets both the connection config *and* the behavioral
instructions automatically.

---

## Trade-offs vs. Static SKILL.md

| Dimension              | SKILL.md (static)              | MCP Knowledge Server (dynamic)             |
|------------------------|--------------------------------|--------------------------------------------|
| **Latency**            | Loaded into context instantly  | Each lookup is a network round-trip         |
| **Context window**     | Full file loaded upfront       | Selective — only pulls in what's relevant   |
| **Freshness**          | Stale until manually updated   | Always reflects current server state        |
| **Depth**              | Limited by file size           | 3000-4000 nodes, full graph traversal       |
| **Reliability**        | Always available                | Depends on server uptime / cold starts      |
| **Team distribution**  | Committed to repo              | Repo has connection config; server is shared |

### Latency

Every knowledge lookup is a tool call (network round-trip), whereas SKILL.md
is loaded into context immediately. For frequently-needed info, this can
slow things down. Consider having your `get_guidance` tool return a concise
"cheat sheet" that gets loaded early, with deeper queries for specific
topics.

### Context Window Usage

MCP tool results consume context tokens just like SKILL.md content. But you
get the advantage of *selective* loading — only pulling in what's relevant
rather than dumping everything into context upfront.

### Reliability

If Cloud Run cold-starts or the server is down, Claude Code falls back to
its training data. SKILL.md is always available. You may want a lightweight
local fallback.

---

## The Ideal Hybrid Pattern

```
CLAUDE.md          → Behavioral instructions + "always consult the MCP server"
.mcp.json          → Server connection config (committed to repo)
MCP server tools   → Dynamic knowledge retrieval (graph, FTS, vector)
  └─ get_guidance  → Returns a concise "bootstrap" context for the current task
```

This gives you the discoverability and team-sharing of the SKILL.md pattern,
with the depth and dynamism of a full knowledge graph. The `CLAUDE.md` tells
Claude *what to do*, the `.mcp.json` tells it *where to connect*, and the
MCP server provides the actual knowledge on demand.

The architecture works because Claude Code already understands how to
discover and use MCP tools — you're just steering it to *prefer* your
knowledge server over its training data through clear, imperative
instructions in `CLAUDE.md`.
