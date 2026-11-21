---
number: 5
title: "ECL Ecosystem Vision Summary"
author: "AI agents"
component: All
tags: [change-me]
created: 2026-01-23
updated: 2026-01-28
state: Superseded
supersedes: null
superseded-by: 9
version: 1.0
---

# ECL Ecosystem Vision Summary

## Expanding from Workflow Engine to Knowledge-Augmented AI Platform

**Purpose**: Preserve context and decisions from initial design discussions

---

## Executive Summary

What began as ECL ("Extract, Cogitate, Load") — a Rust-based workflow orchestration system for AI agents — has expanded into a three-part ecosystem:

1. **ECL** — Workflow orchestration with managed serialism (original scope)
2. **Fabryk** — Persistent knowledge fabric with access control (new)
3. **Fabryk-MCP** — Model Context Protocol server exposing Fabryk to AI agents (new)

Together, these enable a powerful pattern: AI workflows can accumulate knowledge over time, and that knowledge becomes queryable by AI agents in future conversations — all while respecting fine-grained access controls.

---

## The Evolution

### Original ECL Scope

ECL was designed to solve "managed serialism" — orchestrating sequential AI workflow steps with:

- Sequential validation (Step N+1 validates Step N)
- Bounded iteration (feedback loops with hard limits)
- Durability (survive failures across long-running workflows)
- Auditability (trace every decision)

**Persistence in original scope was purely operational:**

- Workflow state (where did we leave off?)
- Step execution records (what happened?)
- Artifacts (files produced by steps)

This answers: *"What did this workflow do?"*

It does NOT answer: *"What has the system learned across all workflows?"*

### The Knowledge Gap

Each ECL workflow starts fresh. Workflow A cannot easily access insights from Workflow B. There's no:

- Semantic indexing of content
- Cross-workflow knowledge queries
- Accumulated institutional memory
- Relationship graphs between entities

### The Expanded Vision

**Fabryk** fills this gap as a separate service that ECL (and other clients) can use:

```
┌─────────────────────────────────────────────────────────────────┐
│                         Fabryk                                  │
│              "Knowledge Fabric with Access Control"             │
├─────────────────────────────────────────────────────────────────┤
│  • Store knowledge items (docs, embeddings, entities)           │
│  • Partition data behind ownership boundaries                   │
│  • Tag-based organization and querying                          │
│  • Fine-grained access control (identities, groups, tags)       │
│  • Query across partitions (with permission)                    │
└─────────────────────────────────────────────────────────────────┘
        ▲                    ▲                    ▲
        │                    │                    │
   ┌────┴────┐         ┌─────┴─────┐        ┌────┴────┐
   │   ECL   │         │    MCP    │        │  Other  │
   │Workflows│         │  Agents   │        │ Clients │
   └─────────┘         └───────────┘        └─────────┘
```

---

## Fabryk: Core Concepts

### What is a Knowledge Item?

Any data submitted to the knowledge store that is useful to the system. Practically:

- **Documents** (markdown being the most common ~90%+ of outputs)
- **Embeddings** (vector representations for semantic search)
- **Entities** (structured data with relationships)
- **Metadata** (tags, timestamps, provenance)

The system should be able to discover/parse items via metadata and offer them as data sources to any other part of the system.

### Partitions

Knowledge items live in partitions — ownership boundaries that provide:

- Isolation by default
- Explicit sharing when desired
- Clear ownership model

### Access Control Model

| Dimension | Description |
|-----------|-------------|
| **Ownership** | Every knowledge item has an owner (identity or group) |
| **Identities** | Individual actors (users, service accounts) |
| **Groups** | Collections of identities |
| **Nested Groups** | Groups can contain groups (hierarchical inheritance) |
| **Tags** | Orthogonal classification (project:alpha, sensitivity:internal) |
| **Tag-based Grants** | "Grant read to Engineering on anything tagged project:alpha" |
| **Item-level Overrides** | Partition may deny-all, but grant specific item access |

### Permission Semantics

- **Tag operations are OR (union)**: If you have access via tag A OR tag B, you're in
- **Group membership is inherited**: Member of Group A which is in Group B → has Group B's permissions
- **Multi-partition queries are explicit**: Must name sources; no implicit cross-partition access

