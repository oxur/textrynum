# Fabryk Phase 4 — Graph

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 137-167. Original bytes are recoverable from import commit 3a39aea. -->

## Phase 4: fabryk-graph (3 weeks)

**Goal:** Extract the graph database with the `GraphExtractor` trait abstraction.
This is the highest-risk, highest-value phase.

**Crate:** `fabryk-graph`
**Dependency level:** 2 (depends on `fabryk-core`, `fabryk-content`)
**Risk:** High

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 4.1 | Graph types & Relationship enum | Extract `graph/types.rs` to `fabryk-graph`. Implement `Relationship` enum with `Custom(String)` variant (Amendment §2b). Types: `Node`, `Edge`, `GraphData`, `EdgeOrigin`. Add `Relationship::default_weight()` | Types compile, Relationship enum complete | §4.4, Amend §2b |
| 4.2 | GraphExtractor trait | Define `GraphExtractor` trait in `fabryk-graph::extractor`. Associated types: `NodeData`, `EdgeData`. Methods: `extract_node()`, `extract_edges()`, `to_graph_node()`, `to_graph_edges()` | Trait defined, mock extractor compiles | §3, §9 |
| 4.3 | Graph algorithms | Extract `graph/algorithms.rs` to `fabryk-graph`. Functions: `neighborhood()`, `shortest_path()`, `prerequisites_sorted()`, centrality. Already fully generic | All algorithm tests pass | §4.4 |
| 4.4 | Graph persistence | Extract `graph/persistence.rs` to `fabryk-graph`. Functions: `save_graph()`, `load_graph()`, `to_petgraph()`. Feature-gate rkyv cache behind `rkyv-cache` feature. Already fully generic | Persistence tests pass with feature flag | §4.4 |
| 4.5 | Graph builder | Extract `graph/builder.rs` to `fabryk-graph`. Refactor to `GraphBuilder<E: GraphExtractor>`. Add `with_manual_edges()` support (Amendment §2f-iii). Builder uses extractor trait instead of `parse_concept_card()` | Builder compiles with generic extractor | §4.4, Amend §2f-iii |
| 4.6 | Query, stats, validation | Extract `graph/query.rs`, `graph/stats.rs`, `graph/validation.rs`, `graph/loader.rs` to `fabryk-graph`. All generic or lightly parameterised | All supporting module tests pass | §4.4 |
| 4.7 | MusicTheoryExtractor | Create `music_theory_extractor.rs` in ai-music-theory. Implement `GraphExtractor` for music theory. Move `ConceptCard`, `RelatedConcepts`, and all parsing logic from `graph/parser.rs` here | Extractor compiles, matches current parser output | §3, §9.1 |
| 4.8 | Graph integration | Wire `MusicTheoryExtractor` into ai-music-theory's graph build pipeline. Remove `graph/parser.rs` from music-theory. Run `graph build --dry-run` and verify identical node/edge counts. Run `graph stats` and compare | `cargo test --all-features`, graph output identical | §7 Phase 5 |

**Phase 4 exit criteria:**

- `fabryk-graph` compiles and all tests pass
- `GraphExtractor` trait is defined and documented
- `MusicTheoryExtractor` produces identical graph output to pre-extraction
- Graph build: same node count, same edge count, same statistics
- All 15 graph MCP tools work identically
- `cargo test --all-features` passes in both repos

---


## Reconstructed slice locations

- [CC Prompt: Fabryk 4.1 — Graph Types & Relationship Enum](slice01-graph-types-relationship-enum/slice-plan.md)
- [CC Prompt: Fabryk 4.2 — GraphExtractor Trait](slice02-graphextractor-trait/slice-plan.md)
- [CC Prompt: Fabryk 4.3 — Graph Algorithms](slice03-graph-algorithms/slice-plan.md)
- [CC Prompt: Fabryk 4.4 — Graph Persistence](slice04-graph-persistence/slice-plan.md)
- [CC Prompt: Fabryk 4.5 — Graph Builder](slice05-graph-builder/slice-plan.md)
- [CC Prompt: Fabryk 4.6 — Query, Stats, Validation](slice06-query-stats-validation/slice-plan.md)
- [CC Prompt: Fabryk 4.7 — MusicTheoryExtractor](slice07-musictheoryextractor/slice-plan.md)
- [CC Prompt: Fabryk 4.8 — Graph Integration](slice08-graph-integration/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
