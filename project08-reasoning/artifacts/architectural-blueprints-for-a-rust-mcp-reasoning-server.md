<!-- Historical source: workbench/architectural-blueprints-for-a-rust-mcp-reasoning-server.md, lines 1-196. Imported on 2026-11-21; completion claims and instructions are historical, not new acceptance or assignments. -->

# Architectural blueprints for a Rust MCP reasoning server

**The most replicable pattern across all 15 projects is the "validate-per-edit" loop**: every tool call returns structured feedback that guides the next LLM action, creating tight iterative refinement cycles. This core loop — expressed as MCP-Solver's per-edit validation, Logic-LM's solver-feedback self-refinement, DSPy's compile-time bootstrapping, and CRAG's three-way retrieval evaluation — maps directly to the existing `critique_loop` workflow in the ECL pipeline. Building the medys crate family should prioritize implementing this feedback loop as a generic trait, then specializing it for constraint solving, retrieval augmentation, and structured reasoning. Below is the full technical extraction from each project.

---

## 1. MCP-Solver: the gold standard for incremental constraint modeling

MCP-Solver (v3.4.0, Stefan Szeider at TU Wien) exposes exactly **6 tools** across 5 solver backends: `clear_model`, `add_item`, `delete_item`, `replace_item`, `get_model`, and `solve_model`. The internal model is an **ordered list of indexed code items** stored in server memory — each item is a constraint declaration, variable definition, or solve directive. Every `add_item`/`replace_item`/`delete_item` triggers **per-edit validation** before the model is mutated: for MiniZinc, this means full syntax parsing + type checking + semantic consistency via the MiniZinc analyzer; for Python-based backends (Z3, PySAT, MaxSAT), it means sandboxed subprocess execution with AST analysis, import policy enforcement, and structural verification of required solver invocations.

The `solve_model` tool accepts a `timeout` parameter (seconds) and returns `{status: SAT|UNSAT|TIMEOUT, solution: {...}, solve_time: float}`. Timeout enforcement varies: MiniZinc uses async worker threads with hard cancellation, PySAT/MaxSAT use CPU-time alarms in subprocesses, and Z3 uses runtime alarms in the main process. When UNSAT is returned, the built-in ReAct client's instruction prompt directs the LLM to check whether "all constraints in the encoding are justified by constraints in the problem statement." The multi-agent variant adds a separate **Review Agent** that independently categorizes results as correct/incorrect/unknown. Experimental data shows UNSAT problems trigger **4-5x more tool calls** than satisfiable ones (e.g., 24.2 `add_item` + 4.2 `solve_model` calls for university scheduling UNSAT vs. single solve calls for SAT instances).

**Rust replication target**: The 6-tool pattern maps cleanly to a `ConstraintModelManager` trait with `add_item(&mut self, index: usize, item: &str) -> ValidationResult`, `solve(&self, timeout: Duration) -> SolveResult`. The per-edit validation pattern should be a generic `ValidatingStore<T>` that wraps any mutable collection with a validation function called on each mutation. The 5 backends (MiniZinc, Z3, PySAT, MaxSAT, Clingo) would be separate feature-gated implementations behind a `SolverBackend` trait. Only one backend per session — this was a deliberate design choice to reduce LLM cognitive load.

---

## 2. Logic-LM: staged NL-to-formal translation with solver routing

Logic-LM (EMNLP 2023, UC Santa Barbara) implements a **three-stage pipeline**: Problem Formulator (LLM translates NL→symbolic via few-shot prompting), Symbolic Reasoner (deterministic solver execution), and Result Interpreter (symbolic→NL answer). The critical innovation is the `SYMBOLIC_FORMULA ::: NL_STATEMENT` grounding format in prompts, which forces the LLM to align each formal statement with its natural language source.

