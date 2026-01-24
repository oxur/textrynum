---
number: 8
title: "Fabryk-MCP Project Proposal"
author: "minimum permission"
component: All
tags: [change-me]
created: 2026-01-23
updated: 2026-01-23
state: Under Review
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk-MCP Project Proposal

## Exposing Knowledge Fabric to AI Agents

**Version**: 1.0
**Date**: January 2026
**Status**: Proposal for Review

---

## Executive Summary

Fabryk-MCP bridges the Fabryk knowledge fabric and AI agents via the Model Context Protocol (MCP). It enables AI assistants — whether in Claude Desktop, Cursor, custom applications, or ECL workflows — to discover, query, and contribute to organizational knowledge through a standardized interface.

This proposal outlines:

1. Why MCP is the right integration approach
2. MCP primitives mapped to Fabryk capabilities
3. Authentication and authorization flow
4. Architecture and implementation approach
5. Risks and mitigations
6. Recommended implementation path

---

## The Opportunity

### The Knowledge Access Problem

Fabryk stores valuable knowledge — analysis reports, architecture documents, research findings, accumulated insights. But knowledge has no value if it can't be accessed when needed.

Current access patterns:

| Pattern | Limitation |
|---------|------------|
| **Direct API calls** | Requires custom integration code per application |
| **Web UI** | Human-only; AI agents can't use it |
| **Copy/paste into prompts** | Manual, error-prone, doesn't scale |

### Why MCP?

Model Context Protocol solves this by providing a **standardized interface** that any MCP-compatible AI host can use:

```
┌─────────────────────────────────────────────────────────────────┐
│              MCP-Compatible AI Hosts                            │
├─────────────────────────────────────────────────────────────────┤
│  Claude Desktop  │  Cursor  │  VS Code  │  Custom Apps  │  ECL  │
└────────┬─────────┴────┬─────┴─────┬─────┴───────┬───────┴───┬───┘
         │              │           │             │           │
         └──────────────┴───────────┴─────────────┴───────────┘
                                    │
                                    │ MCP Protocol (JSON-RPC)
                                    │ (One integration, many clients)
                                    ▼
                        ┌───────────────────────┐
                        │    fabryk-mcp         │
                        │    (MCP Server)       │
                        └───────────────────────┘
                                    │
                                    │ Fabryk Client API
                                    ▼
                        ┌───────────────────────┐
                        │       Fabryk          │
                        │  (Knowledge Fabric)   │
                        └───────────────────────┘
```

**Benefits of MCP approach:**

| Benefit | Description |
|---------|-------------|
| **Write once, use everywhere** | Single server works with all MCP hosts |
| **Standardized discovery** | Hosts automatically learn available tools |
| **User-controlled** | Humans approve sensitive operations |
| **Auth built-in** | MCP spec includes OAuth support |
| **Ecosystem momentum** | Growing adoption across AI tools |

---

## MCP Primer

For context, MCP defines three main primitives that servers expose:

### Tools (Model-Controlled)

Actions the AI model can invoke. The model decides when to use them based on conversation context.

```
User: "What do we know about microservices patterns?"
AI: [decides to call fabryk_query tool]
    → Returns relevant knowledge items
AI: "Based on our knowledge base, here are the key patterns..."
```

### Resources (Application-Controlled)

Data the host application provides to the model. The application decides what context to include.

```
Host app loads: fabryk://partition/project-alpha/recent
    → Injects recent items as context
AI now has awareness of recent knowledge without explicit query
```

### Prompts (User-Controlled)

Pre-defined templates users can invoke. Standardize common workflows.

```
User: /summarize-partition project-alpha
    → Expands to structured prompt with partition contents
AI: Generates comprehensive summary
```

---

## Fabryk-MCP Design

### Tools

Fabryk-MCP exposes these tools for AI model invocation:

#### `fabryk_query`

Search the knowledge base.

