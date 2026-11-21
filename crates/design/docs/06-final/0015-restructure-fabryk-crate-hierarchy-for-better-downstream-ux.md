---
number: 15
title: "Restructure Fabryk Crate Hierarchy for Better Downstream UX"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-03-09
updated: 2026-03-09
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# Restructure Fabryk Crate Hierarchy for Better Downstream UX

## Context

Downstream users currently face a fragmented dependency experience. The goal is two clean entry points:

- `fabryk` — core knowledge fabric (auth, content, fts, graph, vector, acl)
- `fabryk-mcp` — MCP tools facade (all MCP sub-crates)

Vendor-specific crates (fabryk-gcp, fabryk-auth-google) remain separate opt-in.

## Key Decisions

- **Create `fabryk-mcp-core`** to resolve circular dependency (mirrors fabryk/fabryk-core pattern)
- **Rename `fabryk-auth-mcp` → `fabryk-mcp-auth`** for consistent naming
- **fts/graph/vector are always-on** in the facade; backend features remain gated
- **fabryk-acl included** in facade; fabryk-redis excluded

## Target Dependency Graphs

```
fabryk (facade)
├── fabryk-core
├── fabryk-auth
├── fabryk-acl
├── fabryk-content
├── fabryk-fts
├── fabryk-graph
└── fabryk-vector

fabryk-mcp (facade)
├── fabryk-mcp-core (infrastructure, extracted from current fabryk-mcp)
├── fabryk-mcp-auth (renamed from fabryk-auth-mcp)
├── fabryk-mcp-content  ──▸ fabryk-mcp-core
├── fabryk-mcp-fts      ──▸ fabryk-mcp-core
├── fabryk-mcp-graph    ──▸ fabryk-mcp-core
└── fabryk-mcp-semantic ──▸ fabryk-mcp-core
```

## Implementation Phases

### Phase 1: Create `fabryk-mcp-core` (additive, nothing breaks)

1. Create `crates/fabryk-mcp-core/` directory
2. Copy all source files from `crates/fabryk-mcp/src/` → `crates/fabryk-mcp-core/src/`
3. Create `crates/fabryk-mcp-core/Cargo.toml` — identical to current `fabryk-mcp/Cargo.toml` but with `name = "fabryk-mcp-core"`
4. Add `"crates/fabryk-mcp-core"` to workspace members in root `Cargo.toml`
5. Verify: `cargo check -p fabryk-mcp-core`

### Phase 2: Migrate sub-crates to depend on `fabryk-mcp-core`

For each of `fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph`, `fabryk-mcp-semantic`:

1. In `Cargo.toml`: change `fabryk-mcp` dep → `fabryk-mcp-core`
2. In source files: change `use fabryk_mcp::` → `use fabryk_mcp_core::` and `crate::` references to `fabryk_mcp_core::`
3. Verify: `cargo check -p fabryk-mcp-content -p fabryk-mcp-fts -p fabryk-mcp-graph -p fabryk-mcp-semantic`

### Phase 3: Rename `fabryk-auth-mcp` → `fabryk-mcp-auth`

1. `mv crates/fabryk-auth-mcp crates/fabryk-mcp-auth`
2. Update `Cargo.toml` inside: `name = "fabryk-mcp-auth"`
3. Update workspace members in root `Cargo.toml`
4. Verify: `cargo check -p fabryk-mcp-auth`

### Phase 4: Convert `fabryk-mcp` to facade

1. Rewrite `crates/fabryk-mcp/Cargo.toml`:
    - Remove all infrastructure deps (rmcp, tokio, serde, etc.)
    - Add: `fabryk-mcp-core`, `fabryk-mcp-auth`, `fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph`, `fabryk-mcp-semantic`
    - Feature flags: `http` → forwards to `fabryk-mcp-core/http`, `fts-tantivy` → `fabryk-mcp-fts/fts-tantivy`, `graph-rkyv-cache` → `fabryk-mcp-graph/graph-rkyv-cache`

