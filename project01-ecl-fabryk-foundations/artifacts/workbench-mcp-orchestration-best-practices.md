
<!-- Historical source: workbench/mcp-orchestration-best-practices.md, lines 1-325. Original bytes are recoverable from import commit 3a39aea. -->

# Designing MCP Servers for Cross-Domain, Knowledge-Rich Environments

**A Practitioner's Guide from the AI's Perspective**

*Version 0.1 — February 2026*
*Based on real-world experience building and consuming Fabryk, ECL, and domain-specific knowledge MCP servers across music theory, abstract mathematics, and Rust programming.*

---

## Preamble: Why This Document Exists

When you run 10 MCP servers simultaneously — spanning music theory, abstract algebra, Rust programming, work project management, and more — the AI consuming those servers faces a combinatorial discovery problem. The question isn't just "can I find the right tool?" but "do I even *know* to look there?"

This guide captures what actually makes MCP servers effortless to navigate, with particular attention to the cross-domain "unexpected insight" problem: how do you make it easy for an AI to reach for a music theory concept when you're debugging a type system, or pull a category theory insight into a workflow design conversation?

---

## Part 1: Making Individual Servers Effortless to Navigate

### 1.1 — Name Things for the Consumer, Not the Developer

Tool and resource names are the AI's first (and often only) signal about what's available. Every name should pass the **"cold start" test**: if the AI has never used this server before, can it guess what the tool does from the name alone?

**Principle: Verb-noun tool names, noun-path resource URIs.**

Good tool names read like actions: `search_concepts`, `get_guide`, `find_related`, `list_debates`. Bad tool names are internal jargon: `fts_query`, `idx_lookup`, `get_node_adj_list`.

Good resource URIs read like library shelves: `fabryk://music-theory/concepts/picardy-third`, `fabryk://abstract-algebra/guides/group-actions`. Bad resource URIs expose implementation: `fabryk://partition/3/item/a7f2`.

**Why this matters at scale:** When the AI has 50+ tools across 10 servers, it's doing a mental `grep` over tool names every single turn. Descriptive names are the difference between the AI finding the right tool instantly and it missing the tool entirely.

### 1.2 — Write Tool Descriptions as If They're the Only Documentation

The tool description string in your MCP server definition is the single most important piece of text in your entire codebase from the AI's perspective. It's the tooltip, the man page, and the README rolled into one.

Every tool description should include:

- **What it does** (one sentence)
- **When to use it** (the triggering condition — this is the most commonly missing piece)
- **What it returns** (shape and content of the response)
- **One concrete example** (input → output, even abbreviated)

```
search_concepts: Search across all knowledge domains for concept cards
matching a query. Use when exploring a topic, looking for connections
across fields, or when a conversation touches on theoretical foundations.
Returns ranked results with domain, title, and relevance snippet.
Example: query="isomorphism" → results from algebra, music theory, type theory.
```

The "when to use it" line is what enables cross-domain discovery. Without it, the AI will only reach for the tool when the conversation is explicitly about that domain.

### 1.3 — Provide Rich, Structured Metadata in Responses

When a tool returns results, include enough metadata that the AI can decide what to do next without making another round trip.

Every response should include:

- **Domain/partition** — which knowledge area this came from
- **Content type** — concept card, guide, source excerpt, debate, etc.
- **Connections** — what other items relate to this one (even just IDs/titles)
- **Confidence/relevance signal** — search score, exact vs. fuzzy match, etc.

The connections field is what powers serendipitous discovery. When the AI searches for "group action" in algebra and the result says `related: ["symmetry-operations (music-theory)", "functor-action (category-theory)"]`, that's the cross-domain bridge being built automatically.

### 1.4 — Use Prompts as Conversation Starters, Not Just Templates

MCP prompts are underutilised. Most servers treat them as simple query templates, but they're actually the perfect mechanism for teaching the AI *how to think about your domain*.

Design prompts that encode workflows, not just queries:

- `explore_connections`: "Given concept X, find related concepts across all domains, identify structural similarities, and suggest unexpected applications"
- `debate_status`: "For topic X, summarise the current state of unresolved debates, competing definitions, and open questions"
- `cross_reference`: "Find where concept X from domain A appears (possibly under a different name) in domains B and C"

These prompts serve double duty: they're useful templates the user can invoke directly, and they teach the AI what kinds of multi-step reasoning your knowledge base supports.

---

## Part 2: Making Multiple Servers Work Together

### 2.1 — Establish a Shared Vocabulary Layer

The single biggest barrier to cross-domain AI reasoning is inconsistent terminology. When your music theory server calls something a "transformation" and your algebra server calls the structurally identical thing a "morphism" and your Rust server calls it a "trait implementation," the AI has to do the mapping itself — and it often won't, because it doesn't know the mapping exists.

**Solution: A shared ontology resource.**

This can be a dedicated MCP resource (or even a small dedicated server) that provides:

- **Cross-domain term mappings**: `{algebra: "isomorphism", music: "enharmonic equivalence", programming: "lossless type conversion"}`
- **Structural pattern names**: Names for recurring patterns that span domains (e.g., "structure-preserving map" as the umbrella for morphism/transformation/conversion)
- **Analogy index**: Explicitly documented analogies between domains, ranked by strength

This doesn't need to be comprehensive up front. Start with the mappings you discover naturally in cross-domain conversations, and grow it over time. The music theory ↔ abstract algebra connections you've been building are a perfect seed.

### 2.2 — Consistent Naming Conventions Across Servers

All your MCP servers should follow the same structural conventions so the AI can transfer its understanding of one server to all the others.

Adopt a universal pattern:

| Operation | Tool Name Pattern | Example |
|-----------|-------------------|---------|
| Search | `{domain}_search` or `search_{content_type}` | `music_search`, `search_concepts` |
| Retrieve | `{domain}_get` or `get_{content_type}` | `algebra_get`, `get_guide` |
| List | `{domain}_list` or `list_{content_type}` | `rust_list`, `list_debates` |
| Relate | `{domain}_related` or `find_connections` | `music_related`, `find_connections` |
| Store | `{domain}_store` or `store_{content_type}` | `ecl_store`, `store_concept` |

**If you're using Fabryk across all domains**, the naming is even simpler: all servers expose the same `fabryk_*` tools, differentiated by partition/domain parameters. This is the ideal state — it means the AI learns one API and it works everywhere.

### 2.3 — Implement a Cross-Domain Search

This is the highest-leverage single feature for enabling "unexpected source" discoveries. Provide a tool that searches across all domains simultaneously, returning results ranked by relevance regardless of which partition they came from.

```
fabryk_search_all:
  query: "symmetry breaking"
  results:
    - domain: physics, item: spontaneous-symmetry-breaking, score: 0.95
    - domain: music-theory, item: deceptive-cadence, score: 0.72
    - domain: algebra, item: quotient-group, score: 0.68
    - domain: rust, item: enum-exhaustiveness, score: 0.41
```

The music theory result showing up for "symmetry breaking" is exactly the kind of cross-domain connection that makes these conversations so productive. But it only happens if there's a single search endpoint that spans everything.

### 2.4 — Tag with Cross-Cutting Concerns, Not Just Domain Categories

Tags should include both domain-specific categories *and* cross-cutting structural/conceptual tags:

- Domain tags: `domain:music-theory`, `category:harmony`, `source:schoenberg`
- Structural tags: `pattern:transformation`, `pattern:equivalence-class`, `pattern:composition`, `property:commutative`
- Meta tags: `status:debate`, `confidence:established`, `cross-ref:algebra`

The structural tags are what make cross-domain search work. When the AI searches for `pattern:composition`, it should find function composition (programming), chord progression (music), morphism composition (algebra), and workflow chaining (ECL) all in the same result set.

---

## Part 3: Optimising for the AI's Cognitive Model

### 3.1 — Front-Load Context in Every Response

The AI doesn't have persistent memory of your MCP server's full contents. Every tool response should be self-contained enough that the AI can use it without needing to make three more calls to understand what it's looking at.

**Bad response:**
```json
{"id": "picardy-third", "type": "concept", "content": "..."}
```

**Good response:**
```json
{
  "id": "picardy-third",
  "domain": "music-theory",
  "type": "concept-card",
  "title": "Picardy Third",
  "brief": "Major chord resolution at the end of a minor-key passage",
  "content": "...",
  "connections": ["modal-mixture", "tierce-de-picardie", "chromatic-mediant"],
  "cross_domain": ["type-coercion (programming)", "group-completion (algebra)"],
  "tags": ["harmony", "cadence", "pattern:type-change", "era:baroque"],
  "guides_referencing": ["guide:cadence-types", "guide:modal-interchange"]
}
```

The `cross_domain` and `guides_referencing` fields are what let the AI chain naturally from one result to the next without you prompting it.

### 3.2 — Provide Discovery Tools, Not Just Retrieval Tools

Most MCP servers only implement retrieval: "give me the thing I already know I want." But the AI also needs discovery: "show me what's here that I didn't know to ask for."

Implement at minimum:

- **`list_domains`** — what knowledge areas exist (so the AI knows to look)
- **`list_tags`** (with optional domain filter) — what conceptual categories exist
- **`random_connection`** or **`surprise_me`** — given current context/topic, surface an unexpected but relevant item from a different domain
- **`recent_additions`** — what's new, so the AI can update its mental model of what's available

The `surprise_me` tool might sound whimsical, but it's the explicit mechanism for the "unexpected source" pattern you asked about. It could work by taking the current conversation topic, extracting structural tags, and finding items in other domains that share those structural tags but are in a completely different subject area.

### 3.3 — Design for Incremental Depth

The AI should be able to get a useful overview in one call and go deeper only when needed. This means every content type should have at least two levels of detail:

- **Summary level**: Title, brief description, domain, key tags, connections (returned by search and list operations)
- **Full level**: Complete content, all metadata, full relationship graph (returned by get operations)