```json
{
  "name": "fabryk_query",
  "description": "Search the Fabryk knowledge base using keyword or semantic search. Returns relevant knowledge items the user has access to.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query (natural language for semantic search, or keywords)"
      },
      "partitions": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Partition IDs to search. If omitted, searches all accessible partitions."
      },
      "tags": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Filter by tags (OR logic). Format: 'namespace:value' or just 'value'."
      },
      "search_type": {
        "type": "string",
        "enum": ["semantic", "keyword", "hybrid"],
        "default": "hybrid",
        "description": "Type of search to perform."
      },
      "limit": {
        "type": "integer",
        "default": 10,
        "maximum": 50,
        "description": "Maximum number of results."
      }
    },
    "required": ["query"]
  }
}
```

**Example invocation:**

```json
{
  "query": "error handling patterns in Rust",
  "partitions": ["project/alpha"],
  "tags": ["type:architecture"],
  "search_type": "hybrid",
  "limit": 5
}
```

#### `fabryk_get`

Retrieve a specific knowledge item by ID.

```json
{
  "name": "fabryk_get",
  "description": "Retrieve a specific knowledge item by ID. Returns full content if user has access.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "item_id": {
        "type": "string",
        "description": "The knowledge item ID (UUID)."
      },
      "include_content": {
        "type": "boolean",
        "default": true,
        "description": "Whether to include full content or just metadata."
      }
    },
    "required": ["item_id"]
  }
}
```

#### `fabryk_list`

List items in a partition or by tag.

```json
{
  "name": "fabryk_list",
  "description": "List knowledge items in a partition or matching tags. Returns metadata without full content.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "partition": {
        "type": "string",
        "description": "Partition ID to list."
      },
      "tags": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Filter by tags."
      },
      "sort_by": {
        "type": "string",
        "enum": ["created_at", "updated_at", "title"],
        "default": "updated_at"
      },
      "sort_order": {
        "type": "string",
        "enum": ["asc", "desc"],
        "default": "desc"
      },
      "limit": {
        "type": "integer",
        "default": 20,
        "maximum": 100
      },
      "cursor": {
        "type": "string",
        "description": "Pagination cursor from previous response."
      }
    }
  }
}
```

#### `fabryk_store`

Store new knowledge (requires write permission).

```json
{
  "name": "fabryk_store",
  "description": "Store a new knowledge item. Requires write permission to the target partition.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "partition": {
        "type": "string",
        "description": "Target partition ID."
      },
      "title": {
        "type": "string",
        "description": "Title for the knowledge item."
      },
      "content": {
        "type": "string",
        "description": "Content to store (typically markdown)."
      },
      "content_type": {
        "type": "string",
        "default": "text/markdown",
        "description": "MIME type of content."
      },
      "tags": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Tags to apply."
      },
      "summary": {
        "type": "string",
        "description": "Optional summary for search/display."
      }
    },
    "required": ["partition", "title", "content"]
  }
}
```

#### `fabryk_relate`

Find items related to a given item or topic.

```json
{
  "name": "fabryk_relate",
  "description": "Find knowledge items related to a given item or topic using semantic similarity.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "item_id": {
        "type": "string",
        "description": "Find items related to this item."
      },
      "topic": {
        "type": "string",
        "description": "Find items related to this topic (alternative to item_id)."
      },
      "partitions": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Partitions to search."
      },
      "limit": {
        "type": "integer",
        "default": 10
      }
    }
  }
}
```

#### `fabryk_partitions`

List accessible partitions.

```json
{
  "name": "fabryk_partitions",
  "description": "List partitions the current user has access to.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "permission": {
        "type": "string",
        "enum": ["read", "write", "admin"],
        "default": "read",
        "description": "Filter by minimum permission level."
      }
    }
  }
}
```

### Resources

Resources provide context that host applications can inject:

#### Partition Contents

```
URI: fabryk://partition/{partition_id}
Description: All accessible items in a partition (metadata only)

URI: fabryk://partition/{partition_id}/recent
Description: Recently updated items in partition (last 30 days)

URI: fabryk://partition/{partition_id}/items
Description: Full listing with pagination support
```

#### Individual Items

