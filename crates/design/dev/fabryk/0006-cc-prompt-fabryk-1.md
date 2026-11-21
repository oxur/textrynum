---
title: "CC Prompt: Fabryk 1.6 — Phase 1 Completion & Verification"
milestone: "1.6"
phase: 1
author: "Claude (Opus 4.5)"
created: 2026-02-03
updated: 2026-02-03
prerequisites: ["1.0-1.5 completed"]
governing-docs: [0011-audit §4.1, 0012-amendment, 0013-project-plan]
---

# CC Prompt: Fabryk 1.6 — Phase 1 Completion & Verification

## Context

This milestone was originally planned to extract `resources/mod.rs`. However,
upon review, that module is **domain-specific (Classification: D)** — it contains
hardcoded music-theory content (conventions, scope, source materials, skill
index).

**Classification correction**: The resources module stays in music-theory.

This milestone is repurposed as the **Phase 1 completion verification**.

**Music-Theory Migration**: Music-theory continues using its local code until
the v0.1-alpha checkpoint (after Phase 3 completion), when all imports will be
updated in a single coordinated migration.

## Why resources/mod.rs Stays in Music-Theory

The `resources/mod.rs` file (378 lines) contains:

1. **Hardcoded music-theory URIs**: `skill://conventions`, `skill://scope`, etc.
2. **Domain-specific resource names**: "Music Theory Conventions", "Skill Scope"
3. **Default content with music theory terminology**: references to Lewin,
   Tymoczko, Neo-Riemannian theory, set theory
4. **Music-theory specific file paths**: `CONVENTIONS.md`, `SCOPE.md`

This is not extractable to a generic framework. Each Fabryk domain will have
its own resource serving module with its own URIs and content.

**Future consideration**: If a pattern emerges across domains, we could extract
a `ResourceProvider` trait in Phase 5 (fabryk-mcp). For now, leave as-is.

## Objective

Verify that all Phase 1 milestones (1.0-1.5) are complete and fabryk-core is
ready for use:

1. Confirm fabryk-core structure matches specifications
2. Run full test suite and clippy
3. Generate documentation
4. Create a v0.1-alpha tag (optional, for checkpoint tracking)

## Verification Steps

### Step 1: Verify module structure

```bash
cd ~/lab/oxur/ecl/crates/fabryk-core

# Expected structure:
# fabryk-core/src/
# ├── error.rs       (milestone 1.2)
# ├── lib.rs         (all milestones)
# ├── state.rs       (milestone 1.5)
# ├── traits.rs      (milestone 1.5)
# └── util/
#     ├── mod.rs     (milestones 1.3-1.4)
#     ├── files.rs   (milestone 1.3)
#     ├── ids.rs     (milestone 1.4)
#     ├── paths.rs   (milestone 1.3)
#     └── resolver.rs (milestone 1.4)

tree src/
```

### Step 2: Verify public exports

Check `lib.rs` exports:

```rust
// Expected exports:
pub mod error;
pub mod state;
pub mod traits;
pub mod util;

pub use error::{Error, Result};
pub use traits::ConfigProvider;
pub use util::ids::{id_from_path, normalize_id};
pub use util::resolver::PathResolver;
```

### Step 3: Run tests

```bash
cd ~/lab/oxur/ecl
cargo test -p fabryk-core
```

Expected: All tests pass.

### Step 4: Run clippy

```bash
cargo clippy -p fabryk-core -- -D warnings
```

Expected: No warnings.

### Step 5: Generate documentation

```bash
cargo doc -p fabryk-core --no-deps --open
```

Review:
- [ ] Crate-level docs present
- [ ] All public types documented
- [ ] Examples compile (via `cargo test --doc -p fabryk-core`)

### Step 6: Verify Error type coverage

