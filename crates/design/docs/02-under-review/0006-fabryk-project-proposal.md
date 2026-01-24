---
number: 6
title: "Fabryk Project Proposal"
author: "UUID NOT"
component: All
tags: [change-me]
created: 2026-01-23
updated: 2026-01-23
state: Under Review
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk Project Proposal

## Knowledge Fabric with Access Control

**Version**: 1.0
**Date**: January 2026
**Status**: Proposal for Review

---

## Executive Summary

Fabryk is a persistent knowledge storage system designed to complement AI workflow engines like ECL. While ECL orchestrates *what AI does*, Fabryk manages *what AI knows* — providing durable, queryable, access-controlled storage for knowledge accumulated across workflows and conversations.

The name "Fabryk" evokes "fabric" (interwoven knowledge) with a "k" nodding to "knowledge."

This proposal outlines:

1. The problem Fabryk solves
2. Core concepts and architecture
3. Access control model
4. Integration points with ECL and MCP
5. Risks and mitigations
6. Recommended implementation path

---

## The Problem

### AI Systems Have Amnesia

Current AI workflow systems — including ECL as originally designed — treat each workflow as isolated. Workflow A cannot access insights from Workflow B. Each conversation starts fresh. There is no institutional memory.

This creates real problems:

| Problem | Impact |
|---------|--------|
| **Repeated analysis** | Same codebase analyzed multiple times with no memory of prior findings |
| **Lost context** | Insights from last month's architecture review unavailable today |
| **No knowledge accumulation** | System never gets smarter from its own work |
| **Manual knowledge transfer** | Users must copy/paste relevant context into each new conversation |

### Existing Solutions Fall Short

| Approach | Limitation |
|----------|------------|
| **Chat history search** | Unstructured; no semantic understanding; single-user |
| **Document stores (S3, etc.)** | No query intelligence; no access control granularity |
| **Vector databases alone** | No ownership model; no structured metadata; no ACL |
| **RAG systems** | Often single-tenant; limited access control; retrieval-focused only |

### What's Needed

A knowledge layer that provides:

1. **Persistent storage** — Knowledge survives beyond individual workflows/conversations
2. **Intelligent retrieval** — Semantic search, not just keyword matching
3. **Access control** — Fine-grained permissions (who can see what)
4. **Multi-tenancy** — Multiple users/teams with isolated and shared partitions
5. **Provenance** — Know where knowledge came from and when
6. **Integration-ready** — Easy to connect from workflows (ECL) and AI agents (MCP)

---

## Core Concepts

### Knowledge Items

A **knowledge item** is the atomic unit of storage in Fabryk. It represents any piece of information worth persisting:

```rust
pub struct KnowledgeItem {
    pub id: KnowledgeId,
    pub partition_id: PartitionId,
    pub owner: IdentityId,

    // Content
    pub content: KnowledgeContent,
    pub content_type: ContentType,  // text/markdown, application/json, etc.

    // Metadata
    pub title: String,
    pub summary: Option<String>,
    pub tags: Vec<Tag>,
    pub provenance: Provenance,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,

    // Search support
    pub embedding: Option<Embedding>,
}

pub enum KnowledgeContent {
    Text(String),
    Binary(Vec<u8>),
    Reference(Uri),  // External content
}

pub struct Provenance {
    pub source: ProvenanceSource,
    pub workflow_id: Option<WorkflowId>,
    pub step_id: Option<StepId>,
    pub conversation_id: Option<String>,
    pub created_by: IdentityId,
}

pub enum ProvenanceSource {
    EclWorkflow,
    DirectApi,
    McpAgent,
    Import,
}
```

**Typical knowledge items:**

- Analysis reports (markdown)
- Architecture documents
- Code review findings
- Meeting summaries
- Research notes
- Entity extractions (structured JSON)
- Embeddings of any of the above

### Partitions

A **partition** is an ownership and isolation boundary:

```rust
pub struct Partition {
    pub id: PartitionId,
    pub name: String,
    pub description: Option<String>,
    pub owner: IdentityOrGroup,
    pub default_policy: AccessPolicy,
    pub created_at: DateTime<Utc>,
}
```

Partitions provide:

- **Isolation by default** — Items in one partition aren't visible from another
- **Clear ownership** — Every partition has an owner (identity or group)
- **Policy inheritance** — Items inherit partition's default policy unless overridden
- **Query boundary** — Cross-partition queries must be explicit

**Example partitions:**

- `personal/alice` — Alice's private knowledge
- `team/engineering` — Shared engineering team knowledge
- `project/alpha` — Project-specific knowledge
- `org/acme` — Organization-wide knowledge

### Tags

**Tags** provide orthogonal classification independent of partition structure:

```rust
pub struct Tag {
    pub namespace: Option<String>,  // e.g., "project", "type", "sensitivity"
    pub value: String,              // e.g., "alpha", "analysis", "internal"
}

// Examples:
// project:alpha
// type:architecture-doc
// sensitivity:internal
// source:ecl-workflow
// domain:payments
```

Tags enable:

- **Cross-cutting queries** — "All architecture docs across all my partitions"
- **Policy grants** — "Engineering can read anything tagged project:alpha"
- **Organization** — Multiple classification dimensions

**Tag semantics are OR (union):** Access via tag A OR tag B grants access.

---

## Access Control Model

### Principals

A **principal** is an entity that can be granted or denied access:

```rust
pub enum Principal {
    Identity(IdentityId),
    Group(GroupId),
    Everyone,  // Public access
}

pub struct Identity {
    pub id: IdentityId,
    pub name: String,
    pub identity_type: IdentityType,
    pub created_at: DateTime<Utc>,
}

pub enum IdentityType {
    User,
    ServiceAccount,
    McpAgent,
}

pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub members: Vec<Principal>,  // Can include other groups!
    pub owner: IdentityId,
    pub created_at: DateTime<Utc>,
}
```

**Group membership is inherited:** If Alice is in Group A, and Group A is in Group B, Alice has Group B's permissions.

### Permissions

```rust
pub enum Permission {
    Read,           // View item content and metadata
    Write,          // Create/update items
    Delete,         // Remove items
    Admin,          // Manage partition settings and policies
    Grant,          // Grant permissions to others
}
```

### Grants

A **grant** assigns permissions to a principal:

```rust
pub struct Grant {
    pub id: GrantId,
    pub principal: Principal,
    pub permissions: Vec<Permission>,
    pub scope: GrantScope,
    pub created_by: IdentityId,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub enum GrantScope {
    Partition(PartitionId),
    Tag(Tag),
    Item(KnowledgeId),
}
```

**Grant examples:**

```
// Alice can read everything in partition "project/alpha"
Grant {
    principal: Identity("alice"),
    permissions: [Read],
    scope: Partition("project/alpha"),
}

// Engineering group can read/write items tagged "domain:infrastructure"
Grant {
    principal: Group("engineering"),
    permissions: [Read, Write],
    scope: Tag("domain:infrastructure"),
}

// Bob can read one specific item (exception to partition deny)
Grant {
    principal: Identity("bob"),
    permissions: [Read],
    scope: Item("knowledge-item-123"),
}
```

### Policy Evaluation

When checking if principal P can perform action A on item I:

```
1. Collect all grants for P (direct + inherited through groups)
2. Filter grants that apply to I:
   - Partition grants where I.partition matches
   - Tag grants where I.tags intersects grant tags (OR semantics)
   - Item grants where I.id matches
3. Check if any applicable grant includes permission A
4. If yes → ALLOW, else → DENY
```

**Key semantics:**

- **Deny by default** — No grant = no access
- **Tag grants use OR** — Access via any matching tag
- **Item grants override** — Can grant access despite partition policy
- **No explicit deny rules (v1)** — Simplifies reasoning; add later if needed