```
URI: fabryk://item/{item_id}
Description: Full content of a specific knowledge item

URI: fabryk://item/{item_id}/metadata
Description: Metadata only (tags, timestamps, provenance)
```

#### Search Results

```
URI: fabryk://search/{encoded_query}
Description: Pre-executed search results as context

URI: fabryk://search/{encoded_query}?partitions={p1,p2}&tags={t1,t2}
Description: Filtered search results
```

#### Tag-Based Collections

```
URI: fabryk://tags/{tag}
Description: All items with a specific tag

URI: fabryk://tags/{namespace}:{value}
Description: Items with namespaced tag
```

### Prompts

Pre-defined prompt templates for common workflows:

#### `summarize_partition`

```json
{
  "name": "summarize_partition",
  "description": "Generate a comprehensive summary of all knowledge in a partition.",
  "arguments": [
    {
      "name": "partition",
      "description": "Partition ID to summarize",
      "required": true
    },
    {
      "name": "focus": {
        "description": "Optional focus area for the summary",
        "required": false
      }
    }
  ]
}
```

**Expands to:**

```
Given the following knowledge items from partition "{partition}":

{items_content}

Please provide a comprehensive summary of this knowledge base.
{if focus}Focus particularly on: {focus}{/if}

Include:
1. Key themes and topics covered
2. Important findings or conclusions
3. Gaps or areas that may need more documentation
4. Connections between different items
```

#### `find_related`

```json
{
  "name": "find_related",
  "description": "Find and explain relationships between knowledge items.",
  "arguments": [
    {
      "name": "topic",
      "description": "Topic to find related knowledge for",
      "required": true
    },
    {
      "name": "depth",
      "description": "How deep to explore relationships (shallow/medium/deep)",
      "required": false
    }
  ]
}
```

#### `knowledge_report`

```json
{
  "name": "knowledge_report",
  "description": "Generate a structured report from knowledge across partitions.",
  "arguments": [
    {
      "name": "partitions",
      "description": "Comma-separated partition IDs to include",
      "required": true
    },
    {
      "name": "report_type",
      "description": "Type of report: overview, technical, executive",
      "required": false
    },
    {
      "name": "time_range",
      "description": "Time range: all, last_week, last_month, last_quarter",
      "required": false
    }
  ]
}
```

#### `compare_items`

```json
{
  "name": "compare_items",
  "description": "Compare and contrast multiple knowledge items.",
  "arguments": [
    {
      "name": "item_ids",
      "description": "Comma-separated item IDs to compare",
      "required": true
    },
    {
      "name": "aspects",
      "description": "Specific aspects to compare",
      "required": false
    }
  ]
}
```

---

## Authentication & Authorization

### Authentication Flow

MCP supports OAuth 2.0. Fabryk-MCP uses this for user authentication:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   MCP Host  │     │ fabryk-mcp  │     │   Fabryk    │     │  Identity   │
│  (Claude)   │     │   Server    │     │   API       │     │  Provider   │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │                   │
       │ 1. Connect        │                   │                   │
       │──────────────────>│                   │                   │
       │                   │                   │                   │
       │ 2. Auth Required  │                   │                   │
       │<──────────────────│                   │                   │
       │                   │                   │                   │
       │ 3. OAuth Flow     │                   │                   │
       │───────────────────────────────────────────────────────────>
       │                   │                   │                   │
       │ 4. Token          │                   │                   │
       │<───────────────────────────────────────────────────────────
       │                   │                   │                   │
       │ 5. Connect + Token│                   │                   │
       │──────────────────>│                   │                   │
       │                   │                   │                   │
       │                   │ 6. Validate Token │                   │
       │                   │──────────────────>│                   │
       │                   │                   │                   │
       │                   │ 7. Identity       │                   │
       │                   │<──────────────────│                   │
       │                   │                   │                   │
       │ 8. Connected      │                   │                   │
       │<──────────────────│                   │                   │
       │                   │                   │                   │
