# Stage 1.1: Project Scaffolding

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 33-509. Original bytes are recoverable from import commit 2d006a7. -->

## Stage 1.1: Project Scaffolding

**Goal:** Establish project structure, dependencies, and build configuration.

**Dependencies:** None (first stage)

### Detailed Implementation Steps

#### Step 1: Initialize Workspace Structure

```bash
# Create workspace root Cargo.toml
# Create crate directories
# Set up git structure if not already done
```

**Files to create:**

1. **`Cargo.toml` (workspace root)**

```toml
[workspace]
resolver = "2"
members = [
    "crates/ecl-core",
    "crates/ecl-steps",
    "crates/ecl-workflows",
    "crates/ecl-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"  # Verify current stable
authors = ["ECL Contributors"]
license = "Apache-2.0"
repository = "https://github.com/oxur/ecl"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# Restate SDK
restate-sdk = "0.7"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres", "chrono", "uuid"] }

# LLM (verify version on crates.io)
llm = "0.2"

# Resilience
backon = "1"
failsafe = "1"

# Configuration
confyg = "0.3"

# Logging
twyg = "0.6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Error handling
thiserror = "2"
anyhow = "1"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Async trait support
async-trait = "0.1"

# CLI
clap = { version = "4", features = ["derive", "env"] }

# HTTP
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# Testing
proptest = "1"
mockall = "0.12"
tempfile = "3"

[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true

[profile.test]
opt-level = 1
```

2. **`rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin"]
```

#### Step 2: Create Crate Scaffolding

For each crate, create:

**`crates/ecl-core/Cargo.toml`**

```toml
[package]
name = "ecl-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
# Workspace dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
async-trait = { workspace = true }
sqlx = { workspace = true }
backon = { workspace = true }
failsafe = { workspace = true }
twyg = { workspace = true }
tracing = { workspace = true }
llm = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

**`crates/ecl-core/src/lib.rs`**

```rust
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Core Library
//!
//! Core types, traits, and utilities for the ECL workflow orchestration system.

// Public modules
pub mod error;
pub mod types;

// Re-exports for convenience
pub use error::{Error, Result};
pub use types::{StepId, WorkflowId, WorkflowState};
```

**Similar structure for:**
- `crates/ecl-steps/Cargo.toml` and `src/lib.rs`
- `crates/ecl-workflows/Cargo.toml` and `src/lib.rs`
- `crates/ecl-cli/Cargo.toml` and `src/main.rs`

#### Step 3: Development Tooling

**`justfile`**

```just
# ECL Development Commands

# Default: show available commands
default:
    @just --list

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests with coverage
coverage:
    cargo llvm-cov --workspace --all-features --html
    @echo "Coverage report: target/llvm-cov/html/index.html"

# Check code quality
lint:
    cargo clippy --workspace --all-features --all-targets -- -D warnings
    cargo fmt --all -- --check

# Format code
format:
    cargo fmt --all

# Build all crates
build:
    cargo build --workspace --all-features

# Build for release
build-release:
    cargo build --workspace --all-features --release

# Run development environment (Restate + app)
dev:
    docker compose up -d restate
    cargo run --bin ecl-workflows

# Stop development environment
dev-stop:
    docker compose down

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/

# Check for dependency updates
outdated:
    cargo outdated --workspace

# Run security audit
audit:
    cargo audit

# Generate documentation
docs:
    cargo doc --workspace --all-features --no-deps --open
```

**`.env.example`**

```bash
# ECL Configuration

# Environment
ECL_ENV=development
RUST_LOG=info,ecl=debug

# Database
DATABASE_URL=sqlite://ecl.db
# DATABASE_URL=postgres://ecl:password@localhost/ecl

# Claude API
ANTHROPIC_API_KEY=your_api_key_here

# Restate
RESTATE_URL=http://localhost:8080

# API Server
ECL_API_PORT=3000
ECL_API_KEY=dev_api_key_change_in_production

# Artifact Storage
ECL_ARTIFACT_PATH=./artifacts
```

**`docker-compose.yml`**

```yaml
version: '3.8'

services:
  restate:
    image: restatedev/restate:latest
    ports:
      - "8080:8080"  # Ingress
      - "9070:9070"  # Admin
    environment:
      - RUST_LOG=info
    volumes:
      - restate-data:/data

  postgres:
    image: postgres:16-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: ecl
      POSTGRES_USER: ecl
      POSTGRES_PASSWORD: ecl_dev_password
    volumes:
      - postgres-data:/var/lib/postgresql/data

volumes:
  restate-data:
  postgres-data:
```

**`.gitignore`**

```gitignore
# Rust
/target/
**/*.rs.bk
Cargo.lock

# IDE
.idea/
.vscode/
*.swp
*.swo

# Environment
.env
.env.local

# Data
*.db
*.db-shm
*.db-wal
/artifacts/

# Coverage
target/llvm-cov/
*.profraw
*.profdata

# OS
.DS_Store
Thumbs.db
```

#### Step 4: Basic Documentation

**`README.md`** (update existing or create)

```markdown
# ECL - Event-driven Codebase Lifecycle

Rust-based AI workflow orchestration system built on Restate.

## Quick Start

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Docker and Docker Compose
- Just command runner: `cargo install just`

### Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and configure
3. Start dependencies: `just dev-stop && docker compose up -d`
4. Run tests: `just test`
5. Build: `just build`

### Development

```bash
# Run tests
just test

# Run with coverage
just coverage

# Format and lint
just format
just lint

# Start dev environment
just dev
```

## Project Structure

```
ecl/
├── crates/
│   ├── ecl-core/      # Core types, traits, error handling
│   ├── ecl-steps/     # Step implementations
│   ├── ecl-workflows/ # Restate workflow definitions
│   └── ecl-cli/       # Command-line interface
├── docs/              # Documentation
└── migrations/        # Database migrations
```

## Documentation

- [Getting Started](docs/getting-started.md)
- [Architecture](crates/design/docs/05-active/0004-ecl-project-plan.md)
- [API Documentation](https://docs.rs/ecl)

## License

Apache 2.0
```

### Testing Strategy

**Test files to create:**

1. **Workspace builds**: `cargo build --workspace`
2. **Linting passes**: `cargo clippy --workspace -- -D warnings`
3. **Formatting**: `cargo fmt --all -- --check`
4. **Dependency resolution**: All workspace dependencies resolve

**Test data needed:** None for this stage

**Tests to write:**

```rust
// crates/ecl-core/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_compiles() {
        // This test just ensures the crate compiles
        // More substantive tests come in later stages
    }
}
```

### Potential Blockers

1. **Dependency version conflicts**
   - **Mitigation**: Pin workspace dependencies carefully
   - **Mitigation**: Test build before proceeding

2. **Restate SDK API changes**
   - **Mitigation**: Check crates.io for current version
   - **Mitigation**: Review Restate documentation

3. **`llm` crate availability/API**
   - **Mitigation**: Verify crate exists and supports Claude
   - **Fallback**: Implement direct HTTP client to Claude API

### Acceptance Criteria

- [ ] `cargo build --workspace` succeeds with no errors
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] All crates compile and link correctly
- [ ] Restate SDK dependency resolves
- [ ] `just test` runs successfully
- [ ] Documentation builds with `cargo doc --workspace --no-deps`
- [ ] Docker Compose brings up Restate server

### Estimated Effort

**Time:** 2-3 hours

**Breakdown:**
- Cargo.toml setup: 30 min
- Crate scaffolding: 45 min
- Tooling (justfile, docker-compose): 30 min
- Documentation: 30 min
- Testing and validation: 30 min

---

