---
number: 19
title: "Theorem provers for Rust-based MCP integration"
author: "Mario Carneiro"
component: All
tags: [change-me]
created: 2026-03-19
updated: 2026-03-19
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# Theorem provers for Rust-based MCP integration

**Lean 4 is the clear first-choice target for a Rust MCP server that gives Claude access to formal verification.** It leads on every dimension that matters: AI tooling maturity, community momentum, API quality, and existing MCP precedent. Isabelle and Coq/Rocq are strong secondary targets with viable Rust integration paths, while Metamath Zero offers a unique native-Rust option for proof checking. No Rust-based MCP server for any theorem prover exists today — this is an open gap.

The interactive theorem prover landscape has shifted dramatically since 2023. Lean 4 has emerged as the dominant platform for AI-assisted proving, Coq completed its rename to Rocq with version 9.0, and a new generation of AI systems — AlphaProof, Aristotle, DeepSeek-Prover-V2 — has pushed formal theorem proving to IMO gold-medal level. Every major prover now has some form of programmatic interface, but the quality and Rust-friendliness vary enormously.

---

## The ITP landscape in 2026: Lean pulls ahead

**Lean 4** is the fastest-growing proof assistant by a wide margin. The Lean FRO (Focused Research Organization) has secured over **$15 million** in funding, the VS Code extension has been installed **100,000+ times**, and Mathlib4 — at **1.9 million lines** of formalized mathematics with **500+ contributors** — is the largest coherent math library in any proof assistant. Lean won the ACM SIGPLAN Programming Languages Software Award (June 2025) and the Skolem Award (July 2025). Monthly releases continue at a rapid cadence, with Lean 4.29.0 in development as of March 2026. The system is self-hosted (written in Lean and C++), uses dependent type theory (a CIC variant), and has Lake as its build system.

**Rocq** (formerly Coq) completed its rename with the release of Rocq 9.0 on March 12, 2025. The ecosystem remains large and mature — CompCert, Mathematical Components, and the Verified Software Toolchain all continue as flagship projects. The community is stable but not growing the way Lean's is. Rocq is implemented in **OCaml**, uses the Calculus of Inductive Constructions, and distributes via opam. The rename itself consumed significant community energy but is now settled.

**Isabelle** remains the workhorse of systems verification. Isabelle2025 shipped in March 2025 with scaling improvements up to 64GB memory. The Archive of Formal Proofs contains **935 entries**. Implemented in **Standard ML and Scala**, Isabelle's chief strength is Sledgehammer — its integration with external ATPs (Vampire, E, Z3). The seL4 verified microkernel, with 200,000+ lines of proof, remains its most famous application.

**Agda** occupies a stable niche in HoTT and programming language theory. Version 2.7.0.1 (May 2025) made Agda a self-contained single binary. Cubical Agda remains the primary implementation of cubical type theory. **F***continues active development at Microsoft Research, dominating in cryptographic verification — HACL* (used in Firefox, Linux kernel, Python) and Project Everest represent its crown jewels. **HOL4** and **HOL Light** are stable but lack modern programmatic interfaces. **Mizar** has a slowly declining community. **ACL2** maintains a steady industrial niche in hardware verification at AMD, Intel, and ARM.

**Metamath Zero**, while small in community, stands out architecturally: it is **written entirely in Rust** by Mario Carneiro, verifies the entire set.mm library (34,000 proofs) in under **200 milliseconds**, and is designed for formally verified verification — verifying the verifier down to the binary.

---

## API and integration capabilities differ dramatically

The provers span a wide spectrum from excellent programmatic APIs to bare REPL-only interfaces. For a Rust-based MCP server, the integration mechanism is the critical architectural constraint.

**Lean 4** has the most mature integration story. Its built-in LSP server (`lean --server`) speaks **JSON-RPC 2.0** over stdin/stdout with extensible custom RPC methods. Beyond LSP, three purpose-built machine interfaces exist: **Pantograph** (TACAS 2025) provides a machine-to-machine proof interaction layer via REPL, Python API, and C FFI; the community **REPL** project enables stateful tactic-by-tactic interaction; and the **LeanExplore** semantic search tool adds library discovery. Lean's C FFI is well-defined through `@[extern]` and `@[export]` attributes with a documented (though not yet stable) ABI. The `lean4_sys` crate on crates.io provides low-level Rust bindings to Lean's runtime via `lean.h`. Process-based LSP communication is the recommended path for MCP — it's the same mechanism used by the VS Code extension.

**Coq/Rocq** has converged on **rocq-lsp** (formerly coq-lsp) as its primary programmatic interface. The critical feature for MCP integration is the **Pétanque protocol** — a purpose-built extension for AI/tool interaction that provides `petanque/start` (start a proof), `petanque/run` (execute a tactic, returning a new state ID), and `petanque/goals` (retrieve current goals). This explicit state-ID model is arguably the **cleanest API for step-by-step proving** among all provers. Pétanque supports both stdio and TCP modes. No Rust crate exists for Coq, but the JSON-RPC protocol is straightforward to implement.

