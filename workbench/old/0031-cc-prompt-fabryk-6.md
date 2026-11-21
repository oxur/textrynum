---
title: "CC Prompt: Fabryk 6.1 — CLI Framework"
milestone: "6.1"
phase: 6
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["Phase 5 complete"]
governing-docs: [0011-audit §4.10, 0013-project-plan]
---

# CC Prompt: Fabryk 6.1 — CLI Framework

## Context

Phase 6 extracts the CLI framework to `fabryk-cli`. This enables domains to
share common CLI patterns while adding domain-specific subcommands.

The CLI framework provides:
- Common commands (serve, index, version)
- Graph commands (build, validate, stats)
- Extensible subcommand registration

## Objective

Create `fabryk-cli` crate with:

1. `FabrykCli<C: ConfigProvider>` - Generic CLI application
2. Common subcommands (serve, index, version, health)
3. Subcommand registration mechanism for domain-specific commands

## Implementation Steps

### Step 1: Create fabryk-cli crate

```bash
cd ~/lab/oxur/ecl/crates
mkdir -p fabryk-cli/src
```

Create `fabryk-cli/Cargo.toml`:

```toml
[package]
name = "fabryk-cli"
version = "0.1.0"
edition = "2021"
description = "CLI framework for Fabryk domains"
license = "Apache-2.0"

[dependencies]
fabryk-core = { path = "../fabryk-core" }

clap = { version = "4.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tempfile = "3.0"
```

### Step 2: Define CLI structure

Create `fabryk-cli/src/cli.rs`:

```rust
//! CLI argument parsing and command definitions.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Base CLI arguments shared by all Fabryk applications.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable quiet mode (errors only)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<BaseCommand>,
}

/// Base commands available in all Fabryk applications.
#[derive(Subcommand, Debug)]
pub enum BaseCommand {
    /// Start the MCP server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// Build or rebuild the search index
    Index {
        /// Force full reindex
        #[arg(short, long)]
        force: bool,

        /// Only check if index is stale
        #[arg(long)]
        check: bool,
    },

    /// Print version information
    Version,

    /// Check application health
    Health,

    /// Graph operations
    #[command(subcommand)]
    Graph(GraphCommand),
}

/// Graph-related subcommands.
#[derive(Subcommand, Debug)]
pub enum GraphCommand {
    /// Build the knowledge graph
    Build {
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Dry run - show what would be built
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate graph structure
    Validate,

    /// Show graph statistics
    Stats,

    /// Query the graph
    Query {
        /// Node ID to query
        id: String,

        /// Query type (related, prerequisites, path)
        #[arg(short = 't', long, default_value = "related")]
        query_type: String,

        /// Target node for path queries
        #[arg(long)]
        to: Option<String>,
    },
}

/// Trait for domain-specific CLI extensions.
///
/// Domains implement this to add custom subcommands.
pub trait CliExtension: Send + Sync {
    /// Extended command type (should be a clap Subcommand).
    type Command: clap::Subcommand + Send + Sync;

    /// Handle a domain-specific command.
    fn handle_command(
        &self,
        command: &Self::Command,
    ) -> impl std::future::Future<Output = fabryk_core::Result<()>> + Send;
}
```

### Step 3: Create CLI application

Create `fabryk-cli/src/app.rs`:

```rust
//! CLI application framework.

use crate::cli::{BaseCommand, CliArgs, GraphCommand};
use fabryk_core::traits::ConfigProvider;
use fabryk_core::Result;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Generic CLI application for Fabryk domains.
///
/// # Type Parameters
///
/// - `C`: Domain configuration type implementing `ConfigProvider`
///
/// # Example
///
/// ```rust,ignore
/// let cli = FabrykCli::new("music-theory", config);
/// cli.run().await?;
/// ```
pub struct FabrykCli<C: ConfigProvider> {
    name: String,
    config: Arc<C>,
    version: String,
}

