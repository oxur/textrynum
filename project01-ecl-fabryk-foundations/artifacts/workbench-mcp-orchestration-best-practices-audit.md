
<!-- Historical source: workbench/mcp-orchestration-best-practices-audit.md, lines 1-230. Original bytes are recoverable from import commit 3a39aea. -->

# MCP Server AI-Accessibility Audit

**For use by Claude Code as a periodic health check on MCP server deployments.**

*Version 0.1 — February 2026*
*Run this audit after deploying new servers, adding tools, or making significant schema changes.*

---

## How to Use This Document

This is a structured audit designed to be run by Claude Code against one or more MCP servers. For each section, Claude Code should check the listed criteria and flag any failures. The audit produces a pass/warn/fail rating for each category and a summary with prioritised action items.

Run with: `claude "Run the MCP AI-accessibility audit against [server name / all servers]"`

---

## Audit 1: Tool Discoverability

**Goal:** Can an AI find and correctly use every tool without prior knowledge?

For each tool exposed by the server, check:

### 1.1 — Name Quality
- [ ] Tool name follows verb-noun or domain_verb pattern
- [ ] Name is self-explanatory to someone unfamiliar with the codebase
- [ ] No internal jargon, abbreviations, or implementation-leak in the name
- [ ] Consistent naming pattern across all tools in this server
- [ ] If multi-server: consistent with naming patterns in sibling servers

**Flag as FAIL if:** Tool name requires reading source code to understand (e.g., `fts_q`, `gx_adj`, `idx_lu`)

### 1.2 — Description Quality
- [ ] Description exists and is non-empty
- [ ] Description includes WHAT the tool does (one sentence)
- [ ] Description includes WHEN to use it (triggering conditions)
- [ ] Description includes WHAT it returns (response shape/content)
- [ ] Description includes at least one concrete example
- [ ] Description mentions cross-domain relevance (if applicable)

**Flag as FAIL if:** Description is missing or is just the tool name restated.
**Flag as WARN if:** Description lacks "when to use" guidance or examples.

### 1.3 — Parameter Quality
- [ ] All parameters have descriptions
- [ ] Required vs. optional is clearly indicated
- [ ] Enum parameters list all valid values with explanations
- [ ] Default values are documented
- [ ] Parameter names are self-explanatory

**Flag as WARN if:** Parameters lack descriptions or have ambiguous names.

---

## Audit 2: Response Quality

**Goal:** Are tool responses self-contained, structured, and actionable?

Test each tool with representative queries and check responses:

### 2.1 — Metadata Completeness
- [ ] Response includes domain/partition identifier
- [ ] Response includes content type (concept, guide, source, etc.)
- [ ] Response includes item ID or stable reference
- [ ] Response includes title/name (human-readable)
- [ ] Response includes brief/summary (not just full content)

**Flag as WARN if:** Responses are "content only" with no surrounding metadata.

### 2.2 — Connection Information
- [ ] Response includes related items (within same domain)
- [ ] Response includes cross-domain connections (if they exist)
- [ ] Connections include enough info to evaluate relevance (not just IDs)
- [ ] Response includes tags (both domain-specific and structural)

**Flag as FAIL if:** No connection information is ever returned.
**Flag as WARN if:** Cross-domain connections are absent.

### 2.3 — Depth Layering
- [ ] Search/list operations return summary-level results (not full content dumps)
- [ ] Get operations return full content with all metadata
- [ ] Summary-level results include enough info to decide whether to fetch full content
- [ ] Full content doesn't overwhelm context window (check size for large items)

**Flag as WARN if:** Search returns full content for every result (context window bloat).

---

## Audit 3: Cross-Domain Readiness

**Goal:** Can this server participate effectively in multi-server, cross-domain conversations?

### 3.1 — Structural Tagging
- [ ] Items include structural/pattern tags (not just domain categories)
- [ ] Tags use a shared vocabulary (e.g., `pattern:transformation`, `pattern:composition`)
- [ ] Tags are consistent with the structural tag vocabulary used by sibling servers

**Flag as WARN if:** Tags are domain-only with no structural/cross-cutting tags.

### 3.2 — Cross-Domain Connections
- [ ] Items that have known analogues in other domains include explicit cross-references
- [ ] Cross-references include the target domain and item identifier
- [ ] Cross-references include a brief description of *why* the connection exists

**Flag as WARN if:** No cross-domain connections exist for a domain with known analogues.

### 3.3 — Cross-Domain Search Participation
- [ ] Server is registered with the hub (if hub-and-spoke architecture is in use)
- [ ] Server's content is indexed and searchable via the cross-domain search tool
- [ ] Cross-domain search returns results from this server for relevant queries