```rust
// Error should have these variants (from milestone 1.2):
Error::Io(std::io::Error)
Error::IoWithPath { path, message, backtrace }
Error::Config(String)
Error::Json(serde_json::Error)
Error::Yaml(serde_yaml::Error)
Error::NotFound { resource_type, id }
Error::FileNotFound { path }
Error::InvalidPath { path, reason }
Error::Parse(String)
Error::Operation(String)

// And these constructors:
Error::io(err)
Error::io_with_path(err, path)
Error::config(msg)
Error::not_found(resource_type, id)
Error::not_found_msg(msg)
Error::file_not_found(path)
Error::invalid_path(path, reason)
Error::parse(msg)
Error::operation(msg)
```

### Step 7: Verify ConfigProvider trait

```rust
// ConfigProvider should require:
pub trait ConfigProvider: Send + Sync + Clone + 'static {
    fn project_name(&self) -> &str;
    fn base_path(&self) -> Result<PathBuf>;
    fn content_path(&self, content_type: &str) -> Result<PathBuf>;
}
```

### Step 8: Verify PathResolver functionality

```rust
// PathResolver should support:
PathResolver::new(project_name)
    .with_config_marker(marker)
    .with_project_markers(markers)
    .with_config_fallback(path)
    .with_project_fallback(path)

resolver.env_var(suffix) -> String
resolver.config_dir() -> Option<PathBuf>
resolver.project_root() -> Option<PathBuf>
```

### Step 9: Create phase completion tag (optional)

```bash
cd ~/lab/oxur/ecl
git tag -a fabryk-core-v0.1-alpha -m "Fabryk Core Phase 1 complete

fabryk-core is ready with:
- Error types with backtrace support
- File discovery utilities (async)
- Path utilities and PathResolver
- ID normalization utilities
- ConfigProvider trait
- Generic AppState<C>

Ref: Doc 0013 Phase 1"
```

## Exit Criteria

- [ ] fabryk-core module structure matches specification
- [ ] All public exports in lib.rs verified
- [ ] `cargo test -p fabryk-core` passes (all tests)
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean
- [ ] `cargo doc -p fabryk-core --no-deps` builds without warnings
- [ ] Doc tests pass (`cargo test --doc -p fabryk-core`)
- [ ] Error type has all variants and constructors from milestone 1.2
- [ ] ConfigProvider trait defined with required methods
- [ ] PathResolver supports builder pattern and all resolution methods
- [ ] AppState<C: ConfigProvider> is Clone and provides config access

## Phase 1 Summary

After this milestone, fabryk-core provides:

| Module | Contents |
|--------|----------|
| `error` | Error enum with backtrace, Result type alias |
| `util::files` | find_file_by_id, find_all_files, FileInfo, FindOptions |
| `util::paths` | binary_path, binary_dir, find_dir_with_marker, expand_tilde |
| `util::ids` | normalize_id, id_from_path |
| `util::resolver` | PathResolver struct with builder pattern |
| `traits` | ConfigProvider trait |
| `state` | AppState<C: ConfigProvider> generic wrapper |

**What's NOT in fabryk-core:**

- MCP-specific types (deferred to fabryk-mcp, Phase 5)
- Resource serving (domain-specific, stays in each domain)
- Search/FTS functionality (fabryk-fts, Phases 3-4)
- Graph functionality (fabryk-graph, Phases 3-4)
- Domain-specific configuration (stays in each domain)

## Next Steps

Phase 1 complete. Proceed to Phase 2:

- **Milestone 2.1**: fabryk-content crate (content loading abstractions)
- **Milestone 2.2**: ContentProvider trait
- **Milestone 2.3**: Metadata extraction utilities

## Commit Message

```
chore(core): verify Phase 1 completion

All Phase 1 milestones (1.0-1.5) complete. fabryk-core provides:
- Error types with backtrace support
- Async file discovery utilities
- Generic path utilities and PathResolver
- ID normalization utilities
- ConfigProvider trait for domain abstraction
- Generic AppState<C>

Note: resources module stays in music-theory (domain-specific content).

Ref: Doc 0013 Phase 1 completion

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