impl<C: ConfigProvider + 'static> FabrykCli<C> {
    /// Create a new CLI application.
    pub fn new(name: impl Into<String>, config: C) -> Self {
        Self {
            name: name.into(),
            config: Arc::new(config),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Set the version string.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Get the configuration.
    pub fn config(&self) -> &C {
        &self.config
    }

    /// Initialize logging based on CLI args.
    pub fn init_logging(&self, args: &CliArgs) {
        let level = if args.quiet {
            Level::ERROR
        } else if args.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        };

        let subscriber = FmtSubscriber::builder()
            .with_max_level(level)
            .with_target(false)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }

    /// Run the CLI with base commands only.
    pub async fn run(self, args: CliArgs) -> Result<()> {
        self.init_logging(&args);

        match args.command {
            Some(BaseCommand::Serve { port }) => {
                self.handle_serve(port).await
            }
            Some(BaseCommand::Index { force, check }) => {
                self.handle_index(force, check).await
            }
            Some(BaseCommand::Version) => {
                self.handle_version()
            }
            Some(BaseCommand::Health) => {
                self.handle_health().await
            }
            Some(BaseCommand::Graph(cmd)) => {
                self.handle_graph(cmd).await
            }
            None => {
                // No command - show help
                println!("Use --help for usage information");
                Ok(())
            }
        }
    }

    /// Handle the serve command.
    async fn handle_serve(&self, port: u16) -> Result<()> {
        info!("Starting {} server on port {}", self.name, port);
        // Actual server implementation would go here
        // For now, just a placeholder
        println!("Server would start on port {}. Press Ctrl+C to stop.", port);
        tokio::signal::ctrl_c().await.map_err(|e| {
            fabryk_core::Error::operation(format!("Signal error: {}", e))
        })?;
        Ok(())
    }

    /// Handle the index command.
    async fn handle_index(&self, force: bool, check: bool) -> Result<()> {
        if check {
            info!("Checking index freshness...");
            // Check implementation
            println!("Index check: (implementation needed)");
        } else {
            info!("Building search index (force={})", force);
            // Index build implementation
            println!("Index build: (implementation needed)");
        }
        Ok(())
    }

    /// Handle the version command.
    fn handle_version(&self) -> Result<()> {
        println!("{} v{}", self.name, self.version);
        println!("fabryk-cli v{}", env!("CARGO_PKG_VERSION"));
        Ok(())
    }

    /// Handle the health command.
    async fn handle_health(&self) -> Result<()> {
        println!("Health check:");
        println!("  Project: {}", self.config.project_name());
        println!("  Status: OK");
        Ok(())
    }

    /// Handle graph subcommands.
    async fn handle_graph(&self, cmd: GraphCommand) -> Result<()> {
        match cmd {
            GraphCommand::Build { output, dry_run } => {
                info!("Building graph (dry_run={})", dry_run);
                if let Some(path) = output {
                    info!("Output: {}", path.display());
                }
                println!("Graph build: (implementation via GraphBuilder)");
                Ok(())
            }
            GraphCommand::Validate => {
                info!("Validating graph...");
                println!("Graph validation: (implementation via validate_graph)");
                Ok(())
            }
            GraphCommand::Stats => {
                info!("Computing graph statistics...");
                println!("Graph stats: (implementation via compute_stats)");
                Ok(())
            }
            GraphCommand::Query { id, query_type, to } => {
                info!("Querying graph: {} (type={})", id, query_type);
                if let Some(target) = to {
                    info!("Target: {}", target);
                }
                println!("Graph query: (implementation via fabryk-graph)");
                Ok(())
            }
        }
    }
}
```

### Step 4: Create lib.rs

Create `fabryk-cli/src/lib.rs`:

```rust
//! CLI framework for Fabryk domains.
//!
//! Provides a generic CLI application that can be extended by domains.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_cli::{FabrykCli, CliArgs};
//! use clap::Parser;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let args = CliArgs::parse();
//!     let config = MyConfig::load()?;
//!
//!     FabrykCli::new("my-domain", config)
//!         .run(args)
//!         .await
//! }
//! ```

pub mod app;
pub mod cli;

pub use app::FabrykCli;
pub use cli::{BaseCommand, CliArgs, CliExtension, GraphCommand};
```

### Step 5: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-cli
cargo test -p fabryk-cli
cargo clippy -p fabryk-cli -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-cli` crate created
- [ ] `CliArgs` with global options (config, verbose, quiet)
- [ ] `BaseCommand` enum with serve, index, version, health, graph
- [ ] `GraphCommand` enum with build, validate, stats, query
- [ ] `FabrykCli<C: ConfigProvider>` generic application
- [ ] `CliExtension` trait for domain-specific commands
- [ ] Logging initialization based on verbosity flags
- [ ] All tests pass

## Commit Message

```
feat(cli): add fabryk-cli with CLI framework

Add CLI framework for Fabryk domains:
- CliArgs with global options (config, verbose, quiet)
- BaseCommand: serve, index, version, health, graph
- GraphCommand: build, validate, stats, query
- FabrykCli<C: ConfigProvider> generic application
- CliExtension trait for domain customization

Phase 6 milestone 6.1 of Fabryk extraction.

Ref: Doc 0011 §4.10 (CLI framework)
Ref: Doc 0013 Phase 6

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