The framework targets **4 formal languages**: Logic Programming (Pyke), First-Order Logic (Prover9), Constraint Satisfaction (python-constraint), and SAT/SMT (Z3). Solver selection is **statically mapped per dataset** — not dynamically routed per problem. The self-refinement loop in `self_refinement.py` is triggered only by **execution errors** (syntax/parse failures), not semantic incorrectness. It feeds the erroneous formulation + solver error message back to the LLM with demonstrations of common error patterns, iterating up to 3 rounds. This improved executable rates by ~5% but accuracy sometimes slightly decreased — evidence that syntactic validity ≠ semantic correctness.

**Key limitation for Rust design**: The refinement loop only catches executability, not correctness. A medys implementation should layer semantic validation atop syntactic validation — e.g., using the petagraph knowledge graph to cross-check extracted constraints against known domain facts.

---

## 3. Lean Copilot: native tactic integration and proof search trees

Lean Copilot integrates **inside** the Lean 4 elaboration process as a native library, not through CLI or LSP. Three tactics are exposed: `suggest_tactics` (single-step suggestions), `search_proof` (multi-step Aesop-augmented search), and `select_premises` (retrieval-ranked lemmas). The state representation passed between suggestions is a **pretty-printed string** of the proof goal: `"n : ℕ\n⊢ gcd n n = n"`. This string is fed directly to a ByT5-small encoder-decoder model (ReProver) via C++ FFI bindings to CTranslate2.

The proof search tree is managed by **Aesop** (best-first search): goals are tree nodes, tactic applications create child sub-goals, and LLM-generated tactics are registered as dynamic "unsafe rules" with success probability weighting. The `search_proof` tactic augments Aesop's static rule set with goal-dependent LLM suggestions, making the rule set adaptive. For premise retrieval, the goal is encoded into a vector via `TextToVec.encode`, then multiplied against a pre-computed **premise embedding matrix** (167,779 premises from mathlib4).

**MCP wrapping strategy**: The most viable approach for an MCP server is **Pantograph** (used by LeanDojo-v2) — a machine-to-machine RPC interface for Lean 4 providing goal state observation and tactic execution. LeanDojo's older approach uses a modified Lean process communicating via stdin/stdout JSON. Either way, the MCP server would maintain a `ProofState` struct containing the pretty-printed goals and a tactic history, exposing tools like `run_tactic(state_id, tactic_str) -> TacticResult` and `suggest_tactics(state_id) -> Vec<(String, f32)>`.

---

## 4. Sequential Thinking MCP: append-only thought trees with revision references

The official Anthropic server exposes a single `sequentialthinking` tool with this exact schema: **required** fields are `thought: string`, `nextThoughtNeeded: boolean`, `thoughtNumber: integer`, `totalThoughts: integer`; **optional** fields are `isRevision: boolean`, `revisesThought: integer`, `branchFromThought: integer`, `branchId: string`, `needsMoreThoughts: boolean`. Notable: the Zod validator includes `coercedBoolean` preprocessors to handle Gemini CLI sending string "true"/"false".

The data model is deliberately simple — **no tree structure**. The `SequentialThinkingServer` class maintains `thoughtHistory: ThoughtData[]` (flat append-only array) and `branches: Record<string, ThoughtData[]>` (dictionary keyed by branch ID). **Revisions are appended, never replaced in-place** — the original thought persists, and the revision carries a reference to which thought it revises. Branches are one level deep with no native nested branching. Completion is signaled by `nextThoughtNeeded: false`; extension beyond `totalThoughts` uses `needsMoreThoughts: true`.

**Rust implementation**: This maps to `struct ThoughtStore { history: Vec<ThoughtData>, branches: HashMap<String, Vec<ThoughtData>> }` with an append-only mutation API. The revision model (append + reference) is architecturally similar to event sourcing — consider using the ECL pipeline's event model.

---

## 5. Think Strategies: data-driven reasoning strategy selection