**Flag as FAIL if:** Server is unreachable from the cross-domain search hub.

---

## Audit 4: Resource and Prompt Quality

### 4.1 — Resources
- [ ] Resource URIs are human-readable and follow a consistent pattern
- [ ] Resources use domain-name paths (not numeric IDs)
- [ ] A context-primer resource exists for the domain (overview of what's available)
- [ ] Resources are browsable (list operations expose available resource paths)

**Flag as WARN if:** No context-primer resource exists.

### 4.2 — Prompts
- [ ] At least one discovery-oriented prompt exists (e.g., `explore_connections`)
- [ ] At least one cross-domain prompt exists (e.g., `cross_reference`)
- [ ] Prompts include descriptions of what they do and when to use them
- [ ] Prompts encode multi-step workflows, not just simple query templates

**Flag as WARN if:** No prompts are defined, or all prompts are simple query wrappers.

---

## Audit 5: Multi-Server Consistency

**Goal:** Do all servers in the ecosystem play well together?

*Run this section across all active servers simultaneously.*

### 5.1 — Naming Consistency
- [ ] All servers use the same tool naming pattern (verb-noun or domain_verb)
- [ ] Equivalent operations have equivalent names across servers
- [ ] No tool name collisions between servers
- [ ] No ambiguous tools where the AI might call the wrong server's version

### 5.2 — Schema Consistency
- [ ] All servers return responses with the same top-level structure
- [ ] Common fields (id, title, domain, tags, connections) use the same names
- [ ] Content types are consistent across servers (e.g., "concept-card" not "concept" vs "card")
- [ ] Tag vocabulary is shared (structural tags mean the same thing everywhere)

### 5.3 — Discovery Infrastructure
- [ ] A `list_domains` or equivalent tool exists (globally, across all servers)
- [ ] Cross-domain search works and returns results from all active servers
- [ ] The AI can determine what servers exist and what each covers without prior knowledge
- [ ] New servers are automatically discoverable (or at minimum, easily registered)

**Flag as FAIL if:** No global discovery mechanism exists.

---

## Audit 6: Content Health

**Goal:** Is the knowledge base well-maintained and actively useful?

### 6.1 — Coverage
- [ ] Key concepts in the domain have concept cards
- [ ] Guides exist for common tasks and cross-cutting concerns
- [ ] Debates/open-questions are tracked (not just resolved knowledge)
- [ ] Sources are attributed and accessible

### 6.2 — Freshness
- [ ] Recently added content is discoverable via `recent_additions` or equivalent
- [ ] Stale or superseded content is marked as such
- [ ] Cross-domain connections have been reviewed after adding new content to any domain

### 6.3 — Connection Density
- [ ] Average number of connections per item (target: ≥ 3 within-domain, ≥ 1 cross-domain)
- [ ] No orphan items (zero connections)
- [ ] Connection graph is navigable (no isolated clusters)

**Flag as WARN if:** Many items have zero cross-domain connections.
**Flag as WARN if:** Orphan items exist (items with no connections at all).

---

## Audit Output Format

After running the audit, produce a summary in this format:

```
# MCP AI-Accessibility Audit Results
Date: [date]
Servers audited: [list]

## Summary
| Category                  | Status | Score |
|---------------------------|--------|-------|
| Tool Discoverability      | PASS/WARN/FAIL | X/Y checks passed |
| Response Quality          | PASS/WARN/FAIL | X/Y checks passed |
| Cross-Domain Readiness    | PASS/WARN/FAIL | X/Y checks passed |
| Resources & Prompts       | PASS/WARN/FAIL | X/Y checks passed |
| Multi-Server Consistency  | PASS/WARN/FAIL | X/Y checks passed |
| Content Health            | PASS/WARN/FAIL | X/Y checks passed |

## Priority Action Items
1. [FAIL items first, then WARNs, ordered by impact]
2. ...

## Detailed Findings
[Per-server, per-category breakdown]
```

---

## Audit Frequency

| Trigger | Scope |
|---------|-------|
| New server deployed | Full audit on new server + Audit 5 (multi-server consistency) |
| New tools added to existing server | Audits 1, 2, 5 on affected server |
| Significant content additions | Audits 3, 6 on affected server |
| Monthly maintenance | Full audit across all servers |
| After cross-domain conversation reveals a gap | Targeted Audit 3 on relevant servers |

---

*This audit document should itself be updated when new patterns or failure modes are discovered. Treat it as a living checklist that evolves with the ecosystem.*