---

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Fabryk                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                      API Layer                            │  │
│  │                      (axum)                               │  │
│  │  POST /items          - Store knowledge                   │  │
│  │  GET  /items/{id}     - Retrieve item                     │  │
│  │  POST /query          - Search items                      │  │
│  │  GET  /partitions     - List partitions                   │  │
│  │  POST /grants         - Manage permissions                │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    ACL Layer                              │  │
│  │                                                           │  │
│  │  • Identity resolution                                    │  │
│  │  • Group membership expansion                             │  │
│  │  • Grant evaluation                                       │  │
│  │  • Permission checking                                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Query Engine                            │  │
│  │                                                           │  │
│  │  • Keyword search (full-text)                             │  │
│  │  • Semantic search (vector similarity)                    │  │
│  │  • Tag filtering                                          │  │
│  │  • Metadata filtering                                     │  │
│  │  • Result ranking                                         │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  Storage Layer                            │  │
│  │                                                           │  │
│  │  • PostgreSQL (metadata, ACL, full-text search)           │  │
│  │  • pgvector (embeddings, semantic search)                 │  │
│  │  • Object storage (large binary content)                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Crate Structure

```
fabryk/
├── Cargo.toml              (workspace)
├── fabryk-core/            Core types, traits, errors
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs        KnowledgeItem, Partition, Tag, etc.
│   │   ├── error.rs        FabrykError
│   │   └── traits.rs       Storage, QueryEngine traits
│   └── Cargo.toml
│
├── fabryk-acl/             Access control implementation
│   ├── src/
│   │   ├── lib.rs
│   │   ├── identity.rs     Identity, Group types
│   │   ├── grant.rs        Grant, Permission types
│   │   ├── policy.rs       Policy evaluation engine
│   │   └── resolver.rs     Group membership resolution
│   └── Cargo.toml
│
├── fabryk-storage/         Storage backend implementations
│   ├── src/
│   │   ├── lib.rs
│   │   ├── postgres.rs     PostgreSQL implementation
│   │   ├── vector.rs       pgvector integration
│   │   └── object.rs       Object storage (local/S3)
│   └── Cargo.toml
│
├── fabryk-query/           Query engine
│   ├── src/
│   │   ├── lib.rs
│   │   ├── parser.rs       Query DSL parsing
│   │   ├── planner.rs      Query planning
│   │   ├── executor.rs     Query execution
│   │   └── ranking.rs      Result ranking/scoring
│   └── Cargo.toml
│
├── fabryk-api/             HTTP API server
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   ├── routes/
│   │   ├── middleware/
│   │   └── handlers/
│   └── Cargo.toml
│
├── fabryk-client/          Rust client library
│   ├── src/
│   │   ├── lib.rs
│   │   └── client.rs       FabrykClient
│   └── Cargo.toml
│
├── fabryk-mcp/             MCP server (separate proposal)
│   └── ...
│
└── fabryk-cli/             Admin CLI
    ├── src/
    │   └── main.rs
    └── Cargo.toml
```

### Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| HTTP Framework | axum | Tokio ecosystem, type-safe, fast |
| Database | PostgreSQL | Mature, supports pgvector, full-text search |
| Vector Search | pgvector | Integrated with Postgres, good enough for v1 |
| Object Storage | Local filesystem (v1) | Simple; abstract for S3 later |
| Serialization | serde + JSON | Standard Rust ecosystem |
| Async Runtime | Tokio | Same as ECL, de facto standard |

---

## Integration Points

### ECL Integration

ECL workflows can store outputs as knowledge:

```rust
// In ECL's StepContext
impl StepContext {
    pub async fn store_knowledge(
        &self,
        content: impl Into<KnowledgeContent>,
        metadata: KnowledgeMetadata,
    ) -> Result<KnowledgeRef, StepError> {
        let client = self.fabryk_client.as_ref()
            .ok_or(StepError::FabrykNotConfigured)?;

        let item = KnowledgeItem::builder()
            .content(content)
            .partition(self.config.default_partition.clone())
            .tags(self.config.default_tags.clone())
            .provenance(Provenance {
                source: ProvenanceSource::EclWorkflow,
                workflow_id: Some(self.workflow_id.clone()),
                step_id: Some(self.step_id.clone()),
                ..Default::default()
            })
            .merge_metadata(metadata)
            .build()?;

        client.store(item).await.map_err(Into::into)
    }
}
```

**Configuration in ECL workflow:**

```toml
[fabryk]
enabled = true
endpoint = "http://localhost:8100"
default_partition = "project/alpha"
default_tags = ["source:ecl"]

[steps.analysis]
store_as_knowledge = true
knowledge_tags = ["type:analysis", "domain:architecture"]
```

### MCP Integration (Separate Proposal)

Fabryk exposes an MCP server for AI agent access:

```
MCP Tools:
  fabryk_store    - Store new knowledge
  fabryk_query    - Search knowledge base
  fabryk_get      - Retrieve specific item
  fabryk_list     - List items in partition
  fabryk_relate   - Find related items

MCP Resources:
  fabryk://partition/{id}
  fabryk://item/{id}
  fabryk://search/{query}

MCP Prompts:
  summarize_partition
  find_related
  knowledge_report
```

Authentication flows through MCP's auth mechanism, mapped to Fabryk identities.

### Direct API Access

For non-ECL, non-MCP clients:

```rust
use fabryk_client::FabrykClient;

let client = FabrykClient::new("http://localhost:8100")
    .with_api_key("secret")
    .build()?;

// Store
let item = client.store(KnowledgeItem { ... }).await?;

// Query
let results = client.query()
    .partitions(&["project/alpha"])
    .semantic("architecture patterns for microservices")
    .tags(&["type:architecture"])
    .limit(10)
    .execute()
    .await?;

// Retrieve
let item = client.get(&knowledge_id).await?;
```

---

## Risk Assessment

### Risk 1: Complexity of ACL System

**Risk**: Access control adds significant complexity; bugs could leak data.

**Mitigation**:

- Start with simple grant-only model (no explicit deny)
- Extensive unit tests for policy evaluation
- Audit logging for all access decisions
- Security review before production

**Assessment**: Medium risk. Core to the value proposition; must get right.

### Risk 2: Embedding Generation Dependency

**Risk**: Semantic search requires embeddings; external API dependency.

**Mitigation**:

- Abstract embedding provider behind trait
- Support Claude API embeddings initially
- Make embedding optional (keyword search still works)
- Cache embeddings; regenerate only on content change

**Assessment**: Low-Medium risk. Graceful degradation possible.

### Risk 3: Query Performance at Scale

**Risk**: Complex queries across partitions with ACL checks could be slow.

**Mitigation**:

- Denormalize ACL into query-friendly structure
- Index tags and common query patterns
- Implement query result caching
- Pagination required for all list operations

**Assessment**: Medium risk. Requires attention during design.

### Risk 4: PostgreSQL + pgvector Limitations

**Risk**: pgvector may not scale to millions of embeddings.

**Mitigation**:

- Abstract vector storage behind trait
- pgvector sufficient for initial scale (100k-1M items)
- Migration path to dedicated vector DB (Qdrant, Pinecone) if needed
- Monitor query latencies

**Assessment**: Low risk for v1 scale targets.

### Risk 5: Schema Evolution

**Risk**: Knowledge item schema will need to evolve; migrations are hard.

**Mitigation**:

- Store content as JSONB for flexibility
- Version metadata schema explicitly
- Plan migration tooling from start
- Avoid breaking changes to core fields

**Assessment**: Medium risk. Plan for it early.

---

## Recommended Path Forward

### Phase 1: Core Storage (Weeks 1-3)

**Objective**: Basic storage and retrieval without ACL.