**Isabelle** has a well-documented TCP server (`isabelle server`) with JSON-based messaging and password authentication. Commands include `session_build`, `session_start`, `use_theories`, and `purge_theories`. The **`isabelle-client` crate on crates.io** is a full async Rust client (tokio-based) providing typed wrappers for every server command — this is the only production-quality Rust crate for any mainstream proof assistant. The main drawback is JVM startup time (**10–30 seconds** for initial session), though the server persists across queries.

**Metamath Zero** needs no process spawning at all. The `mm0-rs` toolchain is a native Rust library. The `mm0b_parser` crate on crates.io parses MM0's binary proof format. Verification is trivially embeddable — just a Rust dependency, with sub-millisecond verification latency.

**F\*** has a custom JSON-based IDE protocol (`fstar.exe --ide`) that is functional but non-standard — not true LSP, though an LSP mode is in development. **HOL4** and **HOL Light** have no server mode, no LSP, and no JSON interfaces — they operate exclusively as ML REPL sessions. **Agda** has a young external LSP server (`agda-language-server`) but its traditional interaction protocol produces Emacs Lisp S-expressions, not JSON.

---

## AI integration has concentrated heavily around Lean

The AI theorem-proving ecosystem has exploded since 2023, with **Lean 4 attracting roughly 70% of all active projects**. This concentration creates a self-reinforcing advantage for anyone building AI-prover tooling.

The most important infrastructure project is **Pantograph** (TACAS 2025), which has become the standard machine interface for Lean, used by Google DeepMind's AlphaProof, Harmonic's Aristotle, and LeanDojo-v2. **LeanDojo** (~770 GitHub stars) provides the most widely-used open-source toolkit for training ML models on Lean proofs, with Benchmark 4 containing **122,517 theorems** and **259,580 tactics** from Mathlib4. **Lean Copilot** is the most production-ready in-editor AI tool, automating **74.2%** of proof steps on the Mathematics in Lean textbook. DeepSeek-Prover-V2 achieves **88.9%** on MiniF2F-test. Harmonic's Aristotle won IMO 2025 gold (5/6 problems). AlphaProof earned IMO 2024 silver, published in Nature.

For Isabelle, **PISA** (Portal to ISAbelle) remains the foundational infrastructure, with Thor, Baldur, and Magnushammer building on it. The recent **IsaMini/MiniLang** (2025) achieved 79.2% on the PISA benchmark with a minimalist 10-operation proof language designed for LLMs.

For Coq/Rocq, **Tactician** provides the deepest integration — an OCaml plugin with Graph2Tac neural model that solves **26%** of theorems fully automatically across 120 Coq packages. **CoqPilot** (JetBrains Research) uses coq-lsp for LLM-powered proof generation. The **RocqStar** project (2025) explicitly uses MCP for agentic Coq proving, achieving a 60% theorem proving rate.

---

## Rust integration paths: four viable options

For building a Rust MCP server, the integration approaches sort into four tiers:

**Tier 1 — Native Rust (Metamath Zero):** The `mm0-rs` toolchain is pure Rust. Add it as a cargo dependency, call verification functions directly. Zero process overhead, sub-millisecond latency. The limitation is scope: MM0 is excellent for proof checking but lacks the rich tactic language and mathematical library of Lean or Coq. It excels as a lightweight verification backend, not an interactive proving environment.

**Tier 2 — Existing Rust crate (Isabelle):** The `isabelle-client` crate provides async TCP client functionality out of the box: `run_server()`, `session_build()`, `session_start()`, `use_theories()`. Integration requires roughly a day of wiring. The JVM startup penalty is real but manageable with a persistent server. Isabelle's `use_theories` command operates at file/theory granularity — less fine-grained than Lean's tactic-level interaction, which limits interactive proving workflows.

**Tier 3 — JSON-RPC process communication (Lean 4, Coq/Rocq):** Spawn the prover's LSP server as a child process, communicate via JSON-RPC over stdin/stdout. For Lean, this means `lean --server` or the community REPL (5× faster for batch operations). For Coq, `rocq-lsp` with the Pétanque protocol. Both require implementing a JSON-RPC client in Rust (straightforward with `serde_json` and `tokio`). The `lean4_sys` crate exists for low-level FFI but the ABI is unstable — process communication is more reliable.

**Tier 4 — Difficult or impractical (F\*, HOL4, HOL Light, Agda):** These require either OCaml runtime embedding, SML REPL parsing, or Haskell interop. None have Rust crates. Integration would mean wrapping a process and parsing semi-structured text output. Not recommended unless there's a specific use case.

