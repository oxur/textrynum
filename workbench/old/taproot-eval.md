# Taproot GraphEngine Evaluation & Phase 4 Adaptation Recommendations

## Context

We completed Phases 1-3 of the Fabryk extraction (fabryk-core, fabryk-content, fabryk-fts) and are preparing for Phase 4 (fabryk-graph). Meanwhile, a production petgraph implementation was built in the Taproot project at `~/lab/banyan/taproot/crates/taproot-server/src/knowledge/graph.rs` (1,117 lines, 46+ tests). This evaluation compares Taproot's battle-tested implementation against our Phase 4 plans (docs 0018-0025) to identify strengths, weaknesses, and recommended adaptations.

---

## Taproot Implementation Summary

**Architecture:** Single `GraphEngine` struct wrapping `DiGraph<GraphNode, EdgeType>` with `HashMap<String, NodeIndex>` for O(1) slug lookups.

**Key features:**
- Two-phase build: nodes first, then edges (handles forward references)
- 6 edge types as enum: `Prerequisite`, `Extends`, `Related`, `ContrastsWith`, `QueriesAbout`, `UsesTable`
- Runtime mutation: `add_node()`, `add_edge()`, `add_concept_card()`
- User model integration: `NodeType::UserQuery` nodes with `QueriesAbout` edges
- Algorithms: A* pathfinding, BFS neighborhood (capped 5 hops), degree centrality, prerequisite/dependent traversal
- Graceful dangling reference handling (log + skip)
- Validation: orphan node detection
- Stats: node/edge counts, category/tier/edge-type breakdowns, average degree

---

## Evaluation: Taproot Strengths

### S1: Runtime Graph Mutation (HIGH VALUE)
Taproot's `add_node()`, `add_edge()`, and `add_concept_card()` enable runtime graph evolution. This is **absent from the Phase 4 plans**, which only describe build-time graph construction via `GraphBuilder`. Runtime mutation is essential for:
- Adding user query nodes to track learning paths
- Dynamically integrating new content without full rebuilds
- Supporting interactive graph exploration tools

### S2: User Model Integration (HIGH VALUE)
The `NodeType` enum (`DomainConcept` vs `UserQuery`) and the `QueriesAbout` edge type create a clean user-model layer on top of the domain graph. This enables:
- `unexplored_concepts()` - finding gaps in a user's knowledge
- `user_query_nodes()` - tracking what users have explored
- Differentiated edge semantics (user-query prerequisites become `QueriesAbout` edges)
This pattern is production-proven and directly applicable to fabryk-graph.

### S3: Graceful Dangling Reference Handling (MEDIUM VALUE)
Every edge-building path checks `slug_to_node.contains_key()` before creating edges, logging dangling refs instead of erroring. This is pragmatic for real-world data where references may be incomplete. The Phase 4 plans mention validation but not this specific pattern in the builder.

### S4: Two-Phase Build with Duplicate Edge Prevention (MEDIUM VALUE)
The `build()` method creates all nodes first (Phase 1), then all edges (Phase 2). For bidirectional edges (`Related`, `ContrastsWith`), it explicitly checks `edges_connecting()` to prevent duplicates when both cards reference each other. This is a subtle but important correctness detail.

### S5: Practical Algorithm Set (MEDIUM VALUE)
The algorithm choices reflect real usage patterns:
- `get_prerequisites()` / `get_dependents()` - typed edge traversal (only `Prerequisite` edges)
- `get_concept_neighborhood()` with depth cap (5 hops) - prevents runaway BFS
- `get_central_concepts()` with degree centrality - simple but effective
- `find_concept_path()` with A* - lightweight pathfinding

### S6: Strong Test Coverage (MEDIUM VALUE)
46+ tests covering build, empty graphs, traversals, error paths, runtime mutation, user model, and real seed data integration. The test helper `make_test_card()` is clean and reusable.

---

## Evaluation: Taproot Weaknesses

