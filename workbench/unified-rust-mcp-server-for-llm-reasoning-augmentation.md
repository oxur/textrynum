# Unified Rust MCP server for LLM reasoning augmentation

A trait-unified reasoning architecture can emerge from these 12 projects by recognizing that **every project implements one of three core loops — verify, retrieve, or abstract — and the differences collapse into a single parameterized state machine**. The central insight: MCP-Solver's per-edit validation, Lean Copilot's tactic-typecheck-backtrack cycle, Logic-LM's self-refinement, IRCoT's interleaved retrieval, and Buffer of Thoughts' template instantiation are all instances of a `propose → evaluate → revise` loop with different evaluators plugged in. A Rust MCP server that exposes this loop as a generic trait, backed by petagraph for state persistence, tantivy for retrieval, lancedb for embeddings, and fabryk-graph for knowledge, can subsume all 12 systems into roughly **20 MCP tools across 4 tool groups**.

---

## The three loops every project implements

All 12 projects decompose into three fundamental architectural patterns, which in Rust map to three core traits:

**Loop 1 — Formal verification (MCP-Solver, Logic-LM, Lean Copilot).** The LLM proposes a symbolic artifact (constraint item, FOL formula, Lean tactic), a deterministic checker validates it, and structured feedback drives revision. MCP-Solver maintains an ordered `Vec<Item>` with per-edit AST validation across 5 solver backends (MiniZinc, PySAT, MaxSAT, Z3, Clingo). Logic-LM supports 4 formalisms (LP/Pyke, FOL/Prover9, CSP/python-constraint, SAT/Z3) in a batch pipeline with max 3 refinement rounds. Lean Copilot closes this loop natively inside Lean 4 via C++ FFI, where `suggest_tactics` generates candidates and Lean's kernel type-checks them, with `search_proof` running aesop's best-first search over the resulting proof tree. **The unifying abstraction**: a `Verifier` trait with `fn validate(&self, artifact: &str, context: &ModelState) -> VerificationResult` that returns either `Ok(feedback)` or `Err(structured_error)`.

**Loop 2 — Retrieval-grounded reasoning (IRCoT, DSPy, Buffer of Thoughts, Graphiti).** The LLM generates a reasoning step, the system retrieves relevant context, and the enriched context feeds the next step. IRCoT interleaves at sentence-level granularity: each CoT sentence becomes a BM25 query, retrieved paragraphs accumulate (capped at 15), and the process terminates on an "answer is" heuristic — achieving **21-point retrieval recall improvement** over one-shot retrieval. DSPy wraps this in declarative `Signature`/`Module`/`Optimizer` abstractions, where `BootstrapFewShot` generates traces, filters by metric, and assigns them as demonstrations. Buffer of Thoughts adds a persistent template layer: a meta-buffer of generalized reasoning strategies indexed by embedding similarity via LightRAG, achieving **82.4% on Game of 24** (vs 74% for Tree of Thoughts) at **12% of the cost**. **The unifying abstraction**: a `Retriever` trait with `fn retrieve(&self, query: &str, k: usize) -> Vec<RetrievalResult>` that dispatches to tantivy (BM25), lancedb (dense), or petagraph (graph traversal).

**Loop 3 — Structured metacognition (Sequential Thinking, Think Strategies, Better Thinking, YARN).** The LLM externalizes its reasoning as typed state that the server tracks. Sequential Thinking's single `sequentialthinking` tool maintains an append-only `Vec<ThoughtData>` with revision/branching metadata but **no evaluation** — it is a passive recorder in ~200 lines of TypeScript. Think Strategies extends this to 10 selectable strategies (CoT, ReAct, ToT, self-ask, etc.) driven by a `strategy-stages-mapping.json` flow engine, plus session persistence and external tool orchestration. Better Thinking adds `confidence_score: f64` (0.0–1.0) and `knowledge_assessment: Vec<(String, KnowledgeStatus)>` per step, inspired by Anthropic's circuits research. YARN contributes a **4-level abstraction hierarchy** (conceptual → evaluative → narrative arc → stage) that enables far-analogy detection where raw embeddings fail. **The unifying abstraction**: a `ReasoningSession` struct with typed steps, strategy selection, confidence tracking, and tree-structured branching (fixing the flat-array limitation shared by all three TypeScript implementations).