Think Strategies extends Sequential Thinking with **10 named strategies** (Linear, Chain of Thought, ReAct, ReWOO, Scratchpad, Self-Ask, Self-Consistency, Step-Back, Tree of Thoughts, Trilemma) stored as **JSON data** in `strategy-stages-mapping.json` and `semantic-routing-config.json`. Each strategy defines a stage flow pattern: fixed-step (linear), cyclic (ReAct loops), or branching (Tree of Thoughts).

The architecture adds two tools beyond `think-strategies`: `think-tools` (action execution: connect-server, execute-actions, create-branch) and `think-session-manager` (list/get/resume sessions). Sessions auto-save as JSON to `~/Documents/thinking/` with full thought history and branches. A dual numbering system tracks both absolute position (A1, A2) and sequence-relative position (S1, S2) within each strategy phase.

**Replication insight**: The key extractable concept is **strategy-as-data** — reasoning strategies defined declaratively in configuration files rather than hard-coded. For medys, this means a `ReasoningStrategy` struct loaded from YAML/TOML with fields for `stages: Vec<Stage>`, `flow_type: Fixed|Cyclic|Branching`, and `transition_rules: Vec<Transition>`. The `DiscoverableRegistry` pattern in fabryk-mcp-core already supports this kind of dynamic capability registration.

---

## 6. Better Thinking MCP: confidence tracking and knowledge assessment

Better Thinking (199-bio) adds two structured fields beyond Sequential Thinking: `confidence_score: f64` (0.0–1.0 per step) and `knowledge_assessment: Vec<{entity: String, status: Known|Unknown|Uncertain}>`. The "concept activation" model is **philosophical, not computational** — it encourages LLMs to self-report intermediate concept activations, parallel hypothesis tracking, and reasoning faithfulness, inspired by Anthropic's Circuits research. There is **no server-side aggregation** of confidence scores across steps — tracking is left to the LLM's context.

**Extractable pattern**: The `knowledge_assessment` schema is directly useful for the concept card extraction pipeline. Each reasoning step can declare what entities it's uncertain about, enabling targeted retrieval from the petagraph knowledge graph or lancedb vector store.

---

## 7. Graphiti: bi-temporal knowledge graphs with ingestion-time entity resolution

Graphiti's data model carries **four timestamps** on every `EntityEdge`: `created_at` (system ingestion time), `expired_at` (system-level invalidation), `valid_at` (real-world start), `invalid_at` (real-world end). This enables point-in-time queries, out-of-order ingestion handling, and fact rewriting when contradictions are detected.

Entity resolution uses a **three-tier strategy at ingestion time**: (1) entropy-gated fuzzy matching on normalized names, (2) embedding cosine similarity against `name_embedding` vectors, (3) LLM verification only for ambiguous cases. At query time, **zero LLM calls** — only embeddings, BM25, and graph traversal. Edge contradiction detection works by: (a) retrieving existing edges between the same entity pair via hybrid search, (b) an LLM invalidation prompt classifying edges as duplicate/contradicted/new, (c) `resolve_edge_contradictions()` using `valid_at` timestamps to set `invalid_at`/`expired_at` on older edges.

The `GraphDriver` ABC abstracts over **Neo4j, FalkorDB, Kuzu, and Amazon Neptune**. Community detection uses the **Leiden algorithm** (improvement over Louvain) with incremental label propagation for new nodes. The MCP server exposes `add_episode`, `search_nodes`, `search_facts`, `get_entity_edge`, `get_episodes`, `delete_entity_edge`, `delete_episode`, `clear_graph`, and `get_status`.

**Rust mapping**: The bi-temporal model maps directly to petagraph edge properties. The three-tier resolution strategy could leverage fastembed for tier 2 (embedding similarity) and tantivy for tier 1 (fuzzy matching). The `EntityEdge` struct with four timestamps is straightforward: `struct TemporalEdge { created_at: DateTime<Utc>, expired_at: Option<DateTime<Utc>>, valid_at: Option<DateTime<Utc>>, invalid_at: Option<DateTime<Utc>>, fact: String, fact_embedding: Vec<f32> }`. The contradiction detection algorithm is well-specified enough to implement directly from the pseudocode.