2. Rewrite `crates/fabryk-mcp/src/lib.rs`:

    ```rust
    pub use fabryk_mcp_core::*;  // backward compat — all infra symbols stay at fabryk_mcp::
    pub use fabryk_mcp_auth as auth;
    pub use fabryk_mcp_content as content;
    pub use fabryk_mcp_fts as fts;
    pub use fabryk_mcp_graph as graph;
    pub use fabryk_mcp_semantic as semantic;
    ```

15. Delete old source files from `crates/fabryk-mcp/src/` (discoverable.rs, error.rs, registry.rs, server.rs, etc.) — they now live in fabryk-mcp-core
4. Verify: `cargo check -p fabryk-mcp`

### Phase 5: Update `fabryk` facade

1. Update `crates/fabryk/Cargo.toml`:
    - Add `fabryk-auth` and `fabryk-acl` as required deps
    - Make `fabryk-fts`, `fabryk-graph`, `fabryk-vector` required (remove `optional = true`)
    - Remove all MCP and CLI deps
    - Features: keep `fts-tantivy`, `graph-rkyv-cache`, `vector-lancedb`, `vector-fastembed` as pass-throughs; add `full` = all backends enabled; remove `mcp`, `cli`, `fts`, `graph`, `vector` feature names

2. Update `crates/fabryk/src/lib.rs`:
    - Add `pub use fabryk_auth as auth;` and `pub use fabryk_acl as acl;`
    - Remove `#[cfg(feature = ...)]` guards on fts, graph, vector
    - Remove all MCP and CLI re-exports

3. Verify: `cargo check -p fabryk`

### Phase 6: Full verification

1. `cargo check --workspace`
2. `cargo test --workspace`
3. Update README files if needed

## Files Modified

| File | Action |
|------|--------|
| `Cargo.toml` (root) | Add `fabryk-mcp-core` member, rename `fabryk-auth-mcp` → `fabryk-mcp-auth` |
| `crates/fabryk-mcp-core/` (new) | All files copied from `crates/fabryk-mcp/src/` |
| `crates/fabryk-mcp-core/Cargo.toml` (new) | Based on current fabryk-mcp, name = fabryk-mcp-core |
| `crates/fabryk-mcp/Cargo.toml` | Rewrite as facade |
| `crates/fabryk-mcp/src/lib.rs` | Rewrite as re-export facade |
| `crates/fabryk-mcp/src/*.rs` | Delete (moved to fabryk-mcp-core) |
| `crates/fabryk-mcp-auth/Cargo.toml` | Rename package |
| `crates/fabryk-mcp-content/Cargo.toml` | dep fabryk-mcp → fabryk-mcp-core |
| `crates/fabryk-mcp-content/src/tools.rs` | use fabryk_mcp → fabryk_mcp_core |
| `crates/fabryk-mcp-fts/Cargo.toml` | dep fabryk-mcp → fabryk-mcp-core |
| `crates/fabryk-mcp-fts/src/tools.rs` | use fabryk_mcp → fabryk_mcp_core |
| `crates/fabryk-mcp-graph/Cargo.toml` | dep fabryk-mcp → fabryk-mcp-core |
| `crates/fabryk-mcp-graph/src/tools.rs` | use fabryk_mcp → fabryk_mcp_core |
| `crates/fabryk-mcp-semantic/Cargo.toml` | dep fabryk-mcp → fabryk-mcp-core |
| `crates/fabryk-mcp-semantic/src/tools.rs` | use fabryk_mcp → fabryk_mcp_core |
| `crates/fabryk/Cargo.toml` | Add auth/acl, make fts/graph/vector required, remove MCP/CLI |
| `crates/fabryk/src/lib.rs` | Add auth/acl, remove feature gates, remove MCP/CLI |

## Verification

1. `cargo check --workspace` — full compilation
2. `cargo test --workspace` — all tests pass
3. Verify `fabryk` facade exposes: `core`, `auth`, `acl`, `content`, `fts`, `graph`, `vector`
4. Verify `fabryk-mcp` facade exposes: all infra symbols (backward compat via `*` re-export) plus `auth`, `content`, `fts`, `graph`, `semantic`
5. Verify `fabryk-mcp-core` compiles independently with `http` feature