1. Set up project structure and dependencies
2. Implement core types (KnowledgeItem, Partition, Tag)
3. Implement PostgreSQL storage backend
4. Implement basic CRUD API endpoints
5. Implement keyword search (Postgres full-text)
6. Write client library basics

**Deliverable**: Store and retrieve knowledge items; basic search.

### Phase 2: Access Control (Weeks 4-6)

**Objective**: Full ACL implementation.

1. Implement Identity and Group types
2. Implement Grant and Permission types
3. Implement policy evaluation engine
4. Add ACL checks to all API endpoints
5. Add authentication middleware (API key for v1)
6. Comprehensive ACL test suite

**Deliverable**: Multi-user access with permission enforcement.

### Phase 3: Semantic Search (Weeks 7-8)

**Objective**: Vector similarity search.

1. Integrate pgvector extension
2. Implement embedding storage
3. Create embedding provider abstraction
4. Implement Claude API embedding provider
5. Add semantic search to query engine
6. Hybrid ranking (keyword + semantic)

**Deliverable**: Semantic search capabilities.

### Phase 4: ECL Integration (Weeks 9-10)

**Objective**: Seamless ECL workflow integration.

1. Implement `fabryk-client` crate
2. Add `StepContext::store_knowledge()`
3. Add workflow-level Fabryk configuration
4. Add provenance tracking from ECL
5. Integration tests with ECL workflows
6. Documentation for ECL users

**Deliverable**: ECL workflows can store knowledge.

### Phase 5: Hardening (Weeks 11-12)

**Objective**: Production readiness.

1. Audit logging implementation
2. Rate limiting
3. Performance optimization
4. Deployment packaging (Docker, K8s)
5. Monitoring and alerting
6. Security review

**Deliverable**: Production-ready v1.

---

## Success Criteria for v1

1. **Storage**: Knowledge items persist reliably with full CRUD
2. **Retrieval**: Keyword and semantic search return relevant results
3. **Access Control**: Permissions enforced correctly; no data leakage
4. **Multi-tenancy**: Multiple users with isolated and shared partitions
5. **ECL Integration**: Workflows can store outputs as knowledge
6. **Performance**: <100ms for typical queries at 100k items
7. **Auditability**: All access decisions logged

---

## Open Questions Requiring Decision

### Q1: Embedding Provider

**Option A**: Claude API only (consistent with ECL)

- Pro: Single provider, simpler
- Con: API cost for embeddings

**Option B**: Multiple providers (OpenAI, local models)

- Pro: Flexibility, cost optimization
- Con: Complexity, consistency concerns

**Recommendation**: Start with Claude API; abstract for future flexibility.

### Q2: Object Storage for Large Content

**Option A**: Store all content in PostgreSQL

- Pro: Transactional consistency, simpler
- Con: Postgres not optimized for large blobs

**Option B**: PostgreSQL metadata + filesystem/S3 for content

- Pro: Better performance for large items
- Con: Consistency complexity

**Recommendation**: PostgreSQL for v1 (<10MB items); abstract for S3 later.

### Q3: Query Language

**Option A**: Simple filter API (JSON-based)

- Pro: Easy to implement, clear semantics
- Con: Limited expressiveness

**Option B**: Query DSL (SQL-like or custom)

- Pro: Powerful queries
- Con: Parsing complexity, learning curve

**Recommendation**: Simple filter API for v1; evaluate DSL need based on usage.

### Q4: Real-time Updates

**Option A**: Polling only

- Pro: Simple implementation
- Con: Latency for collaborative use cases

**Option B**: WebSocket subscriptions

- Pro: Real-time updates
- Con: Connection management complexity

**Recommendation**: Polling for v1; WebSocket if real-time needs emerge.

---

## Conclusion

Fabryk addresses a fundamental gap in AI systems: the lack of persistent, secure, queryable knowledge storage. By providing a robust knowledge fabric with fine-grained access control, Fabryk enables AI workflows and agents to accumulate and leverage institutional knowledge over time.