---

## 8. Codebase-Memory MCP: tree-sitter knowledge graphs in a zero-dependency binary

Codebase-Memory (v0.5.0+, rewritten from Go to C) ships as a **single static binary** with vendored tree-sitter grammars for 66 languages. It indexes the Linux kernel (28M LOC) in ~3 minutes, producing **2.1M nodes and 4.9M edges** with sub-millisecond query latency.

The 14 MCP tools split into 4 indexing tools (`index_repository`, `list_projects`, `delete_project`, `index_status`) and 10 query tools, with `search_graph` being the most complex (regex name patterns, glob file patterns, relationship type filters, degree-based filtering, pagination). The knowledge graph uses **13 node labels** (Project, Package, Folder, File, Module, Class, Function, Method, Interface, Enum, Type, Route, Resource) and **18+ edge types** (CONTAINS_*, DEFINES, IMPORTS, CALLS, HTTP_CALLS, ASYNC_CALLS, IMPLEMENTS, TESTS, SIMILAR_TO, SEMANTICALLY_RELATED).

The multi-pass indexing pipeline runs: structure → definitions → calls → HTTP links → config → tests. Call resolution uses **fused Aho-Corasick multi-pattern matching** — all function name patterns loaded simultaneously for a single pass over the AST. The v0.6.0 additions include `SIMILAR_TO` edges via MinHash/Jaccard and `SEMANTICALLY_RELATED` edges using an 11-signal combined scoring system (TF-IDF, Reflective Random Indexing, API/Type/Decorator signatures, AST structural profiles, Halstead-lite metrics, and more).

**Louvain community detection** runs on CALLS edges to discover functional modules — clusters of functions that form cohesive units based on call patterns, which may not align with file/folder organization. The `get_architecture` tool returns these clusters alongside languages, packages, entry points, routes, hotspots, boundaries, and layers.

**Rust replication**: The textrynum monorepo already has tree-sitter available. The key extractable patterns are: (a) the multi-pass indexing pipeline as an ECL workflow, (b) the Aho-Corasick call resolution strategy, (c) the 13-label/18-edge graph schema stored in petagraph, and (d) the Louvain clustering for architectural analysis. The SQLite schema maps to petagraph node/edge properties.

---

## 9. YARN: four-level narrative abstraction for structural analogy

YARN (arXiv:2603.29997, March 2026) defines four abstraction levels for analogical reasoning: **Conceptual** (modifier-root semantic frames like "tasks-workload"), **Evaluative** (3×3 matrix of functional role × polarity: Struggle, Ease, Effort, Indifference, Gain, Loss + 3 neutrals), **Narrative Arc** (Background, Main Event, Challenge, Action, Conclusion), and **Stage** (hierarchical aggregation within arc stages, culminating in a single whole-story abstraction).

Structural mapping follows a three-step process: pair generation from unit phrases/abstractions, local mapping scoring via **cosine similarity over embeddings** with evaluative/arc labels as constraints, and global one-to-one mapping via a **greedy algorithm**. The critical finding: structural mapping without abstraction performs **below random** on far analogies — cosine similarity over raw text captures surface semantics and misses functional roles, causal structure, and contradiction.

**Extractable concept**: The four-level abstraction hierarchy is directly applicable to concept card extraction. Each concept card could carry evaluative and structural annotations that enable analogical retrieval from lancedb — finding structurally similar concepts across different domains rather than just semantically similar text.

---

## 10. DSPy: declarative LLM pipelines with compile-time optimization

DSPy's core abstractions are: **Signature** (declarative I/O contract, e.g., `"question -> answer"`), **Module** (composable building block with `forward()` method), **Predict** (base module that constructs prompts from signature + demos + inputs), and **Optimizer** (compile-time optimizer that automatically generates few-shot demonstrations). The key modules are `ChainOfThought` (prepends `reasoning: str` to output), `ReAct` (iterative thought→action→observation with tool functions), `ProgramOfThought` (generates executable Python), `BestOfN` (runs N times, picks best), and `Refine` (BestOfN + LLM feedback loop).

