---
title: "CC Prompt: Fabryk Checkpoint — Music Theory Migration"
milestone: "3.5"
phase: 3.5
author: "Claude (Opus 4.5)"
created: 2026-02-03
updated: 2026-02-03
prerequisites: ["Phase 1-3 complete", "v0.1-alpha tagged"]
governing-docs: [0011-audit §7, 0012-amendment, 0013-project-plan]
---

# CC Prompt: Fabryk Checkpoint — Music Theory Migration

## ⚠️ IMPORTANT: Timing

**This milestone executes AFTER Phase 3 completion**, not during Phase 1.

Per the confirmed checkpoint approach:
- Phases 1-3 build the Fabryk crates in isolation
- Music-theory continues using its local modules during this time
- This checkpoint milestone migrates music-theory to use Fabryk imports
- Executed once at the v0.1-alpha checkpoint, not incrementally

**Do NOT execute this milestone until:**
- [ ] Phase 1 complete (fabryk-core)
- [ ] Phase 2 complete (fabryk-content)
- [ ] Phase 3 complete (fabryk-fts, fabryk-graph)
- [ ] All Fabryk crates pass tests
- [ ] v0.1-alpha tag created

## Context

This is the **checkpoint migration** — the moment when music-theory switches
from its local modules to Fabryk imports. All previous phases built the Fabryk
crates in isolation. Now we wire them into the music-theory MCP server.

**Key constraint (Doc 0013):** The music-theory MCP server must remain fully
functional after this migration. All 25 MCP tools must work.
`cargo test --all-features` must pass in both repos.

## Why Checkpoint Approach?

1. **Avoids dual-maintenance**: No need to keep both local and Fabryk versions
   in sync during development
2. **Clean cut-over**: One migration instead of incremental updates
3. **Easier testing**: Fabryk crates are fully tested before integration
4. **Simpler rollback**: If issues arise, music-theory still has local code

## Objective

1. Add all Fabryk crates as dependencies to ai-music-theory
2. Implement required traits (`ConfigProvider`, `ContentProvider`, etc.)
3. Update all imports from local modules to Fabryk crates
4. Remove extracted files from the music-theory repo
5. Verify: `cargo test --all-features` passes in **both** repos

## Pre-Migration Checklist

Before starting this milestone:

```bash
# Verify Fabryk is ready
cd ~/lab/oxur/ecl
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Check version tag exists
git tag -l 'fabryk*v0.1*'
```

## Implementation Steps

### Step 0: Create a working branch

```bash
cd ~/lab/music-comp/ai-music-theory
git checkout -b feature/fabryk-migration
```

### Step 1: Add Fabryk dependencies

Edit `crates/server/Cargo.toml`:

```toml
[dependencies]
# Fabryk crates (path dependencies during development)
fabryk-core = { path = "../../../oxur/ecl/crates/fabryk-core" }
fabryk-content = { path = "../../../oxur/ecl/crates/fabryk-content" }
fabryk-fts = { path = "../../../oxur/ecl/crates/fabryk-fts", optional = true }
fabryk-graph = { path = "../../../oxur/ecl/crates/fabryk-graph", optional = true }
fabryk-mcp = { path = "../../../oxur/ecl/crates/fabryk-mcp" }

[features]
fts = ["fabryk-fts"]
graph = ["fabryk-graph"]
```

**Note:** Adjust paths based on actual directory layout. The paths above assume:
- Music-theory: `~/lab/music-comp/ai-music-theory`
- ECL: `~/lab/oxur/ecl`

### Step 2: Implement ConfigProvider

```rust
// In config.rs or config_provider.rs
use fabryk_core::traits::ConfigProvider;

impl ConfigProvider for Config {
    fn project_name(&self) -> &str {
        "music-theory"
    }

    fn base_path(&self) -> fabryk_core::Result<PathBuf> {
        self.paths.base_path()
            .map_err(|e| fabryk_core::Error::config(e.to_string()))
    }

    fn content_path(&self, content_type: &str) -> fabryk_core::Result<PathBuf> {
        let base = self.base_path()?;
        Ok(base.join(content_type))
    }
}
```

### Step 3: Update imports systematically

Process modules in order, compiling after each:

**Phase 1 (fabryk-core):**
```
crate::error::{Error, Result}     → fabryk_core::{Error, Result}
crate::util::files::*             → fabryk_core::util::files::*
crate::util::paths::*             → use PathResolver instead
crate::state::AppState            → fabryk_core::state::AppState (or wrapper)
```

**Phase 2 (fabryk-content):**
```
crate::content::*                 → fabryk_content::*
```

**Phases 3-4 (fabryk-fts, fabryk-graph):**
```
crate::search::*                  → fabryk_fts::* (feature-gated)
crate::graph::*                   → fabryk_graph::* (feature-gated)
```

### Step 4: Handle AppState transition

The current `AppState` holds domain-specific state (search, graph). Options:

**Option A — Wrapper struct (recommended for minimal churn):**

```rust
use fabryk_core::state::AppState as CoreState;

pub struct AppState {
    core: CoreState<Config>,
    search: Arc<RwLock<Option<SearchBackend>>>,
    graph: Arc<RwLock<Option<GraphState>>>,
}

impl AppState {
    pub fn config(&self) -> &Config {
        self.core.config()
    }
    // ... delegate and extend
}
```

**Option B — Separate state:**

Pass search and graph state separately instead of through AppState.

### Step 5: Update path resolution

Replace hardcoded path functions with PathResolver:

```rust
use fabryk_core::PathResolver;

lazy_static! {
    static ref RESOLVER: PathResolver = PathResolver::new("music-theory")
        .with_config_marker("config/default.toml")
        .with_project_markers(&["SKILL.md", "CONVENTIONS.md"]);
}

pub fn config_dir() -> Option<PathBuf> {
    RESOLVER.config_dir()
}
```

### Step 6: Remove extracted files

Only after the crate compiles:

```bash
cd ~/lab/music-comp/ai-music-theory/crates/server/src

# Files extracted to fabryk-core
rm error.rs
rm -r util/  # files.rs, paths.rs extracted

# Files extracted to fabryk-content
rm -r content/loader.rs  # if extracted

# Update lib.rs module declarations
```

### Step 7: Verify

```bash
# Music-theory tests
cd ~/lab/music-comp/ai-music-theory
cargo test --all-features
cargo clippy --all-features -- -D warnings

# Fabryk tests (sanity check)
cd ~/lab/oxur/ecl
cargo test --workspace
```

### Step 8: Functional verification

```bash
cd ~/lab/music-comp/ai-music-theory
cargo run --features fts,graph -- serve
```

Test:
- [ ] Server starts
- [ ] Health tool responds
- [ ] `list_concepts` works
- [ ] `search_concepts` works
- [ ] `get_concept` works
- [ ] Graph tools work

## Exit Criteria

- [ ] All Fabryk crates added as dependencies
- [ ] `ConfigProvider` implemented for music-theory `Config`
- [ ] All imports updated from local modules to Fabryk crates
- [ ] Extracted files removed from music-theory repo
- [ ] No duplicate code between repos
- [ ] `cargo test --all-features` passes in ai-music-theory
- [ ] `cargo test --workspace` passes in ECL
- [ ] All 25 MCP tools functional
- [ ] `cargo clippy` clean in both repos

## Risk Mitigation

1. **Compile incrementally**: Update one module at a time
2. **Keep old files initially**: Delete only after crate compiles
3. **Git commit at checkpoints**: Easy rollback if needed
4. **Run the MCP server**: Don't rely only on tests

## Commit Message

```
feat: migrate ai-music-theory to Fabryk crates

Switch from local modules to Fabryk imports:
- fabryk-core: errors, utilities, paths, ConfigProvider
- fabryk-content: content loading
- fabryk-fts: full-text search (feature-gated)
- fabryk-graph: concept graph (feature-gated)
- fabryk-mcp: MCP protocol types

Implement ConfigProvider for music-theory Config.
Remove extracted local modules.

All 25 MCP tools verified functional.

BREAKING: Internal module paths changed. No public API change.

Ref: Doc 0013 checkpoint migration

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

## Post-Migration

After successful migration:

1. **Tag the migration commit**: `git tag -a v0.5.0-fabryk -m "Fabryk migration complete"`
2. **Update CHANGELOG**: Document the migration
3. **Consider publishing**: Fabryk crates to crates.io (optional)
4. **Remove path dependencies**: Switch to version dependencies for release