---

## MCP servers exist but none in Rust

Five MCP servers for theorem provers have been built, all in Python or Node.js:

**lean-lsp-mcp** (Python) is the most mature, exposing 12 tools including `lean_diagnostics`, `lean_goal`, `lean_sorry_goals`, `lean_multi_attempt`, and `lean_local_search`. It bridges MCP to Lean's LSP via the `leanclient` Python library and supports a REPL mode for faster execution. It has been cited in multiple NeurIPS papers and works with Claude Code, Cursor, and VS Code Agent Mode.

**LLM4Rocq/rocq-mcp** (Python) wraps Coq's Pétanque protocol, exposing `rocq_start_proof`, `rocq_run_tactic`, `rocq_get_goals`, and `rocq_get_premises`. **mcp-coq-lsp** (Node.js) provides a lighter alternative via coq-lsp's JSON-RPC. Two simpler projects exist for Coq (angrysky56/mcp-rocq) and first-order logic (mcp-logic).

**No Rust MCP server exists for any theorem prover.** This represents both a gap and a clear opportunity. The existing Python implementations validate the tool designs and protocol patterns; a Rust implementation could offer better performance, type safety, and deployment characteristics.

---

## Recommended architecture and comparison

The optimal architecture is a **trait-based Rust MCP server** with pluggable prover backends. Define a `ProverBackend` trait with methods like `check_file()`, `start_proof()`, `run_tactic()`, `get_goals()`, and `search_library()`. Each prover backend implements this trait using its native protocol. Coq's Pétanque state-ID model is the cleanest reference design for the trait interface.

For MCP tool design, the existing servers converge on **9 essential tools**: file checking with diagnostics, proof session start, tactic execution, goal retrieval, library search, hover/type information, multi-tactic attempts, premise selection, and sorry/hole enumeration.

**Priority implementation order:** Start with Lean 4 (largest community, most AI research, best documentation), then add MM0 (trivial Rust integration, ultrafast verification), then Isabelle (existing Rust crate, systems verification niche), then Coq/Rocq (clean Pétanque API, strong research backing).

| Criterion | Lean 4 | Coq/Rocq | Isabelle | MM0 | F* | HOL4 | Agda |
|---|---|---|---|---|---|---|---|
| **Community momentum** | Strongly growing | Stable | Stable | Small/niche | Growing (niche) | Stable | Stable/niche |
| **AI ecosystem richness** | Excellent (15+ projects) | Good (Tactician, CoqPilot) | Good (PISA, Thor, Baldur) | Minimal | Limited | Limited | None |
| **Programmatic API** | Built-in LSP + Pantograph + REPL | rocq-lsp + Pétanque | isabelle server (TCP/JSON) | Rust library | Custom JSON IDE | SML REPL only | Emacs protocol / young LSP |
| **Rust crate available** | `lean4_sys` (low-level) | None | `isabelle-client` (async) | `mm0b_parser`, `mm0-rs` | None | None | None |
| **Integration method** | JSON-RPC process | JSON-RPC + Pétanque | TCP client (existing crate) | Direct library link | Process (custom JSON) | Process (text REPL) | Process (LSP) |
| **Integration complexity** | **Medium** — implement JSON-RPC client | **Medium** — implement Pétanque client | **Low** — use existing crate | **Very low** — cargo dependency | **High** — custom protocol, niche docs | **Very high** — parse SML REPL | **High** — immature LSP |
| **Interactive proving** | Excellent (tactic-level) | Excellent (Pétanque state IDs) | Limited (theory-level) | None (whole-proof only) | Good (tactic-level) | Good (ML-level) | Good (interaction commands) |
| **Startup latency** | ~2–5s | ~1–3s | ~10–30s (JVM) | <1ms | ~2–5s | ~3–5s | ~3–5s |
| **Math library size** | 1.9M lines (Mathlib4) | Large (mathcomp, etc.) | 935 AFP entries | 34K proofs (set.mm) | ~940K LOC | CakeML ecosystem | agda-stdlib + 1Lab |
| **Existing MCP server** | Yes (Python) | Yes (Python, Node.js) | No | No | No | No | No |
| **Overall recommendation** | **1st priority** | **4th priority** | **3rd priority** | **2nd priority** | Low priority | Not recommended | Not recommended |

The strongest practical architecture combines **Lean 4 for interactive theorem proving** (where the AI constructs proofs step-by-step) with **MM0 for fast proof verification** (where complete proofs are checked in bulk). Lean gives Claude the rich tactic language and vast mathematical library needed for creative proving; MM0 gives the microsecond-scale verification needed for rapid feedback during search. Isabelle adds access to the systems verification world (seL4, CakeML); Coq/Rocq rounds out coverage of the formal methods research community.