**"Bootstrapping demonstrations"** concretely means: (1) run a teacher program on training inputs at `temperature=1.0`, (2) score each output with a metric function, (3) accept passing outputs as synthetic few-shot examples, (4) inject these into each Predict module's prompt. The optimizers range from `LabeledFewShot` (random selection) to `MIPROv2` (joint instruction + demo optimization via Bayesian optimization with Optuna TPE sampler).

The Rust trait mapping is well-defined:

- `Signature` → derive macro on structs generating `input_fields()` / `output_fields()` metadata
- `Module` → trait with `fn forward(&self, input: Self::Input) -> Prediction` + `fn named_predictors(&self)` + `fn dump_state()`
- `Predict<S: Signature>` → struct holding `demos: Vec<Example>`, `config: LMConfig`, `lm: Option<Arc<dyn LM>>`
- `ChainOfThought` → newtype wrapping Predict that prepends a `reasoning` field via signature manipulation
- `Optimizer` → trait with `fn compile<M: Module>(&self, student: M, trainset: &[Example], metric: &dyn Fn) -> M`
- Settings → `thread_local!` + `Arc<RwLock<Config>>` for global/thread-local LM configuration

**Minimal viable subset** for medys: Signature + Predict + ChainOfThought + one optimizer (BootstrapFewShot). The assertion system was deprecated in DSPy 2.6 in favor of `BestOfN`/`Refine`, which are simpler to implement.

---

## 11. IRCoT: sentence-level interleaving of retrieval and reasoning

IRCoT (ACL 2023) implements **strict sentence-level alternation**: after each CoT sentence, the raw sentence text is used as a BM25 query to retrieve K additional paragraphs, which are added to a cumulative context set (capped at 15 paragraphs). The state machine is: `INIT(Q) → [REASON(generate 1 sentence) → CHECK(contains "answer is"? or step > 8?) → RETRIEVE(BM25 with last sentence)] → OUTPUT(collected paragraphs)`.

The key insight: no query reformulation module is needed — each CoT sentence naturally introduces new entities and facts that serve as effective lexical retrieval terms. This is far simpler than learned query rewriters and surprisingly effective for multi-hop reasoning.

**Rust implementation**: This maps to a simple state machine using tantivy for BM25 retrieval. The `InterleaveState` struct holds `collected_paragraphs: Vec<Paragraph>`, `cot_sentences: Vec<String>`, `step_count: usize`, and the loop alternates between LLM generation (constrained to first sentence only) and tantivy search.

---

## 12. Buffer of Thoughts: meta-buffer of reusable reasoning templates

Buffer of Thoughts (NeurIPS 2024 Spotlight) maintains a **meta-buffer** — a library of thought-templates indexed by embedding similarity. Each entry is `(template, description, category)` across 6 categories (Text Comprehension, Creative Language, Common Sense, Mathematical, Code, Scheduling). Templates come in three structural types: prompt-based (NL step-by-step), procedure-based (procedural instructions), and programming-based (Python code with functions to fill).

The four-stage pipeline: (1) **Problem Distillation** — LLM extracts key parameters, objectives, constraints into `x_d`; (2) **Template Retrieval** — embedding similarity `Sim(f(x_d), f(DT_i)) >= δ` (threshold 0.5–0.7); (3) **Instantiated Reasoning** — LLM adapts retrieved template to specific problem; (4) **Buffer Manager** — distills new templates from successful solutions and adds to meta-buffer only if sufficiently novel (max similarity < δ).

**Rust replication**: The meta-buffer is a lancedb collection with `description_embedding: Vec<f32>`, `template: String`, `category: Category`, `created_from: Vec<ProblemId>`. Template retrieval uses fastembed + lancedb vector search with a similarity threshold. The distillation process is an ECL pipeline step that extracts generalizable patterns from successful reasoning traces. This is perhaps the **highest-leverage pattern** for medys — a growing library of reusable reasoning strategies that improves with use.