### Design Decisions Captured

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Tag logic | OR (union) | Matches user expectations; less restrictive |
| Group inheritance | Yes | Standard RBAC pattern |
| Cross-partition queries | Explicit only | Security; prevent accidental data leakage |
| Item-level ACLs | Supported | Enables exceptions to partition-level rules |
| Default new items | Inherit partition permissions | Sensible default; override when needed |

---

## Fabryk-MCP: AI Agent Access

### Why MCP?

Model Context Protocol provides a standardized way for AI agents to:

- **Discover** available tools and resources
- **Invoke** actions with structured parameters
- **Receive** data in a format optimized for LLM consumption

Without MCP, every AI integration would need custom code.

### Rust MCP Ecosystem

**Recommended: `rmcp` (Official SDK)**

| Aspect | Details |
|--------|---------|
| Crate | `rmcp` v0.8.x |
| Repo | github.com/modelcontextprotocol/rust-sdk |
| Status | Official, actively maintained |
| Runtime | Tokio async |
| License | MIT |
| Features | Client + Server, multiple transports, OAuth, macros |

### MCP Primitives Mapped to Fabryk

| MCP Primitive | Control | Fabryk Implementation |
|---------------|---------|----------------------|
| **Tools** | Model-controlled (LLM invokes) | `fabryk_store`, `fabryk_query`, `fabryk_get`, `fabryk_list`, `fabryk_relate` |
| **Resources** | App-controlled (app provides) | `fabryk://partition/{id}/items`, `fabryk://item/{id}`, `fabryk://search/{query}` |
| **Prompts** | User-controlled (templates) | "Summarize partition X", "Find related to Y", "Generate knowledge report" |

### The Beautiful Integration Pattern

1. **ECL workflow runs** → produces analysis documents
2. **Documents stored in Fabryk** → via ECL's `store_knowledge()` with partition + tags
3. **Later, different conversation** → AI agent queries Fabryk via MCP
4. **ACL enforced** → Agent only sees what authenticated user can access

This creates **persistent, secure, cross-conversation AI memory**.

---

## Architecture: The Full Picture

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI Agent / LLM Host                          │
│              (Claude Desktop, Cursor, Custom App)               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ MCP Protocol
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    fabryk-mcp-server                            │
│                   (Built with rmcp)                             │
│                                                                 │
│  Tools: store, query, get, list, relate                         │
│  Resources: partitions, items, search results                   │
│  Prompts: summarize, find_related, report                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Fabryk Client API
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Fabryk                                  │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  Storage    │  │   Access    │  │   Query     │              │
│  │  Layer      │  │   Control   │  │   Engine    │              │
│  │             │  │   Layer     │  │             │              │
│  │ • Items     │  │ • Identities│  │ • Keyword   │              │
│  │ • Partitions│  │ • Groups    │  │ • Semantic  │              │
│  │ • Tags      │  │ • Policies  │  │ • Filters   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
        ▲                                        ▲
        │ Fabryk Client                          │ Fabryk Client
        │                                        │
┌───────┴───────┐                        ┌───────┴───────┐
│      ECL      │                        │    Other      │
│   Workflows   │                        │   Clients     │
│               │                        │               │
│ Step outputs  │                        │ Direct API    │
│ marked as     │                        │ access        │
│ "knowledge"   │                        │               │
└───────────────┘                        └───────────────┘
```

---

## ECL ↔ Fabryk Integration Points

### StepContext Extension

```rust
impl StepContext {
    // Existing: workflow-scoped artifacts
    pub async fn store_artifact(&self, ...) -> Result<ArtifactRef>;

    // NEW: persistent knowledge storage
    pub async fn store_knowledge(
        &self,
        item: KnowledgeItem,
        partition: &PartitionId,
        tags: &[Tag],
    ) -> Result<KnowledgeRef>;

    // NEW: query existing knowledge
    pub async fn query_knowledge(
        &self,
        query: FabrykQuery,
        partitions: &[PartitionId],  // explicit!
    ) -> Result<Vec<KnowledgeItem>>;
}
```

### Workflow Configuration

```toml
[workflow.knowledge]
enabled = true
default_partition = "project-alpha"
default_tags = ["source:ecl", "workflow:codebase-analysis"]