### W1: No Trait Abstraction (HIGH IMPACT)
`GraphEngine` is tightly coupled to `ConceptCard` and domain-specific types (`Category`, `Tier`, `CardFrontmatter`). There's no extractor trait, so the graph construction logic cannot be reused for other domains. **Phase 4 correctly addresses this** with the `GraphExtractor` trait.

### W2: No Persistence (HIGH IMPACT)
The graph lives entirely in memory and must be rebuilt from source files on every startup. For large knowledge bases, this is expensive. **Phase 4 correctly addresses this** with JSON serialization + optional rkyv caching.

### W3: No Edge Weights (MEDIUM IMPACT)
All edges are unweighted (A* uses `|_| 1` cost). This means pathfinding can't distinguish between strong prerequisites and weak "related" connections. **Phase 4 correctly addresses this** with `Edge::weight` and `Relationship::default_weight()`.

### W4: No Edge Provenance Tracking (MEDIUM IMPACT)
Edges don't record their origin (frontmatter vs. content body vs. manual). Debugging why an edge exists requires re-examining source files. **Phase 4 correctly addresses this** with `EdgeOrigin`.

### W5: Limited Validation (LOW IMPACT)
Only orphan detection. No cycle detection, no self-loop detection, no duplicate edge detection. **Phase 4 plans include** more comprehensive validation.

### W6: Single-File Architecture (LOW IMPACT)
1,117 lines in one file is manageable but will become unwieldy as the system grows. **Phase 4 correctly splits** into types, extractor, algorithms, persistence, builder, validation, stats modules.

### W7: `unwrap()` Usage in Serialization (LOW IMPACT, AP-09 violation)
The `stats()` method and `build()` use `unwrap_or_default()` and `unwrap_or_else()` on serde serialization, which is fine for display strings but not ideal. A minor Rust anti-patterns concern.

---

## Recommended Adaptations to Phase 4 Plans

### A1: Add Runtime Mutation API to GraphData (NEW - from S1)

**Current plan gap:** `GraphData` is built once via `GraphBuilder` and is effectively immutable.

**Recommendation:** Add mutation methods to `GraphData`:

```rust
impl GraphData {
    pub fn add_node(&mut self, node: Node) -> NodeIndex { ... }
    pub fn add_edge(&mut self, edge: Edge) -> Result<()> { ... }
    pub fn remove_node(&mut self, id: &str) -> Result<Node> { ... }
}
```

**Where:** Add to milestone 4.1 (types.rs) — `GraphData` mutation methods.

**Why:** Enables user model integration, incremental updates, and interactive tools without requiring a full `GraphBuilder` round-trip.

### A2: Add NodeType Enum for User Model Support (NEW - from S2)

**Current plan gap:** `Node` has no concept of node types (domain vs. user-generated).