This is especially important at scale. When the AI has 10 servers and needs to figure out which 2 of them are relevant to the current conversation, it needs fast, lightweight overview calls — not full content dumps that consume its context window.

---

## Part 4: Practical Architecture for 10+ Servers

### 4.1 — The Hub-and-Spoke Model

For your setup, I'd recommend a hub-and-spoke architecture:

```
                    ┌──────────────────────┐
                    │   fabryk-hub (MCP)    │
                    │                      │
                    │  • cross-domain search│
                    │  • ontology/mappings  │
                    │  • discovery tools    │
                    │  • list_all_domains   │
                    └──────────┬───────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
    ┌─────────▼──┐   ┌────────▼───┐   ┌────────▼───┐
    │music-theory│   │  algebra   │   │    rust    │   ... (more spokes)
    │  (Fabryk)  │   │  (Fabryk)  │   │  (Fabryk)  │
    └────────────┘   └────────────┘   └────────────┘
```

The hub server provides cross-domain operations. Each spoke is a domain-specific Fabryk instance. The hub knows about all spokes and can federate queries across them.

This means the AI only needs to discover the hub to gain access to everything. Individual spoke servers are still available for deep domain-specific work.

### 4.2 — Configuration Over Convention

Create a standard `mcp-manifest.json` (or equivalent) for each server that declares:

```json
{
  "server_name": "music-theory",
  "domain": "music-theory",
  "description": "Western music theory: harmony, counterpoint, form, analysis",
  "cross_domain_tags": ["pattern:transformation", "pattern:symmetry", "pattern:hierarchy"],
  "hub_registered": true,
  "capabilities": ["search", "get", "list", "relate", "store"],
  "content_types": ["concept-card", "guide", "source", "debate"],
  "version": "0.3.1"
}
```

This manifest serves as both documentation and machine-readable configuration for the hub.

### 4.3 — Monitor What the AI Actually Uses

Instrument your servers to track which tools the AI calls, what queries it makes, and — critically — which cross-domain paths it discovers. This usage data is gold for:

- Identifying tools that need better descriptions (never called despite being relevant)
- Finding missing cross-domain connections (queries that return no results but should)
- Pruning tools that add noise without value
- Discovering usage patterns you hadn't anticipated

---

## Part 5: The Cross-Domain Catalyst Patterns

These are specific design patterns for enabling those "spur of the moment" connections you asked about.

### 5.1 — The Structural Fingerprint

Every concept card should include a "structural fingerprint" — a set of abstract properties that describe *what kind of thing it is* independent of domain:

```yaml
structural_fingerprint:
  - pattern: transformation        # it maps one thing to another
  - property: invertible           # the mapping can be reversed
  - property: structure-preserving # it maintains relationships
  - arity: binary                  # it operates on pairs
  - composable: true               # multiple can be chained
```

Two items from completely different domains with similar fingerprints are prime candidates for cross-domain insight.

### 5.2 — The Analogy Bridge

Maintain explicit, curated analogies between domains. Not automated similarity scores — actual documented reasoning about *why* two things from different domains illuminate each other:

```yaml
analogy:
  source: music-theory/voice-leading
  target: algebra/continuous-group-action
  relationship: "Voice leading rules are constraints on paths through pitch-class
    space, just as continuous group actions are constraints on paths through a
    topological space. The 'shortest path' principle in voice leading is the
    musical equivalent of geodesics."
  strength: strong
  discovered_in: "conversation-2026-01-15"
  implications:
    - "Graph-distance metrics from algebra apply to voice leading analysis"
    - "Musical 'smoothness' can be formalised as continuity"
```

These analogy bridges are the highest-value content in the entire system for cross-domain conversations. They should be first-class citizens, not afterthoughts.

### 5.3 — The Context Priming Pattern

When the AI begins a conversation that touches on a domain, the MCP server can proactively surface a "context primer" — a lightweight summary of what's available and what's interesting:

```
fabryk://music-theory/context-primer

"This knowledge base covers Western music theory with particular depth in:
harmony (functional and non-functional), counterpoint (species and free),
form analysis, and 20th-century techniques. Notable cross-domain connections
exist with abstract algebra (group theory applied to pitch-class sets,
category theory applied to transformational theory) and programming
(type systems as analogues for key relationships, composition patterns).
Recent additions: Riemannian harmonic analysis, neo-Riemannian theory."
```

This primer can be exposed as an MCP resource that the AI reads at the start of relevant conversations, giving it a map of the territory.

---

## Summary: The Minimum Viable Multi-Server Setup

If you do nothing else, do these five things:

1. **Write tool descriptions as if they're the only documentation** — include "when to use" triggers
2. **Implement cross-domain search** — one search endpoint that spans all servers
3. **Tag with structural patterns** — not just domain categories
4. **Include connections in every response** — especially cross-domain connections
5. **Provide a `list_domains` tool** — so the AI knows what's available

Everything else in this document builds on these five foundations.

---

*This document is a living guide. Update it as we discover new patterns through actual cross-domain conversations and as the Fabryk ecosystem matures.*