[workflow.steps.analysis]
store_output_as_knowledge = true
additional_tags = ["type:analysis-report"]
```

### Graceful Degradation

- ECL works without Fabryk (original behavior)
- If Fabryk unavailable, log warning, continue workflow
- Knowledge storage is enhancement, not requirement

---

## Naming

| Project | Name | Meaning |
|---------|------|---------|
| Workflow Engine | **ECL** | "Extract, Cogitate, Load" |
| Knowledge Fabric | **Fabryk** | "Fabric" + "K" for Knowledge |
| MCP Server | **fabryk-mcp** | Fabryk's MCP interface |

"Fabryk" chosen because:

- Evokes "fabric" (woven knowledge)
- The "k" nods to "knowledge"
- Available on crates.io (unlike "fabric" or "loom")
- Memorable and pronounceable

---

## Crate Structure

### ECL (existing plan)

```
ecl/
├── ecl-core/
├── ecl-steps/
├── ecl-workflows/
└── ecl-cli/
```

### Fabryk (new)

```
fabryk/
├── fabryk-core/       # Types, traits, errors
├── fabryk-acl/        # Access control primitives
├── fabryk-storage/    # Storage backends
├── fabryk-api/        # HTTP API
├── fabryk-client/     # Rust client library
├── fabryk-mcp/        # MCP server (uses rmcp)
└── fabryk-cli/        # Admin CLI
```

---

## Document Roadmap

| Doc # | Name | Status | Description |
|-------|------|--------|-------------|
| 0003 | ECL Project Proposal | ✅ Complete | Original ECL vision |
| 0004 | ECL Project Plan | ✅ Complete | Phased implementation plan |
| 0005 | Fabryk Project Proposal | 📝 Next | Knowledge fabric vision |
| 0006 | Fabryk Project Plan | ⏳ Future | Phased implementation plan |
| 0007 | Fabryk-MCP Proposal | ⏳ Future | MCP integration vision |
| 0008 | Fabryk-MCP Plan | ⏳ Future | MCP implementation plan |

---

## Open Questions for Future Docs

### Fabryk Core

1. **Storage backend**: SQLx only, or also vector DB (pgvector, Qdrant)?
2. **Embedding generation**: Built-in or external service?
3. **Schema evolution**: How do knowledge item schemas change over time?
4. **Retention policies**: Auto-expire old knowledge?

### Fabryk ACL

1. **Existing Rust RBAC libraries**: What can we reuse?
2. **Policy language**: Simple grants or full policy DSL?
3. **Audit logging**: What level of access logging?

### Fabryk-MCP

1. **Authentication flow**: How does MCP auth map to Fabryk identities?
2. **Rate limiting**: Per-identity? Per-partition?
3. **Streaming**: Large query results via MCP streaming?

### ECL Integration

1. **Transaction semantics**: Workflow step + knowledge store atomicity?
2. **Failure handling**: What if Fabryk store fails mid-workflow?

---

## Summary

The ECL ecosystem has grown from a single workflow engine into a three-part platform:

1. **ECL** handles the *doing* — orchestrating AI workflows with durability and auditability
2. **Fabryk** handles the *remembering* — persisting knowledge with access control
3. **Fabryk-MCP** handles the *accessing* — letting AI agents query knowledge naturally

This creates a flywheel: workflows produce knowledge, knowledge informs future workflows and conversations, and access control keeps it all secure.

The separation of concerns is clean:

- Each project has its own repo, crates, and documentation
- Integration is via well-defined client APIs
- Each can evolve independently
- Each can be deployed independently (ECL-only is still valid)

**Next step**: Write 0005-fabryk-proposal.md to formalize the Fabryk vision.

---

## Appendix: Key Decisions Log

| Decision | Choice | Date | Rationale |
|----------|--------|------|-----------|
| Knowledge fabric name | Fabryk | 2026-01 | Available, evocative, "k" for knowledge |
| MCP SDK | rmcp (official) | 2026-01 | Official, maintained, full-featured |
| Tag semantics | OR (union) | 2026-01 | User expectation, less restrictive |
| Cross-partition queries | Explicit only | 2026-01 | Security, prevent data leakage |
| ECL↔Fabryk coupling | Optional/graceful | 2026-01 | ECL works standalone |
| Separate repos | Yes | 2026-01 | Independent evolution, clear boundaries |
