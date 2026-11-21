# Fabryk Extraction Project — Context Bootstrap

**Last Updated**: 2026-02-04
**Current Phase**: Phase 1 (fabryk-core foundation)
**Current Milestone**: Ready for 1.3 (File & Path Utilities)
**Status**: ✅ Milestones 1.0, 1.1, 1.2 complete

---

## Table of Contents

1. [Getting Full Context](#getting-full-context)
2. [Project Overview](#project-overview)
3. [Key Concepts](#key-concepts)
4. [Repository Structure](#repository-structure)
5. [Current Status](#current-status)
6. [Next Steps](#next-steps)
7. [Development Workflow](#development-workflow)
8. [Key Documents](#key-documents)
9. [Critical Conventions](#critical-conventions)

---

## Getting Full Context

**⚠️ IMPORTANT**: This bootstrap document is a **navigation guide**. For complete understanding, you MUST read the actual source documents. Don't rely on summaries—read the originals!

### Essential Reading (In Order)

Read these files to get complete context on the project:

#### 1. Rust Development Standards

```bash
# ALWAYS read this first - critical Rust conventions
~/lab/oxur/ecl/assets/ai/ai-rust/skills/claude/SKILL.md

# ALWAYS load before writing code - 80 anti-patterns to avoid
~/lab/oxur/ecl/assets/ai/ai-rust/guides/11-anti-patterns.md
```

**Why**: These define the coding standards you MUST follow. Every milestone requires these.

#### 2. Project Planning Documents (Read in Order)

```bash
# Read these to understand the extraction project:

~/lab/oxur/ecl/crates/design/dev/fabryk/0011-audit.md
# → Original extraction audit
# → File inventory and classification system (G/P/D/S)
# → Why each file was chosen for extraction

~/lab/oxur/ecl/crates/design/dev/fabryk/0012-amendment.md
# → Checkpoint approach clarification
# → Why music-theory doesn't migrate until v0.1-alpha
# → Avoids dual-maintenance complexity

~/lab/oxur/ecl/crates/design/dev/fabryk/0013-project-plan.md
# → Complete phase breakdown (Phases 1-6)
# → Dependency level architecture
# → Milestone-by-milestone plan
# → Feature flag strategy
```

**Why**: These explain the **what** and **why** of the entire extraction project.

#### 3. Completed Milestone Documents (For Context)

```bash
# Milestones 1.0 & 1.1: Workspace setup
~/lab/oxur/ecl/crates/design/dev/fabryk/0001-cc-prompt-fabryk-1.md
# → ECL workspace cleanup (removed legacy crates)
# → Scaffolded all 11 Fabryk crates
# → Workspace structure and dependencies
# → Commit: dab3814

# Milestone 1.2: Error types extraction
~/lab/oxur/ecl/crates/design/dev/fabryk/0002-cc-prompt-fabryk-1.md
# → Full error type implementation
# → Constructor helpers and inspector methods
# → Test suite (16 tests)
# → Why MCP error mapping goes in fabryk-mcp, not core
# → Commit: e658ada
```

**Why**: See what's been done and the patterns established.

#### 4. Next Milestone Document (What to Execute)

```bash
# Milestone 1.3: File & path utilities (NEXT TO EXECUTE)
~/lab/oxur/ecl/crates/design/dev/fabryk/0003-cc-prompt-fabryk-1.md
# → Extract util/files.rs and util/paths.rs
# → Async file discovery functions
# → Path resolution utilities
# → Testing requirements
```

**Why**: This is your execution blueprint for the next work.

#### 5. Additional Context Documents (As Needed)

```bash
# When working on specific milestones, also read:

# For error handling (all milestones)
~/lab/oxur/ecl/assets/ai/ai-rust/guides/03-error-handling.md

# For project structure (crate creation)
~/lab/oxur/ecl/assets/ai/ai-rust/guides/12-project-structure.md

# Checkpoint migration (Phase 3.5, much later)
~/lab/oxur/ecl/crates/design/dev/fabryk/0007-cc-prompt-fabryk-3.5.md
```

### Quick Start Checklist

Before starting work on any milestone:

- [ ] Read SKILL.md
- [ ] Read 11-anti-patterns.md (ALWAYS!)
- [ ] Read the 3 planning docs (0011, 0012, 0013) if first time
- [ ] Skim completed milestone docs (0001, 0002) for patterns
- [ ] **READ THE CURRENT MILESTONE DOC COMPLETELY** (0003 for next work)
- [ ] Check which additional guides the milestone references
- [ ] Read those guides before starting

### How to Use This Bootstrap

This document provides:

- **Current status snapshot** (what's done, what's next)
- **Navigation map** (which files to read for what purpose)
- **Quick reference** (commands, conventions, paths)

This document does NOT provide:

- ❌ Complete implementation details (read milestone docs!)
- ❌ Full Rust patterns (read the guides!)
- ❌ Step-by-step instructions (read milestone docs!)

**Always prefer reading the original documents over relying on summaries.**

---

## Project Overview

### What is Fabryk?

Fabryk is a **domain-agnostic knowledge fabric framework** being extracted from the music-theory MCP server. The goal is to create reusable Rust crates that any knowledge domain can use to build MCP servers with:

- Content management (markdown parsing, frontmatter)
- Full-text search (Tantivy)
- Knowledge graphs (petgraph + rkyv persistence)
- MCP server infrastructure
- CLI frameworks

### The Two Projects

**1. ECL (Extract, Cogitate, Load)** — `~/lab/oxur/ecl/`

- Workflow orchestration framework (Restate-based)
- **Contains the Fabryk crates** being extracted
- Workspace with both ECL and Fabryk crates
- Current version: `0.1.0-alpha.0`

**2. Music-Theory MCP Server** — `~/lab/music-comp/ai-music-theory/mcp-server/`

- Production MCP server for music theory concepts
- **Source of code being extracted** to Fabryk
- Will eventually migrate to use Fabryk crates
- Continues using local code during extraction

### Extraction Strategy: Checkpoint-Based Migration

**Critical Understanding**: This is NOT incremental dual-maintenance.

```
Phase 1-3: Build Fabryk crates in isolation
    ↓
Music-theory keeps using local modules (no changes)
    ↓
v0.1-alpha checkpoint: Single coordinated migration
    ↓
Music-theory switches all imports to Fabryk at once
```

**Why checkpoint approach?**

- Avoids dual-maintenance complexity
- Clean cut-over at tested milestone
- Easier testing (Fabryk crates fully tested before integration)
- Simpler rollback if issues arise

---

## Key Concepts

### Crate Dependency Levels

Fabryk uses a layered architecture (no circular dependencies):

| Level | Crate | Dependencies |
|-------|-------|--------------|
| 0 | `fabryk-core` | None (foundation) |
| 1 | `fabryk-content` | core |
| 1 | `fabryk-fts` | core |
| 1 | `fabryk-graph` | core, content |
| 2 | `fabryk-mcp` | core |
| 3 | `fabryk-mcp-content` | core, content, mcp |
| 3 | `fabryk-mcp-fts` | core, fts, mcp |
| 3 | `fabryk-mcp-graph` | core, graph, mcp |
| 2 | `fabryk-cli` | core |
| N/A | `fabryk-acl` | core (placeholder for v0.2/v0.3) |
| 0 | `fabryk` | All (umbrella re-export) |

### Code Classification System

When extracting code from music-theory, files are classified:

- **G (Generic)**: Domain-agnostic, extract as-is
- **P (Parameterized)**: Generic with trait abstraction needed
- **D (Domain-specific)**: Stays in music-theory
- **S (Specialized)**: Domain-specific with extractable patterns

### Feature Flags

**Critical**: All features must be **additive** (can't break code when enabled).

- `fts-tantivy`: Enable Tantivy search backend
- `graph-rkyv-cache`: Enable rkyv-based graph persistence

---

## Repository Structure

### ECL Workspace (`~/lab/oxur/ecl/`)

```
ecl/
├── Cargo.toml              # Workspace config (version: 0.1.0-alpha.0)
├── crates/
│   ├── ecl/                # ECL main crate
│   ├── ecl-core/           # ECL core types
│   ├── ecl-steps/          # Step execution
│   ├── ecl-workflows/      # Workflow orchestration
│   ├── ecl-cli/            # ECL CLI
│   ├── design/             # Design docs (THIS PROJECT!)
│   │   └── dev/fabryk/     # Fabryk milestone documents
│   │       ├── 0001-*.md   # Milestones 1.0 & 1.1
│   │       ├── 0002-*.md   # Milestone 1.2
│   │       ├── 0003-*.md   # Milestone 1.3
│   │       ├── 0004-*.md   # Milestone 1.4
│   │       ├── 0005-*.md   # Milestone 1.5
│   │       ├── 0006-*.md   # Milestone 1.6
│   │       ├── 0007-*.md   # Milestone 3.5 (checkpoint)
│   │       ├── 0011-*.md   # Extraction audit
│   │       ├── 0012-*.md   # Amendment
│   │       └── 0013-*.md   # Project plan
│   └── fabryk*/            # 11 Fabryk crates (being built)
└── target/
```

### Music-Theory Source (`~/lab/music-comp/ai-music-theory/mcp-server/`)

```
mcp-server/
└── crates/server/src/
    ├── error.rs            # ✅ Extracted to fabryk-core
    ├── util/
    │   ├── files.rs        # 🔜 Next: Extract to fabryk-core
    │   ├── paths.rs        # 🔜 Next: Extract to fabryk-core
    │   └── ids.rs          # 🔜 Extract to fabryk-core
    ├── config.rs           # ⛔ Domain-specific (stays)
    ├── state.rs            # 🔜 Extract generic parts
    ├── content/            # 🔜 Extract to fabryk-content
    ├── search/             # 🔜 Extract to fabryk-fts
    ├── graph/              # 🔜 Extract to fabryk-graph
    └── tools/              # 🔜 Extract to fabryk-mcp-*
```

---

## Current Status

### ✅ Completed Milestones

#### Milestone 1.0: ECL Workspace Cleanup

**Commit**: `dab3814`

- Removed legacy crates: `fabryk-storage`, `fabryk-query`, `fabryk-api`, `fabryk-client`
- Gutted existing stubs to minimal shells
- Updated workspace `Cargo.toml`:
  - Version: `0.1.0-alpha.0`
  - Added all Fabryk dependencies (rmcp, schemars, pulldown-cmark, tantivy, petgraph, rkyv, etc.)
  - Updated ECL crate version constraints to match

#### Milestone 1.1: Workspace Scaffold

**Commit**: `dab3814` (same)

Created 6 new Fabryk crates with full structure:

- `fabryk-content` (markdown, frontmatter)
- `fabryk-fts` (search, feature: `fts-tantivy`)
- `fabryk-graph` (knowledge graph, feature: `graph-rkyv-cache`)
- `fabryk-mcp-content` (content MCP tools)
- `fabryk-mcp-fts` (search MCP tools)
- `fabryk-mcp-graph` (graph MCP tools)

Each with: `Cargo.toml`, `src/lib.rs`, `README.md`

**Verification**:

```bash
cd ~/lab/oxur/ecl
cargo check --workspace  # ✅ Passes
cargo clippy --workspace -- -D warnings  # ✅ Clean
```

#### Milestone 1.2: Error Types (Full Extraction)

**Commit**: `e658ada`

Extracted complete error implementation to `fabryk-core/src/error.rs`:

- **10 error variants**: `Io`, `IoWithPath`, `Config`, `Json`, `Yaml`, `NotFound`, `FileNotFound`, `InvalidPath`, `Parse`, `Operation`
- **Constructor helpers**: `io()`, `io_with_path()`, `config()`, `not_found()`, etc.
- **Inspector methods**: `is_io()`, `is_not_found()`, `is_config()`, `is_path_error()`, `is_parse()`
- **From implementations**: `std::io::Error`, `serde_json::Error`, `serde_yaml::Error`
- **16 comprehensive tests** (all passing)

**Note**: Using `thiserror 1.x` for Rust 1.75 compatibility. Backtrace support deferred until Rust 1.81+ upgrade.

**Verification**:

```bash
cd ~/lab/oxur/ecl
cargo test -p fabryk-core  # ✅ 16/16 tests pass
cargo clippy -p fabryk-core -- -D warnings  # ✅ Clean
cargo doc -p fabryk-core --no-deps  # ✅ Generates docs
```

---

## Next Steps

### 🔜 Milestone 1.3: File & Path Utilities

**Document**: `crates/design/dev/fabryk/0003-cc-prompt-fabryk-1.md`

**Objective**: Extract file discovery and path utilities from music-theory.

**Source files** (music-theory):

- `util/files.rs` (~200 lines) — async file discovery
- `util/paths.rs` (~150 lines) — path utilities

**Tasks**:

1. Create `fabryk-core/src/util/mod.rs`
2. Extract `files.rs` → `fabryk-core/src/util/files.rs`
   - `find_file_by_id()` — async file search
   - `find_all_files()` — async directory traversal
   - `FileInfo` struct
   - `FindOptions` for filtering
3. Extract `paths.rs` → `fabryk-core/src/util/paths.rs`
   - `binary_path()`, `binary_dir()`
   - `find_dir_with_marker()`
   - `expand_tilde()`
4. Write tests (use `tempfile` for filesystem tests)
5. Update `fabryk-core/src/lib.rs` to export `util` module

**Dependencies already in place**:

- `glob`, `shellexpand`, `dirs`, `async-walkdir`, `tokio`

### 📋 Remaining Phase 1 Milestones

- **1.4**: ID utilities & PathResolver
- **1.5**: ConfigProvider trait & AppState
- **1.6**: Phase 1 completion & verification

Then: Phase 2 (fabryk-content), Phase 3 (fabryk-fts, fabryk-graph), etc.

---

## Development Workflow

### Step-by-Step Process

For each milestone:

1. **Read the milestone document** in `crates/design/dev/fabryk/`
2. **Check referenced guides** (see Key Documents below)
3. **Read source files** from music-theory to understand implementation
4. **Extract code** to appropriate Fabryk crate
5. **Adapt for generality** (remove domain-specific assumptions)
6. **Write tests** (aim for 95%+ coverage)
7. **Verify compilation**:

   ```bash
   cargo check -p <crate>
   cargo test -p <crate>
   cargo clippy -p <crate> -- -D warnings
   cargo doc -p <crate> --no-deps
   ```

8. **Commit with proper message format** (see conventions below)

### Development Commands

```bash
# Switch to ECL workspace
cd ~/lab/oxur/ecl

# Check specific crate
cargo check -p fabryk-core

# Run tests for specific crate
cargo test -p fabryk-core

# Run all workspace tests
cargo test --workspace

# Lint with warnings as errors
cargo clippy --workspace -- -D warnings

# Generate docs
cargo doc -p fabryk-core --no-deps --open

# Check git status
git status
git log --oneline -5
```

---

## Key Documents

### Essential Reading Order

**1. Start with SKILL.md** — Rust development guidelines

```bash
~/lab/music-comp/ai-music-theory/mcp-server/assets/ai/ai-rust/skills/claude/SKILL.md
```

- Critical rules to always apply
- Pattern ID reference (AP-XX, EH-XX, PS-XX)
- Which guides to read for different tasks

**2. Anti-Patterns (ALWAYS load first)**

```bash
~/lab/music-comp/ai-music-theory/mcp-server/assets/ai/ai-rust/guides/11-anti-patterns.md
```

- 80 anti-patterns to avoid (AP-01 through AP-80)
- **Critical**: AP-02 (use `&str` not `&String`), AP-09 (no unwrap in libraries), AP-19 (don't expose ErrorKind), AP-54 (no sync I/O in async)

**3. Error Handling**

```bash
~/lab/music-comp/ai-music-theory/mcp-server/assets/ai/ai-rust/guides/03-error-handling.md
```

- 32 error handling patterns (EH-01 through EH-32)
- **Critical**: EH-17 (canonical error structs), EH-11 (is_xxx() methods)

**4. Project Structure**

```bash
~/lab/music-comp/ai-music-theory/mcp-server/assets/ai/ai-rust/guides/12-project-structure.md
```

- 31 project structure patterns (PS-01 through PS-31)
- **Critical**: PS-04 (error module pattern), PS-06 (features are additive)

### Fabryk Project Documents

All in `~/lab/oxur/ecl/crates/design/dev/fabryk/`:

| Document | Purpose |
|----------|---------|
| `0011-audit.md` | Original extraction audit, file inventory |
| `0012-amendment.md` | Checkpoint approach clarification |
| `0013-project-plan.md` | Complete phase breakdown, dependency levels |
| `0001-*.md` | Milestones 1.0 & 1.1 (cleanup & scaffold) |
| `0002-*.md` | Milestone 1.2 (error types) |
| `0003-*.md` | Milestone 1.3 (file & path utilities) |
| `0004-*.md` | Milestone 1.4 (ID utilities & PathResolver) |
| `0005-*.md` | Milestone 1.5 (ConfigProvider & AppState) |
| `0006-*.md` | Milestone 1.6 (Phase 1 verification) |
| `0007-*.md` | Milestone 3.5 (checkpoint migration) |

---

## Critical Conventions

### Rust Code Standards

**1. Parameter Types** (AP-02)

```rust
// ✅ Good
pub fn process(name: &str, items: &[Item])

// ❌ Bad
pub fn process(name: &String, items: &Vec<Item>)
```

**2. Error Handling** (AP-09)

```rust
// ✅ Good (library code)
pub fn load_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)?
}

// ❌ Bad (library code)
pub fn load_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()  // Never in libraries!
}
```

**3. Derive Traits** (from SKILL.md)

```rust
// Minimum for all types
#[derive(Debug, Clone, PartialEq, Eq)]

// Add as needed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
```

**4. Async I/O** (AP-54)

```rust
// ✅ Good (in async context)
use tokio::fs;
async fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).await?
}

// ❌ Bad (in async context)
async fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)?  // Blocks async runtime!
}
```

**5. Inspector Methods** (EH-11, AP-19)

```rust
impl Error {
    // ✅ Good: Provide is_xxx() methods
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }
}

// ❌ Bad: Don't expose ErrorKind enum
pub enum ErrorKind { /* ... */ }  // Don't do this
```

### Git Commit Format

```
<type>(<scope>): <subject>

<body>

Ref: <doc references>

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**Types**: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`

**Example**:

```
feat(core): extract file discovery utilities

Add fabryk-core/src/util/files.rs with async file operations:
- find_file_by_id() for single file search
- find_all_files() for directory traversal
- FileInfo struct with metadata
- FindOptions for filtering

Tests: 12/12 passing, coverage >95%

Ref: Doc 0013 milestone 1.3, Audit §4.1

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### Testing Standards

**1. Test Naming** (from ai-rust guides)

```rust
#[test]
fn test_<function>_<scenario>_<expectation>() {
    // Example: test_find_file_by_id_found_returns_path()
}
```

**2. Use tempfile for Filesystem Tests**

```rust
#[test]
fn test_find_all_files_empty_directory_returns_empty() {
    let temp_dir = tempfile::tempdir().unwrap();
    let result = find_all_files(temp_dir.path()).await.unwrap();
    assert!(result.is_empty());
}
```

**3. Coverage Target**: ≥95%

### Dependency Management

**Version constraints** for publishable crates:

```toml
# ✅ Good: Explicit version for crates that will be published
fabryk-core = { version = "0.1.0-alpha.0", path = "../fabryk-core" }

# ❌ Bad: No version breaks publishing
fabryk-core = { path = "../fabryk-core" }
```

**Workspace dependency inheritance**:

```toml
[dependencies]
# Use workspace = true for centralized version management
tokio = { workspace = true }
serde = { workspace = true }
```

---

## Quick Reference

### File Paths

| Item | Path |
|------|------|
| ECL workspace | `~/lab/oxur/ecl/` |
| Fabryk crates | `~/lab/oxur/ecl/crates/fabryk-*/` |
| Milestone docs | `~/lab/oxur/ecl/crates/design/dev/fabryk/` |
| Music-theory source | `~/lab/music-comp/ai-music-theory/mcp-server/` |
| Rust guides | `~/lab/music-comp/ai-music-theory/mcp-server/assets/ai/ai-rust/guides/` |

### Version Information

- **Workspace version**: `0.1.0-alpha.0`
- **Rust edition**: 2021
- **Rust version**: 1.75
- **thiserror**: 1.x (for Rust 1.75 compatibility)

### Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `tokio` | 1 | Async runtime |
| `serde` | 1 | Serialization |
| `thiserror` | 1 | Error derive macros |
| `rmcp` | 0.14 | MCP protocol |
| `pulldown-cmark` | 0.9 | Markdown parsing |
| `tantivy` | 0.22 | Full-text search |
| `petgraph` | 0.6 | Graph algorithms |
| `rkyv` | 0.7 | Graph persistence |

---

## How to Continue

### If Starting Fresh

1. Read this document completely
2. Read SKILL.md and the anti-patterns guide
3. Read milestone 0013 (project plan overview)
4. Review the three completed milestones (0001, 0002)
5. Read milestone 0003 (next up: 1.3)
6. Execute milestone 1.3 following the workflow above

### If Resuming After Break

1. Skim this document to refresh context
2. Check current git status: `cd ~/lab/oxur/ecl && git log --oneline -5`
3. Verify tests still pass: `cargo test --workspace`
4. Read the next milestone document
5. Continue extraction

### If Stuck

**Check these in order:**

1. Is the workspace compiling? `cargo check --workspace`
2. Are tests passing? `cargo test -p <crate>`
3. Is clippy happy? `cargo clippy -p <crate> -- -D warnings`
4. Review the milestone document again
5. Check the source code in music-theory for reference
6. Consult the anti-patterns guide for common issues

---

## Success Criteria

You're on track if:

- ✅ Workspace compiles: `cargo check --workspace`
- ✅ All tests pass: `cargo test --workspace`
- ✅ Clippy is clean: `cargo clippy --workspace -- -D warnings`
- ✅ Documentation builds: `cargo doc --workspace --no-deps`
- ✅ Git history is clean with proper commit messages
- ✅ Each milestone has clear, focused commits
- ✅ Music-theory code remains unchanged (checkpoint approach)

---

## Contact & Resources

- **Git repository**: <https://github.com/oxur/ecl>
- **Milestone documents**: `~/lab/oxur/ecl/crates/design/dev/fabryk/`
- **Rust guides**: `~/lab/music-comp/ai-music-theory/mcp-server/assets/ai/ai-rust/guides/`

**Remember**: When in doubt, read the milestone document again and check the guides. The extraction is methodical and well-documented—trust the process!

---

**End of Bootstrap Document**