---

## Concrete data model for the unified server

The extracted data models converge on a surprisingly compact core. MCP-Solver's indexed item list, Sequential Thinking's thought history, Graphiti's bi-temporal edges, and Lean Copilot's proof goals all share a common shape: **a timestamped, typed node in a directed graph with metadata**.

```rust
/// Core reasoning step — generalizes ThoughtData, Item, Tactic, CoT sentence
struct ReasoningNode {
    id: NodeId,
    session_id: SessionId,
    step_number: u32,
    node_type: NodeType,  // Thought | FormalArtifact | RetrievalResult | Verification
    content: String,
    confidence: Option<f64>,           // Better Thinking pattern
    knowledge_status: Vec<(String, KnowledgeStatus)>,  // per-entity assessment
    strategy: Option<StrategyId>,      // Think Strategies pattern
    verification: Option<VerificationResult>,  // MCP-Solver/Lean pattern
    // Bi-temporal model from Graphiti
    created_at: DateTime,   // transaction time
    valid_at: DateTime,     // event time  
    invalid_at: Option<DateTime>,
}

/// Edges: revision, branching, derivation, retrieval-grounding
enum ReasoningEdge {
    Derives { from: NodeId, to: NodeId },          // sequential steps
    Revises { from: NodeId, target: NodeId },       // Better Thinking revision
    BranchesFrom { from: NodeId, branch_id: String }, // Sequential Thinking branch
    GroundedBy { node: NodeId, source: RetrievalSource }, // IRCoT grounding
    Verifies { artifact: NodeId, result: NodeId },  // MCP-Solver validation
}

/// Strategy enum — from Think Strategies, extensible
enum ReasoningStrategy {
    Linear, ChainOfThought, ReAct, SelfAsk,
    SelfConsistency, StepBack, TreeOfThoughts,
    FormalVerification { backend: SolverBackend },
    InterleavedRetrieval { max_steps: u32 },
    TemplateInstantiation { template_id: String },
}

/// Solver backend trait — abstracts MCP-Solver + Logic-LM + Lean Copilot
trait SolverBackend: Send + Sync {
    fn validate(&self, item: &str, context: &[String]) -> Result<ValidationOk, ValidationErr>;
    fn solve(&self, model: &[String], timeout: Duration) -> SolveResult;
    fn formalism(&self) -> Formalism; // FOL, CSP, SAT, LP, Lean4, Z3Smt
}
```

This model stores naturally in petagraph (nodes and edges), with tantivy indexing the `content` fields and lancedb storing embeddings for semantic retrieval. The bi-temporal timestamps from Graphiti enable "what was the reasoning state at time T?" queries essential for audit and debugging.

---

## MCP tool surface: 20 tools across 4 groups

Analyzing the tool signatures across all projects reveals heavy overlap. MCP-Solver's 6 tools, Codebase-Memory's 14, Graphiti's 5, Sequential Thinking's 1, and Think Strategies' 3 can be rationalized into **4 tool groups** that preserve the textrynum core while adding reasoning capabilities:

**Group 1 — Session management (4 tools).** `create_session(strategy, config) → session_id` starts a reasoning session with an optional strategy (defaulting to Linear). `add_step(session_id, thought, confidence?, knowledge_assessment?) → StepResult` is the primary tool — the equivalent of Sequential Thinking's single tool but with Better Thinking's metacognitive fields and MCP-Solver's validation feedback built in. `branch_session(session_id, from_step, branch_id)` creates an explicit branch point (fixing the passive-annotation pattern). `get_session(session_id, format?) → SessionState` returns the current reasoning tree with optional truncation (mirroring MCP-Solver's `get_model`).

**Group 2 — Formal verification (5 tools).** `add_artifact(session_id, code, formalism) → ValidationResult` combines MCP-Solver's `add_item` with Logic-LM's multi-formalism support — the `formalism` parameter selects the backend (Lean4, Z3, MiniZinc, SAT, ASP). Each addition triggers per-edit validation. `replace_artifact(session_id, index, code) → ValidationResult` and `delete_artifact(session_id, index)` mirror MCP-Solver exactly. `verify_model(session_id, timeout?) → SolveResult` runs the solver. `suggest_tactics(session_id, goal?) → Vec<(String, f64)>` wraps Lean Copilot's tactic suggestion with scored alternatives.

