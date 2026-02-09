---
title: "CC Prompt: Fabryk 6.3 — Music Theory CLI Integration"
milestone: "6.3"
phase: 6
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["6.1-6.2 complete"]
governing-docs: [0011-audit §7, 0013-project-plan]
---

# CC Prompt: Fabryk 6.3 — Music Theory CLI Integration

## Context

This is the **final milestone of Phase 6**. It integrates `fabryk-cli` into
ai-music-theory and removes the extracted CLI files.

## Objective

Complete CLI integration in ai-music-theory:

1. Use `FabrykCli<MusicTheoryConfig>` as the CLI base
2. Override graph build with `MusicTheoryExtractor`
3. Register music-theory-specific subcommands
4. Remove extracted CLI files
5. Verify all CLI commands work

## Implementation Steps

### Step 1: Update music-theory Cargo.toml

```toml
[dependencies]
fabryk-cli = { path = "../../fabryk-cli" }
```

### Step 2: Update main.rs

Create new `crates/music-theory/mcp-server/src/main.rs`:

```rust
//! Music theory MCP server and CLI.

use clap::Parser;
use fabryk_cli::{CliArgs, BaseCommand, GraphCommand, FabrykCli, handle_build, BuildOptions};
use fabryk_core::Result;
use music_theory_mcp_server::{Config, MusicTheoryExtractor};

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    // Load configuration
    let config = Config::load()?;

    // Create CLI application
    let cli = MusicTheoryCli::new(config);
    cli.run(args).await
}

/// Music theory CLI - extends FabrykCli with domain-specific commands.
struct MusicTheoryCli {
    inner: FabrykCli<Config>,
    config: Config,
}

impl MusicTheoryCli {
    fn new(config: Config) -> Self {
        let inner = FabrykCli::new("music-theory", config.clone())
            .with_version(env!("CARGO_PKG_VERSION"));
        Self { inner, config }
    }

    async fn run(self, args: CliArgs) -> Result<()> {
        self.inner.init_logging(&args);

        match args.command {
            // Override graph build to use MusicTheoryExtractor
            Some(BaseCommand::Graph(GraphCommand::Build { output, dry_run })) => {
                let extractor = MusicTheoryExtractor::new();
                let manual_edges = self.config.base_path()?.join("data/graphs/manual_edges.json");

                let options = BuildOptions {
                    output,
                    dry_run,
                    manual_edges: Some(manual_edges),
                };

                handle_build(&self.config, extractor, options).await
            }

            // Delegate other commands to base CLI
            _ => self.inner.run(args).await,
        }
    }
}
```

### Step 3: Remove extracted CLI files

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server/src

# Remove extracted files
rm cli.rs      # Replaced by fabryk-cli
rm graph/cli.rs  # Replaced by fabryk-cli graph handlers

# Keep these if they have domain-specific commands not yet extracted
# Otherwise remove them too
```

### Step 4: Update lib.rs

```rust
//! Music theory MCP server library.

pub mod config;
pub mod music_theory_extractor;
pub mod music_theory_content;
pub mod music_theory_sources;

pub use config::Config;
pub use music_theory_extractor::{MusicTheoryExtractor, MusicTheoryNodeData, MusicTheoryEdgeData};
pub use music_theory_content::{MusicTheoryContentProvider, ConceptInfo, ConceptDetail};
pub use music_theory_sources::{MusicTheorySourceProvider, SourceInfo};

// Re-export server creation for programmatic use
pub use fabryk_mcp::FabrykMcpServer;
```

### Step 5: Verify all CLI commands

```bash
cd ~/lab/oxur/ecl

# Test version
cargo run -p music-theory-mcp-server -- version

# Test help
cargo run -p music-theory-mcp-server -- --help

# Test graph commands
cargo run -p music-theory-mcp-server -- graph --help
cargo run -p music-theory-mcp-server -- graph build --dry-run
cargo run -p music-theory-mcp-server -- graph stats
cargo run -p music-theory-mcp-server -- graph validate
cargo run -p music-theory-mcp-server -- graph query picardy-third -t related
cargo run -p music-theory-mcp-server -- graph query jazz-harmony -t prerequisites
cargo run -p music-theory-mcp-server -- graph query major-scale -t path --to jazz-harmony

# Test other commands
cargo run -p music-theory-mcp-server -- health
cargo run -p music-theory-mcp-server -- index --check
```

### Step 6: Run full test suite

```bash
cd ~/lab/oxur/ecl

cargo test -p fabryk-cli
cargo test -p music-theory-mcp-server
cargo clippy -p fabryk-cli -p music-theory-mcp-server -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-cli` dependency added to music-theory
- [ ] `MusicTheoryCli` extends `FabrykCli<Config>`
- [ ] Graph build uses `MusicTheoryExtractor`
- [ ] Extracted CLI files removed
- [ ] All CLI commands work:
  - [ ] `--help` shows correct commands
  - [ ] `version` shows correct version
  - [ ] `health` reports status
  - [ ] `graph build` produces correct graph
  - [ ] `graph validate` reports issues
  - [ ] `graph stats` shows statistics
  - [ ] `graph query` works for all query types
- [ ] `cargo test --all-features` passes

## Phase 6 Completion

After this milestone, Phase 6 is complete:

| Feature | Status |
|---------|--------|
| Generic CLI framework | ✓ fabryk-cli |
| Base commands (serve, index, version, health) | ✓ |
| Graph commands (build, validate, stats, query) | ✓ |
| Domain extension via `FabrykCli<C>` | ✓ |
| Music theory CLI integration | ✓ |

**Music-theory now uses Fabryk for CLI infrastructure.**

## Commit Message

```
feat(music-theory): integrate fabryk-cli, Phase 6 complete

Wire fabryk-cli into music-theory:
- MusicTheoryCli extends FabrykCli<Config>
- Graph build uses MusicTheoryExtractor
- Remove extracted CLI files

Verified: All CLI commands functional:
- version, health, serve, index
- graph build/validate/stats/query

Phase 6 complete. CLI infrastructure fully extracted to Fabryk.

Ref: Doc 0013 Phase 6 completion

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