```

### Identity Mapping

MCP authentication tokens map to Fabryk identities:

```rust
pub struct McpSession {
    pub mcp_token: String,
    pub fabryk_identity: IdentityId,
    pub permissions_cache: PermissionsCache,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

### Authorization Enforcement

Every MCP operation checks Fabryk ACL:

```rust
impl FabrykMcpServer {
    async fn handle_query(&self, params: QueryParams, session: &McpSession) -> Result<QueryResponse> {
        // 1. Resolve partitions (default to all accessible if not specified)
        let partitions = match params.partitions {
            Some(p) => p,
            None => self.fabryk.list_accessible_partitions(&session.fabryk_identity).await?,
        };

        // 2. Filter to only authorized partitions
        let authorized = self.fabryk
            .filter_authorized(&session.fabryk_identity, &partitions, Permission::Read)
            .await?;

        if authorized.is_empty() {
            return Ok(QueryResponse::empty("No accessible partitions"));
        }

        // 3. Execute query with authorization context
        let results = self.fabryk
            .query_with_acl(&session.fabryk_identity, &authorized, params.query)
            .await?;

        Ok(QueryResponse::from(results))
    }
}
```

### Permission Requirements by Tool

| Tool | Required Permission |
|------|---------------------|
| `fabryk_query` | Read on queried partitions |
| `fabryk_get` | Read on item's partition |
| `fabryk_list` | Read on listed partition |
| `fabryk_store` | Write on target partition |
| `fabryk_relate` | Read on searched partitions |
| `fabryk_partitions` | (Lists based on permissions) |

---

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      fabryk-mcp                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  MCP Protocol Layer                       │  │
│  │                     (rmcp)                                │  │
│  │                                                           │  │
│  │  • JSON-RPC handling                                      │  │
│  │  • Tool/Resource/Prompt registration                      │  │
│  │  • Transport management (stdio, HTTP)                     │  │
│  │  • OAuth integration                                      │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  Handler Layer                            │  │
│  │                                                           │  │
│  │  • Tool handlers (query, get, list, store, relate)        │  │
│  │  • Resource resolvers                                     │  │
│  │  • Prompt expanders                                       │  │
│  │  • Response formatting                                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  Session Layer                            │  │
│  │                                                           │  │
│  │  • Authentication state                                   │  │
│  │  • Identity mapping                                       │  │
│  │  • Permission caching                                     │  │
│  │  • Rate limiting                                          │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  Fabryk Client                            │  │
│  │                 (fabryk-client)                           │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │     Fabryk       │
                    │   (HTTP API)     │
                    └──────────────────┘
```

### Crate Structure

```
fabryk-mcp/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── main.rs              # Standalone server entry
    │
    ├── server.rs            # MCP server setup (rmcp)
    │
    ├── handlers/
    │   ├── mod.rs
    │   ├── tools.rs         # Tool implementations
    │   ├── resources.rs     # Resource resolvers
    │   └── prompts.rs       # Prompt expanders
    │
    ├── session/
    │   ├── mod.rs
    │   ├── auth.rs          # OAuth handling
    │   ├── identity.rs      # Fabryk identity mapping
    │   └── cache.rs         # Permission caching
    │
    ├── formatting/
    │   ├── mod.rs
    │   ├── items.rs         # Format knowledge items for LLM
    │   ├── results.rs       # Format search results
    │   └── errors.rs        # Format error responses
    │
    └── config.rs            # Configuration
```

### Dependencies

```toml
[dependencies]
# MCP Protocol
rmcp = { version = "0.8", features = ["server", "transport-stdio", "transport-http"] }

# Fabryk Client
fabryk-client = { path = "../fabryk-client" }

# Async Runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Configuration
figment = { version = "0.10", features = ["toml", "env"] }

# Observability
tracing = "0.1"
tracing-subscriber = "0.3"

# Utilities
thiserror = "2"
anyhow = "1"
```

---

## Transport Options

### Stdio Transport (Local)

For local MCP hosts (Claude Desktop, Cursor):

```bash
# In MCP host configuration
{
  "mcpServers": {
    "fabryk": {
      "command": "fabryk-mcp",
      "args": ["--transport", "stdio"],
      "env": {
        "FABRYK_ENDPOINT": "http://localhost:8100"
      }
    }
  }
}
```

### HTTP Transport (Remote)

For remote/shared deployments:

```bash
fabryk-mcp serve --transport http --port 8200
```

Hosts connect via:

```
{
  "mcpServers": {
    "fabryk": {
      "url": "https://fabryk-mcp.example.com",
      "transport": "http"
    }
  }
}
```

---

## Response Formatting

### Optimizing for LLM Consumption

Knowledge items are formatted for optimal LLM understanding:

```rust
impl FormatForLlm for KnowledgeItem {
    fn format(&self, verbosity: Verbosity) -> String {
        match verbosity {
            Verbosity::Brief => format!(
                "**{}** ({})\n{}\n[Tags: {}]",
                self.title,
                self.id,
                self.summary.as_deref().unwrap_or("No summary"),
                self.tags.join(", ")
            ),

            Verbosity::Standard => format!(
                "# {}\n\n\
                 **ID**: {}\n\
                 **Partition**: {}\n\
                 **Created**: {}\n\
                 **Tags**: {}\n\n\
                 ## Summary\n{}\n\n\
                 ## Content\n{}",
                self.title,
                self.id,
                self.partition_id,
                self.created_at.format("%Y-%m-%d"),
                self.tags.join(", "),
                self.summary.as_deref().unwrap_or("No summary"),
                self.content.truncate_for_context(4000)
            ),

            Verbosity::Full => // Include everything
        }
    }
}
```

### Search Results Formatting

```rust
impl FormatForLlm for QueryResponse {
    fn format(&self) -> String {
        let mut output = format!(
            "Found {} results for query.\n\n",
            self.items.len()
        );

        for (i, item) in self.items.iter().enumerate() {
            output.push_str(&format!(
                "## Result {} (Relevance: {:.0}%)\n{}\n\n",
                i + 1,
                item.score * 100.0,
                item.format(Verbosity::Brief)
            ));
        }

        if self.has_more {
            output.push_str(&format!(
                "\n*{} more results available. Use pagination to see more.*",
                self.total_count - self.items.len()
            ));
        }

        output
    }
}
```

---

## Configuration

```toml
# fabryk-mcp.toml

[server]
transport = "stdio"  # or "http"
http_port = 8200     # if transport = "http"

[fabryk]
endpoint = "http://localhost:8100"
timeout_secs = 30

[auth]
# OAuth configuration
provider = "fabryk"  # Use Fabryk's built-in auth
client_id = "fabryk-mcp"
# Or external provider:
# provider = "oidc"
# issuer = "https://auth.example.com"

[limits]
max_results_per_query = 50
max_content_length = 100000  # Characters
rate_limit_per_minute = 60

[caching]
enabled = true
permission_ttl_secs = 300   # 5 minutes
identity_ttl_secs = 3600    # 1 hour

[logging]
level = "info"
format = "json"  # or "pretty"
```

---

## Risk Assessment

### Risk 1: Authentication Complexity

**Risk**: OAuth flows can be complex; poor implementation could leak access.

**Mitigation**:

- Use rmcp's built-in OAuth support
- Leverage Fabryk's existing auth infrastructure
- Comprehensive auth flow testing
- Token validation on every request

**Assessment**: Medium risk. Critical to get right.

### Risk 2: Context Window Limits

**Risk**: Knowledge items may be too large for LLM context windows.

**Mitigation**:

- Implement intelligent truncation
- Provide summary + "get full content" pattern
- Support pagination
- Allow verbosity configuration

**Assessment**: Low risk. Manageable with good formatting.

### Risk 3: Permission Leakage via Prompts

**Risk**: Pre-defined prompts might expose data user shouldn't see.

**Mitigation**:

- Prompts execute with user's permissions
- No special prompt privileges
- Audit prompt expansions
- Test with multi-tenant scenarios

**Assessment**: Low risk with proper implementation.

### Risk 4: Rate Limiting / Abuse

**Risk**: AI agents could hammer Fabryk with queries.

**Mitigation**:

- Per-session rate limiting
- Query complexity limits
- Caching for repeated queries
- Monitoring and alerting

**Assessment**: Low-Medium risk. Standard operational concern.

### Risk 5: rmcp SDK Maturity

**Risk**: Official Rust MCP SDK is relatively new.

**Mitigation**:

- SDK is actively maintained by MCP team
- 2.6k stars, 103 contributors
- Abstract where possible for future flexibility
- Monitor SDK updates

**Assessment**: Low risk. SDK is production-viable.

---

## Recommended Path Forward

### Phase 1: Core Tools (Weeks 1-2)

**Objective**: Basic tool functionality with auth.

1. Set up fabryk-mcp crate with rmcp
2. Implement `fabryk_query` tool
3. Implement `fabryk_get` tool
4. Implement `fabryk_list` tool
5. Implement `fabryk_partitions` tool
6. Add API key authentication (simple, for testing)
7. Test with Claude Desktop

**Deliverable**: Query and retrieve knowledge via MCP.

### Phase 2: Full Tools + Auth (Weeks 3-4)

**Objective**: Complete tool set with proper OAuth.

1. Implement `fabryk_store` tool
2. Implement `fabryk_relate` tool
3. Implement OAuth authentication flow
4. Add permission caching
5. Add rate limiting
6. Comprehensive tool tests

**Deliverable**: All tools working with secure auth.

### Phase 3: Resources (Weeks 5-6)

**Objective**: Resource support for context injection.

1. Implement partition resources
2. Implement item resources
3. Implement search result resources
4. Implement tag-based resources
5. Add resource subscription support
6. Test with various MCP hosts

**Deliverable**: Full resource support.

### Phase 4: Prompts + Polish (Weeks 7-8)

**Objective**: Prompts and production readiness.

1. Implement `summarize_partition` prompt
2. Implement `find_related` prompt
3. Implement `knowledge_report` prompt
4. Implement `compare_items` prompt
5. Optimize response formatting
6. Add HTTP transport option
7. Performance optimization
8. Documentation

**Deliverable**: Production-ready v1.

---

## Success Criteria for v1

1. **Discovery**: MCP hosts can discover all Fabryk tools
2. **Query**: Semantic and keyword search work correctly
3. **Store**: Can create new knowledge items (with permission)
4. **Auth**: OAuth flow works; permissions enforced
5. **Resources**: Hosts can inject knowledge as context
6. **Prompts**: Pre-defined prompts expand and execute correctly
7. **Performance**: <500ms typical response time
8. **Compatibility**: Works with Claude Desktop, Cursor, and custom hosts

---

## Open Questions Requiring Decision

### Q1: Streaming for Large Results

**Option A**: Return all results at once

- Pro: Simpler implementation
- Con: May timeout for large result sets

**Option B**: MCP streaming support

- Pro: Better UX for large results
- Con: More complex; depends on rmcp streaming support

**Recommendation**: Start with Option A; add streaming if needed.

### Q2: Embedding Generation via MCP

**Option A**: Fabryk generates embeddings on store

- Pro: Consistent; user doesn't need embedding access
- Con: Requires Fabryk to have embedding provider

**Option B**: MCP client provides embeddings

- Pro: Flexible; client controls embedding model
- Con: Inconsistent embeddings; more complex protocol

**Recommendation**: Option A — Fabryk handles embeddings internally.

### Q3: Real-time Notifications

**Option A**: No notifications; client polls

- Pro: Simple
- Con: No real-time updates

**Option B**: MCP notifications for changes

- Pro: Real-time awareness of new knowledge
- Con: Connection management; complexity

**Recommendation**: Start without notifications; evaluate need based on usage.

### Q4: Multi-Server Discovery

**Option A**: Single fabryk-mcp per Fabryk instance

- Pro: Simple 1:1 mapping
- Con: User manages multiple servers for multiple Fabryks

**Option B**: fabryk-mcp can proxy multiple Fabryk instances

- Pro: Single MCP endpoint
- Con: Complexity; auth across instances

**Recommendation**: Option A for v1; simpler mental model.

---

## Conclusion

Fabryk-MCP completes the knowledge accessibility story. By exposing Fabryk through MCP, any AI agent in any compatible host can discover, query, and contribute to organizational knowledge — all while respecting access controls.

The implementation builds on:

- **rmcp**: Official, production-viable Rust MCP SDK
- **fabryk-client**: Clean integration with Fabryk core
- **MCP ecosystem**: Growing adoption across AI tools

Combined with ECL for workflow orchestration and Fabryk for knowledge storage, Fabryk-MCP enables a powerful pattern: AI workflows produce knowledge, that knowledge persists securely, and AI agents anywhere can access it.

We recommend proceeding with the proposed architecture and phased implementation plan.

---

## References

### Related Documents

- 0003-ecl-project-proposal.md — ECL workflow engine
- 0004-ecl-project-plan.md — ECL implementation plan
- 0005-fabryk-proposal.md — Fabryk knowledge fabric
- ecl-ecosystem-vision-summary.md — Full ecosystem vision

### MCP Specification

- <https://modelcontextprotocol.io> — Official MCP documentation
- <https://spec.modelcontextprotocol.io> — Protocol specification

### rmcp (Rust MCP SDK)

- <https://github.com/modelcontextprotocol/rust-sdk> — Official repository
- <https://docs.rs/rmcp> — API documentation
- <https://crates.io/crates/rmcp> — Crate registry

### MCP Hosts

- Claude Desktop — Anthropic's desktop app
- Cursor — AI-powered IDE
- VS Code + Continue — Open source AI coding assistant

---

## Appendix A: Tool Response Examples

### fabryk_query Response

```json
{
  "content": [
    {
      "type": "text",
      "text": "Found 3 results for query 'error handling patterns'.\n\n## Result 1 (Relevance: 94%)\n**Rust Error Handling Best Practices** (item-abc123)\nComprehensive guide to error handling in Rust applications.\n[Tags: type:guide, domain:rust, topic:errors]\n\n## Result 2 (Relevance: 87%)\n**API Error Response Standards** (item-def456)\nStandardized error response format for REST APIs.\n[Tags: type:standard, domain:api]\n\n## Result 3 (Relevance: 82%)\n**Exception Handling Patterns Review** (item-ghi789)\nCode review findings on exception handling.\n[Tags: type:code-review, project:alpha]"
    }
  ]
}
```

### fabryk_get Response

```json
{
  "content": [
    {
      "type": "text",
      "text": "# Rust Error Handling Best Practices\n\n**ID**: item-abc123\n**Partition**: team/engineering\n**Created**: 2026-01-15\n**Tags**: type:guide, domain:rust, topic:errors\n\n## Summary\nComprehensive guide to error handling in Rust applications using thiserror, anyhow, and custom error types.\n\n## Content\n\n### Introduction\n\nError handling in Rust is explicit and type-safe...\n\n[Content continues...]"
    }
  ]
}
```

---

## Appendix B: MCP Host Configuration Examples

### Claude Desktop

```json
{
  "mcpServers": {
    "fabryk": {
      "command": "/usr/local/bin/fabryk-mcp",
      "args": [],
      "env": {
        "FABRYK_ENDPOINT": "http://localhost:8100",
        "FABRYK_LOG_LEVEL": "info"
      }
    }
  }
}
```

### Cursor

```json
{
  "mcp": {
    "servers": {
      "fabryk": {
        "command": "fabryk-mcp",
        "transport": "stdio"
      }
    }
  }
}
```

### Custom Application (HTTP)

```rust
use rmcp::{Client, transport::HttpTransport};

let transport = HttpTransport::new("https://fabryk-mcp.example.com")?;
let client = Client::new(transport).await?;

// List available tools
let tools = client.list_tools().await?;

// Call a tool
let result = client.call_tool("fabryk_query", json!({
    "query": "architecture patterns",
    "limit": 5
})).await?;
```