**Group 3 — Knowledge and retrieval (6 tools).** These wrap the existing textrynum surface. `search_text(query, k?) → Vec<Result>` wraps tantivy. `search_semantic(query, k?) → Vec<Result>` wraps lancedb. `search_graph(query, entity_types?, temporal_range?) → Vec<Result>` wraps petagraph/fabryk with Graphiti's temporal filtering. `add_knowledge(content, source_type, valid_at?) → EntityId` ingests into the knowledge graph with Graphiti's bi-temporal model. `trace_connections(entity_id, depth?, direction?) → Graph` performs graph traversal like Codebase-Memory's `trace_call_path`. `search_analogies(source, abstraction_level?) → Vec<AnalogicalMatch>` implements YARN's hierarchical abstraction matching.

**Group 4 — Meta-reasoning (5 tools).** `select_strategy(problem_description) → StrategyRecommendation` implements automatic strategy routing (Think Strategies' experimental `semantic-routing-config.json` made real). `retrieve_template(problem_description) → ThoughtTemplate` fetches from a Buffer of Thoughts-style meta-buffer. `evaluate_confidence(session_id) → ConfidenceReport` aggregates per-step confidence scores into a session-level assessment. `optimize_pipeline(session_id, metric) → OptimizationResult` applies DSPy-style trace analysis to identify which tool invocation patterns worked. `interleave_retrieval(session_id, query) → EnrichedStep` wraps IRCoT's retrieve-then-reason pattern as a single compound tool call.

---

## Where existing textrynum infrastructure maps

Duncan's existing stack covers roughly **60% of the required infrastructure**. Petagraph handles the graph storage for reasoning sessions, knowledge nodes, and code structure — directly replacing Neo4j (Graphiti), SQLite graphs (Codebase-Memory), and the implicit state stores in Sequential Thinking. Tantivy replaces Elasticsearch/BM25 used by IRCoT and provides the keyword search layer for Graphiti-style hybrid retrieval. Lancedb replaces the embedding indices used by Buffer of Thoughts (LightRAG), Graphiti (entity name embeddings), and YARN (structural mapping similarity). Fabryk-graph provides the typed knowledge graph layer that Graphiti implements from scratch atop Neo4j.

**What's missing and must be built:**
- **Solver process management**: spawning and sandboxing external solvers (Z3, Lean 4, MiniZinc) with timeout control — MCP-Solver uses Python multiprocessing; in Rust, `tokio::process::Command` with `tokio::time::timeout`
- **Per-edit validation pipeline**: AST parsing for constraint languages — tree-sitter-rust crates exist for 66+ languages (directly from Codebase-Memory's approach), plus MiniZinc/Z3/Lean grammar support
- **Lean 4 FFI bridge**: Lean Copilot uses C++ FFI via CTranslate2; a Rust equivalent needs either a C FFI shim to Lean's runtime or an HTTP API to a Lean Language Server Protocol instance
- **Bi-temporal query layer**: Graphiti's 4-timestamp model (created_at, expired_at, valid_at, invalid_at) over petagraph edges
- **Meta-buffer persistence**: thought template storage and embedding-indexed retrieval (straightforward over lancedb)
- **Strategy flow engine**: Think Strategies' `strategy-stages-mapping.json` equivalent as a Rust state machine

---

## Critical failure modes to design around

Every analyzed project reveals characteristic failure modes that a unified system must address:

**LLM formalization brittleness** is the dominant failure across formal verification tools. Logic-LM achieves only **32.6% executable rate** on AR-LSAT even with GPT-4. MCP-Solver's per-edit validation dramatically mitigates this — by catching errors at each incremental step rather than after batch generation, the LLM gets immediate feedback. **Design principle**: always validate incrementally, never batch-submit formal artifacts.

**Metacognitive features go unused** in practice. Sequential Thinking's GitHub Issue #2332 documents that LLMs (Gemini) almost never use `isRevision` or `branchFromThought` despite their availability. Better Thinking's confidence scores are self-reported with no ground truth. **Design principle**: the server should actively trigger metacognitive features (e.g., force a confidence check every N steps, auto-detect potential contradictions in the reasoning graph) rather than passively offering them.

**Solver timeout cascades** plague MCP-Solver (MCP clients default to 25s timeout, which must exceed solver timeout, which must exceed network latency). **Design principle**: implement a three-tier timeout hierarchy in Rust with `tokio::select!` — solver timeout < server timeout < client timeout, with early-return partial results.

**Template/strategy selection degrades at scale.** Buffer of Thoughts retrieves a single template per problem; Think Strategies offers 10 fixed strategies. Neither adapts based on intermediate results. **Design principle**: implement DSPy-style metric-driven strategy switching — if formal verification fails 3 times, auto-switch to a self-refinement strategy; if confidence drops below threshold, trigger interleaved retrieval.

**Knowledge graph ingestion cost** is Graphiti's Achilles heel: **600K+ tokens per conversation** for entity extraction, resolution, and contradiction detection. **Design principle**: make LLM-based entity extraction optional with a tree-sitter-based fast path (from Codebase-Memory) for structured inputs and an LLM path only for unstructured natural language.

---

## Phased implementation plan

**Phase 1 (weeks 1–3): Reasoning session core.** Implement the `ReasoningNode`/`ReasoningEdge` data model over petagraph. Build the session management tool group: `create_session`, `add_step`, `branch_session`, `get_session`. This is ~500 lines of Rust — Sequential Thinking is 200 lines of TypeScript and the data model is straightforward. Add confidence tracking and knowledge assessment fields from Better Thinking. Implement tree-structured branching (not flat append-only). Add strategy enum with Linear and ChainOfThought. Wire to existing textrynum MCP transport. **Deliverable**: a reasoning scratchpad that surpasses Sequential Thinking + Better Thinking combined.

**Phase 2 (weeks 4–7): Formal verification trait and Z3 backend.** Define the `SolverBackend` trait. Implement Z3 first (shared by MCP-Solver and Logic-LM, and it's Duncan's stated primary backend alongside Lean 4). Use the `z3` Rust crate (z3-sys FFI bindings). Build the per-edit validation pipeline: parse submitted artifacts through a Z3-specific AST check before solver invocation, returning structured errors on failure (mirroring MCP-Solver's 3-stage validation). Implement `add_artifact`, `replace_artifact`, `delete_artifact`, `verify_model`. Add process isolation via `tokio::process::Command` for solver execution with configurable timeouts. **Deliverable**: MCP-Solver-equivalent functionality for SMT solving, natively in Rust.

**Phase 3 (weeks 8–11): Lean 4 integration and proof search.** This is the highest-complexity phase. Lean Copilot's approach (C++ FFI with CTranslate2 inside Lean) cannot be directly replicated in Rust. Instead, implement a **Lean 4 Language Server bridge**: communicate with `lean --server` via JSON-RPC, submit tactic candidates, read back diagnostics. Implement `suggest_tactics` by maintaining a prompt template that serializes the current proof goal (extracted from Lean's diagnostic output) and queries an LLM. Implement proof search as best-first search over a `BTreeMap<Priority, Vec<GoalNode>>` — this replicates aesop's core algorithm without requiring aesop's Lean-native implementation. Each expansion applies a candidate tactic via the language server, and children are the resulting subgoals. **Deliverable**: Lean 4 tactic suggestion and multi-step proof search via MCP tools.

**Phase 4 (weeks 12–15): Knowledge retrieval integration.** Wire the knowledge/retrieval tool group to existing textrynum infrastructure. Implement Graphiti's bi-temporal model as 4 timestamp fields on petagraph edges with query filters (`valid_at <= T AND (invalid_at IS NULL OR invalid_at > T)`). Build hybrid search combining tantivy BM25 scores + lancedb cosine similarity + petagraph graph distance, with configurable weight blending. Implement IRCoT's interleaved pattern as a compound tool: `interleave_retrieval` calls `search_text`/`search_semantic` after each reasoning step, accumulates results, and returns enriched context. Cap accumulated results at configurable limit (IRCoT uses 15). **Deliverable**: retrieval-augmented reasoning over the existing knowledge base with temporal awareness.

**Phase 5 (weeks 16–19): Meta-reasoning and templates.** Implement Buffer of Thoughts' meta-buffer as a lancedb collection of thought templates with embedding-indexed retrieval. Build the `retrieve_template` and `optimize_pipeline` tools. Implement YARN's 4-level abstraction hierarchy as a configurable extraction pipeline — each level is an LLM prompt that maps `content → abstraction` at increasing generality. Store abstractions as separate lancedb collections keyed by level. Implement `search_analogies` as cosine similarity over the highest abstraction level (YARN shows this outperforms surface-level matching for far analogies). Add DSPy-inspired trace analysis: log all tool calls as `(tool, inputs, outputs, metric_score)` tuples, implement `optimize_pipeline` as a bootstrapping pass that identifies high-scoring traces and extracts them as few-shot examples for the system prompt. **Deliverable**: self-improving reasoning with reusable templates and cross-domain analogy detection.

**Phase 6 (weeks 20–22): Strategy routing and adaptive control.** Implement the strategy flow engine as a Rust state machine with per-strategy stage definitions (from Think Strategies' `strategy-stages-mapping.json`). Build `select_strategy` as an embedding-similarity match against strategy descriptions, with override capability. Add **adaptive strategy switching**: monitor per-step confidence scores and verification results; if confidence drops below threshold for 3 consecutive steps, auto-trigger interleaved retrieval; if formal verification fails 3 times on the same artifact, suggest switching formalism (e.g., FOL → CSP). Implement Think Strategies' `think-tools` pattern for cross-server MCP tool orchestration. **Deliverable**: an autonomous reasoning system that selects and adapts its approach based on intermediate results.

---

## What makes this design non-obvious

Three architectural decisions emerge from this analysis that contradict conventional approaches:

**Verification is the evaluator, not a separate step.** Most reasoning frameworks (Sequential Thinking, Think Strategies, Better Thinking) treat verification as optional or absent. MCP-Solver and Lean Copilot demonstrate that making verification the **mandatory evaluator in the propose-evaluate-revise loop** transforms LLM reasoning quality. In the unified architecture, every `add_step` call should optionally route through a verifier — not just for formal artifacts, but for factual claims (via retrieval), logical consistency (via graph analysis), and confidence calibration (via historical accuracy of similar confidence scores).

**The meta-buffer should store tool orchestration patterns, not just thought templates.** Buffer of Thoughts stores reasoning strategies as text. But in an MCP server, the reusable unit is a **tool invocation sequence**: "for problems of type X, call `add_artifact` with Z3 formalism, then `verify_model`, then if UNSAT call `search_semantic` for related theorems, then `replace_artifact` with the enriched constraint." These patterns are strictly more powerful than text templates because they carry typed tool signatures. Store them in lancedb indexed by problem-description embeddings.

**Bi-temporal state enables reasoning replay and counterfactual analysis.** Graphiti's 4-timestamp model is not just for knowledge graphs — applied to reasoning sessions, it enables questions like "what would the proof tree look like if we hadn't revised step 4?" or "show me the reasoning state as it was before the retrieval at step 7 changed the direction." This is invaluable for debugging complex multi-step reasoning and for the DSPy-style optimization loop, which needs to compare alternative reasoning traces.

---

## Conclusion

The 12 analyzed projects represent three generations of the same idea: making LLMs interact with formal systems through structured feedback loops. The first generation (Logic-LM, IRCoT) used batch pipelines. The second (MCP-Solver, Sequential Thinking) introduced stateful MCP tool interaction. The third (Buffer of Thoughts, DSPy) adds persistent learning across sessions. A unified Rust implementation can leapfrog all three by combining MCP-Solver's per-edit validation rigor, Lean Copilot's formal verification feedback loop, Graphiti's temporal knowledge model, IRCoT's interleaved retrieval, and Buffer of Thoughts' template reuse — all atop Duncan's existing petagraph/tantivy/lancedb/fabryk stack. The **20-tool surface across 4 groups** preserves textrynum's existing API shape while adding formal reasoning, metacognition, and adaptive strategy selection. The phased plan front-loads the highest-value components (reasoning sessions in phase 1, Z3 in phase 2, Lean 4 in phase 3) and defers the self-improving capabilities (templates, strategy routing) to later phases where the foundational infrastructure can support them.