The architecture builds on proven technologies (PostgreSQL, pgvector, axum) while abstracting for future flexibility. The phased implementation plan delivers value incrementally, with ECL integration as a key milestone.

Combined with ECL for workflow orchestration and MCP for AI agent access, Fabryk completes a powerful ecosystem for knowledge-augmented AI applications.

We recommend proceeding with the proposed architecture and phased implementation plan.

---

## References

### Related Documents

- 0003-ecl-project-proposal.md — ECL workflow engine proposal
- 0004-ecl-project-plan.md — ECL implementation plan
- ecl-ecosystem-vision-summary.md — Expanded vision context

### Technologies

- PostgreSQL: <https://postgresql.org>
- pgvector: <https://github.com/pgvector/pgvector>
- axum: <https://github.com/tokio-rs/axum>
- SQLx: <https://github.com/launchbadge/sqlx>

### MCP (for future integration)

- MCP Specification: <https://modelcontextprotocol.io>
- rmcp (Rust SDK): <https://github.com/modelcontextprotocol/rust-sdk>

---

## Appendix A: Example Queries

### Keyword Search

```json
{
  "partitions": ["project/alpha"],
  "keyword": "microservices architecture",
  "tags": ["type:architecture"],
  "limit": 10
}
```

### Semantic Search

```json
{
  "partitions": ["project/alpha", "team/engineering"],
  "semantic": "How should we structure our payment processing service?",
  "limit": 5
}
```

### Combined Query

```json
{
  "partitions": ["project/alpha"],
  "semantic": "error handling patterns",
  "tags": ["type:code-review"],
  "created_after": "2026-01-01T00:00:00Z",
  "limit": 20
}
```

---

## Appendix B: Database Schema Sketch

```sql
-- Core tables
CREATE TABLE partitions (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    owner_id UUID NOT NULL,
    owner_type TEXT NOT NULL,  -- 'identity' or 'group'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE knowledge_items (
    id UUID PRIMARY KEY,
    partition_id UUID NOT NULL REFERENCES partitions(id),
    owner_id UUID NOT NULL,

    title TEXT NOT NULL,
    summary TEXT,
    content TEXT,
    content_type TEXT NOT NULL,

    provenance JSONB NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    -- Full-text search
    search_vector TSVECTOR GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(title, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(summary, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(content, '')), 'C')
    ) STORED
);

CREATE TABLE knowledge_tags (
    item_id UUID NOT NULL REFERENCES knowledge_items(id) ON DELETE CASCADE,
    namespace TEXT,
    value TEXT NOT NULL,
    PRIMARY KEY (item_id, namespace, value)
);

CREATE TABLE knowledge_embeddings (
    item_id UUID PRIMARY KEY REFERENCES knowledge_items(id) ON DELETE CASCADE,
    embedding vector(1536),  -- Adjust dimension for model
    model TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ACL tables
CREATE TABLE identities (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    identity_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE groups (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    owner_id UUID NOT NULL REFERENCES identities(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE group_members (
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    member_id UUID NOT NULL,
    member_type TEXT NOT NULL,  -- 'identity' or 'group'
    PRIMARY KEY (group_id, member_id, member_type)
);

CREATE TABLE grants (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL,
    principal_type TEXT NOT NULL,
    permissions TEXT[] NOT NULL,
    scope_type TEXT NOT NULL,  -- 'partition', 'tag', 'item'
    scope_value TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES identities(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

-- Indexes
CREATE INDEX idx_items_partition ON knowledge_items(partition_id);
CREATE INDEX idx_items_search ON knowledge_items USING GIN(search_vector);
CREATE INDEX idx_embeddings_vector ON knowledge_embeddings USING ivfflat(embedding vector_cosine_ops);
CREATE INDEX idx_tags_value ON knowledge_tags(namespace, value);
CREATE INDEX idx_grants_principal ON grants(principal_id, principal_type);
CREATE INDEX idx_grants_scope ON grants(scope_type, scope_value);
```