---

## 13. Corrective RAG: three-way retrieval evaluation with lightweight classifier

CRAG uses a **fine-tuned T5-large model** (0.77B parameters) as a lightweight retrieval evaluator — significantly cheaper than an LLM call and **outperforming ChatGPT** at relevance assessment (84.3% vs. 64.7% accuracy). Each (question, document) pair receives a score from -1 to +1. Two thresholds determine the action:

- **Correct** (any score > τ_upper ≈ 0.5): refine retrieved documents via decompose→filter→recompose
- **Incorrect** (all scores < τ_lower ≈ -0.9): discard all documents, rewrite query via GPT-3.5, web search fallback
- **Ambiguous** (mixed): combine refined internal knowledge with web search results

The knowledge refinement pipeline segments documents into fine-grained "strips," re-evaluates each strip with the same T5 evaluator (threshold -0.5, keep top-5), then concatenates survivors.

**Rust replication**: The three-way evaluation rubric is the key pattern. For medys, a lightweight evaluator (potentially a small ONNX model via fastembed's runtime, or a heuristic combining BM25 score + embedding similarity from tantivy + lancedb) classifies retrieval quality before the LLM ever sees the results. The Correct/Ambiguous/Incorrect branching maps to a Rust enum: `enum RetrievalQuality { Correct(Vec<Document>), Ambiguous(Vec<Document>), Incorrect }` with different downstream processing for each variant.

---

## 14. SymbolicAI: Design by Contract for LLM outputs

SymbolicAI's `@contract` decorator enforces pre/post conditions on LLM outputs with **automatic LLM-based remediation**. The execution flow: return type validation → input collection → pre-condition check (with optional `pre_remedy` via SemanticValidationFunction) → intermediate processing → output generation + post-condition check (with optional `post_remedy` using error accumulation across retries). The `forward()` method **always executes** in a `finally` block regardless of contract outcome. Retry uses exponential backoff with jitter (`tries=3, delay=0.4, backoff=1.8, jitter=0.15`).

The `Symbol` class is the foundational type with **dual representation**: `.value` (Python value) and `.embedding` (vector). Mode switching between syntactic (conventional Python operators) and semantic (LLM-mediated understanding) is via `.sem`/`.syn` projections. Key semantic primitives include `.choice(cases)` (best semantic match), `.foreach(cond, apply)` (element-wise processing), `.cluster()` (DBSCAN semantic clustering), and `.similarity()` (embedding comparison).

Solver integration is via **LLM-generated code**: for Z3, the LLM generates Python code importing Z3, which is executed via `exec()`. WolframAlpha is accessed through the API as a "Symbolic Engine" distinct from the "Neuro-Symbolic Engine" (LLM).

**Rust trait design**: The DbC pattern maps to a `Contract<T>` trait with `fn pre_condition(&self, input: &Input) -> Result<(), ContractViolation>`, `fn post_condition(&self, output: &T) -> Result<(), ContractViolation>`, and a blanket implementation that wraps any `Module::forward()` with validation + retry logic. The `Symbol` dual-representation concept maps to `struct Symbol<T> { value: T, embedding: Option<Vec<f32>> }` with `Semantic` and `Syntactic` wrapper types providing different operator overloads.

---

## 15. Code analysis cluster: archmap, Semgrep MCP, and ast-grep MCP

**archmap** (Rust CLI with MCP) computes coupling (fan-in with configurable thresholds), god object detection (line count), dependency depth (max chain length), cohesion (0.0–1.0 scale), and circular dependency detection. Boundary violations are defined in `.archmap.toml` via **indicator patterns** — strings like `"sqlx::"`, `"reqwest::"` that flag operations outside designated architectural layers. Output formats include Markdown, JSON, XML, interactive HTML graphs (`--serve` with live-reload), and AI-optimized compact summaries with token budgets. The `archmap mcp` subcommand exposes all analysis via stdio transport.

**Semgrep MCP** (now in main `semgrep/semgrep` repo) exposes `security_check`, `semgrep_scan`, `semgrep_scan_with_custom_rule`, `get_abstract_syntax_tree`, `semgrep_findings`, and `supported_languages`. Custom rules use Semgrep's YAML DSL with metavariables (`$X`), composite operators (`pattern-either`, `pattern-not`), and language-specific semantic understanding. The `semgrep_rule_schema` tool dynamically fetches the latest JSON Schema for rule validation.

**ast-grep MCP** provides 4 tools: `dump_syntax_tree` (AST visualization), `test_match_code_rule` (iterative rule development), `find_code` (simple pattern search), and `find_code_by_rule` (complex YAML rule search). Patterns use source-code-like syntax with `$VARIABLE` metavariables for single nodes and `$$$` for variadic matching. YAML rules support relational operators (`has`, `inside`, `precedes`, `follows` with `stopBy: end`) and composite operators (`all`, `any`, `not`). The text output format reduces token usage by ~75% compared to JSON.

**Integration pattern for medys**: archmap's indicator-based boundary detection and ast-grep's structural pattern matching both produce structured findings that feed naturally into the concept card extraction pipeline. The archmap Rust crate could be directly integrated as a dependency, while ast-grep patterns could be stored in the meta-buffer (Buffer of Thoughts pattern) as reusable code analysis templates.

---

## Cross-cutting architectural patterns for the medys crate family

Seven patterns recur across these 15 projects and should form the medys foundation:

**Pattern 1 — Validate-per-mutation** (MCP-Solver, SymbolicAI contracts). Every state change triggers validation before commitment. In Rust: `trait ValidatingStore<T> { fn mutate(&mut self, op: Mutation) -> Result<T, ValidationError>; fn validate(&self, state: &T) -> Result<(), ValidationError>; }`.

**Pattern 2 — Three-way retrieval evaluation** (CRAG, IRCoT). Before the LLM sees retrieved context, classify retrieval quality as Correct/Ambiguous/Incorrect and branch processing accordingly. Combine tantivy BM25 scores + lancedb cosine similarity as a lightweight evaluator.

**Pattern 3 — Append-only thought history with revision references** (Sequential Thinking, Better Thinking, Think Strategies). Never mutate past thoughts — append revisions with back-references. This is event sourcing for reasoning traces, naturally stored in petagraph as a temporal chain.

**Pattern 4 — Strategy-as-data** (Think Strategies, Buffer of Thoughts, DSPy Signatures). Reasoning strategies defined declaratively in configuration, not code. The DiscoverableRegistry already supports this — extend it with strategy schemas loaded from TOML/YAML.

**Pattern 5 — Bi-temporal fact management** (Graphiti). Every fact carries both system time and real-world time, enabling out-of-order ingestion and point-in-time queries. Implement on petagraph edges with `created_at`, `expired_at`, `valid_at`, `invalid_at` fields.

**Pattern 6 — Staged NL↔formal translation** (Logic-LM, MCP-Solver, Lean Copilot). Natural language → formal representation → deterministic solver → result interpretation, with solver error messages feeding back into refinement. The `:::` grounding format from Logic-LM (aligning each formal statement with its NL source) is worth replicating.

**Pattern 7 — Template meta-buffer** (Buffer of Thoughts). A growing library of distilled reasoning templates, indexed by embedding similarity, that improves with use. Store in lancedb with fastembed vectors, distill new templates via the ECL pipeline's critique_loop when reasoning succeeds.

These seven patterns, implemented as composable traits in the medys crate family, would cover the full spectrum from constraint solving to retrieval augmentation to structured multi-step reasoning — all within the existing textrynum infrastructure of petagraph, tantivy, lancedb, fastembed, and the ECL pipeline engine.