**Recommendation:** Add a `NodeType` field to `Node`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// Content from the knowledge domain.
    Domain,
    /// User-generated query or interaction node.
    UserQuery,
    /// Custom node type for domain-specific use.
    Custom(String),
}
```

And add a `node_type: NodeType` field to `Node` (defaulting to `Domain`).

**Where:** Milestone 4.1 (types.rs).

**Why:** Enables the user model pattern that proved valuable in Taproot. The `Custom(String)` variant follows the established fabryk pattern for extensibility.

### A3: Add Dangling Reference Handling Strategy to GraphBuilder (ENHANCE - from S3)

**Current plan gap:** Doc 0022 (builder) mentions `ErrorHandling` enum (`FailFast`, `Collect`, `Skip`) but doesn't specifically address dangling edge references.

**Recommendation:** The builder should:
1. Log dangling references at `debug!` level (like Taproot)
2. Collect them in build stats as `dangling_references: Vec<(String, String, Relationship)>`
3. Never fail on dangling refs by default (they're expected in real data)

**Where:** Milestone 4.5 (builder.rs) — enhance `BuildStats` and two-phase build.

### A4: Ensure Two-Phase Build with Duplicate Edge Prevention (ENHANCE - from S4)

**Current plan gap:** The builder plan doesn't explicitly discuss the two-phase build pattern or bidirectional edge deduplication.

**Recommendation:** The `GraphBuilder::build()` should:
1. Phase 1: Create all nodes (extracting from all content files)
2. Phase 2: Create all edges (with all nodes available for reference resolution)
3. For `RelatesTo` and other bidirectional relationships, check for existing edges before creating duplicates

**Where:** Milestone 4.5 (builder.rs).

### A5: Add Depth-Capped BFS to Neighborhood Algorithm (ENHANCE - from S5)

**Current plan gap:** The neighborhood algorithm in doc 0020 is well-designed but doesn't specify a depth cap.

**Recommendation:** Add a `max_depth` constant (default 10) to prevent runaway BFS on highly connected graphs. Taproot uses 5; fabryk should make it configurable but with a sensible default.

**Where:** Milestone 4.3 (algorithms.rs) — the `neighborhood()` function.

### A6: Add QueriesAbout to Relationship Enum (ENHANCE - from S2)

**Current plan gap:** The `Relationship` enum has 7 variants + `Custom(String)` but no `QueriesAbout`.

**Recommendation:** This can remain as `Custom("queries_about".to_string())` since it's a user-model concern, not a universal knowledge-domain relationship. However, if the user model is integrated per A2, it may warrant a first-class variant. Defer to `Custom(String)` initially.

**Where:** No change needed — `Custom(String)` already handles this.

### A7: Add Typed Edge Traversal Helpers (NEW - from S5)

**Current plan gap:** The algorithms module has generic `get_related()` but no typed convenience methods.

**Recommendation:** Add convenience methods on `GraphData`:

```rust
impl GraphData {
    pub fn prerequisites(&self, id: &str) -> Vec<&Node> { ... }
    pub fn dependents(&self, id: &str) -> Vec<&Node> { ... }
    pub fn related_by(&self, id: &str, rel: &Relationship) -> Vec<&Node> { ... }
}
```

**Where:** Milestone 4.3 (algorithms.rs) or as convenience methods on `GraphData` in 4.1.

---

## Summary: What Phase 4 Already Gets Right

The Phase 4 plans are architecturally superior to Taproot in several important ways:
- **Trait abstraction** (`GraphExtractor`) enables domain-agnostic reuse
- **Edge weights** enable nuanced pathfinding
- **Edge provenance** (`EdgeOrigin`) enables debugging and validation
- **Persistence** (JSON + rkyv) avoids costly rebuilds
- **Modular architecture** (8 modules vs. 1 file) scales better
- **Relationship extensibility** (`Custom(String)`) future-proofs the enum
- **Comprehensive validation** (cycles, orphans, self-loops, duplicates)

## Summary: What We Should Adopt from Taproot

| Adaptation | Priority | Phase 4 Milestone |
|-----------|----------|-------------------|
| A1: Runtime mutation API | HIGH | 4.1 (types) |
| A2: NodeType enum | HIGH | 4.1 (types) |
| A3: Dangling ref handling | MEDIUM | 4.5 (builder) |
| A4: Two-phase build + dedup | MEDIUM | 4.5 (builder) |
| A5: Depth-capped BFS | LOW | 4.3 (algorithms) |
| A7: Typed traversal helpers | LOW | 4.1 or 4.3 |

The two HIGH priority items (A1, A2) represent genuinely new capabilities that the Phase 4 plans missed. They should be incorporated into milestone 4.1 design docs before implementation begins.

---

## Verification

After incorporating these adaptations:
1. Update doc 0018 (milestone 4.1) to include `NodeType` enum and `GraphData` mutation methods
2. Update doc 0022 (milestone 4.5) to specify two-phase build with dangling ref handling
3. Ensure milestone 4.3 neighborhood algorithm includes configurable depth cap
4. Verify that all Taproot test scenarios have analogous tests in the fabryk-graph test suite
