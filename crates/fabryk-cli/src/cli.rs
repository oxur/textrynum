//! CLI argument parsing and command definitions.
//!
//! Provides the common CLI structure that all Fabryk-based applications share:
//! configuration, verbosity, and base commands (serve, index, version, health, graph).
//!
//! Domain applications extend this via the [`CliExtension`] trait.

use clap::{Parser, Subcommand};

use crate::sources_handlers::SourcesCommand;

// ============================================================================
// CLI argument types
// ============================================================================

/// Top-level CLI arguments for Fabryk applications.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
pub struct CliArgs {
    /// Path to configuration file.
    #[arg(short, long, env = "FABRYK_CONFIG")]
    pub config: Option<String>,

    /// Enable verbose output.
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress non-essential output.
    #[arg(short, long)]
    pub quiet: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Option<BaseCommand>,
}

/// Built-in commands shared by all Fabryk applications.
#[derive(Subcommand, Debug)]
pub enum BaseCommand {
    /// Start the MCP server.
    Serve {
        /// Port to listen on.
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// Build or refresh the content index.
    Index {
        /// Force full re-index.
        #[arg(short, long)]
        force: bool,

        /// Check index freshness without rebuilding.
        #[arg(long)]
        check: bool,
    },

    /// Print version information.
    Version,

    /// Check system health.
    Health,

    /// Graph operations.
    Graph(GraphCommand),

    /// Configuration operations.
    Config(ConfigCommand),

    /// Source management operations.
    Sources(SourcesCommand),

    /// Vector database operations.
    #[cfg(feature = "vector-fastembed")]
    Vectordb(VectordbCommand),
}

/// Config-specific subcommands.
#[derive(Parser, Debug)]
pub struct ConfigCommand {
    /// Config subcommand to execute.
    #[command(subcommand)]
    pub command: ConfigAction,
}

/// Available config subcommands.
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show the resolved config file path.
    Path,

    /// Show the full configuration, or a specific value by dotted key.
    Get {
        /// Dotted key (e.g., "server.port"). Omit to show full config as TOML.
        key: Option<String>,
    },

    /// Set a configuration value by dotted key.
    Set {
        /// Dotted key (e.g., "server.port").
        key: String,

        /// Value to set.
        value: String,
    },

    /// Create a default configuration file.
    Init {
        /// Output file path (defaults to XDG config path).
        #[arg(short, long)]
        file: Option<String>,

        /// Overwrite existing file.
        #[arg(long)]
        force: bool,
    },

    /// Export configuration as environment variables.
    Export {
        /// Format as Docker --env flags.
        #[arg(long)]
        docker_env: bool,

        /// Write output to a file (e.g., ".env") instead of stdout.
        #[arg(long)]
        file: Option<String>,
    },
}

/// Graph-specific subcommands.
#[derive(Parser, Debug)]
pub struct GraphCommand {
    /// Graph subcommand to execute.
    #[command(subcommand)]
    pub command: GraphSubcommand,
}

/// Available graph subcommands.
#[derive(Subcommand, Debug)]
pub enum GraphSubcommand {
    /// Build the knowledge graph from content.
    Build {
        /// Output file path for the graph.
        #[arg(short, long)]
        output: Option<String>,

        /// Show what would be built without writing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate graph integrity.
    Validate,

    /// Show graph statistics.
    Stats,

    /// Query the graph.
    Query {
        /// Node ID to query.
        #[arg(short, long)]
        id: String,

        /// Type of query: related, prerequisites, path.
        #[arg(short = 't', long, default_value = "related")]
        query_type: String,

        /// Target node ID (for path queries).
        #[arg(long)]
        to: Option<String>,
    },
}

// ============================================================================
// Vectordb commands (feature-gated)
// ============================================================================

/// Vectordb-specific subcommands.
#[cfg(feature = "vector-fastembed")]
#[derive(Parser, Debug)]
pub struct VectordbCommand {
    /// Vectordb subcommand to execute.
    #[command(subcommand)]
    pub command: VectordbAction,
}

/// Available vectordb subcommands.
#[cfg(feature = "vector-fastembed")]
#[derive(Subcommand, Debug)]
pub enum VectordbAction {
    /// Download and cache the embedding model.
    GetModel {
        /// Embedding model name (e.g., "bge-small-en-v1.5").
        #[arg(long)]
        model: Option<String>,

        /// Directory to cache the downloaded model.
        #[arg(long)]
        cache_dir: Option<String>,
    },
}

// ============================================================================
// CliExtension trait
// ============================================================================

/// Extension point for domain-specific CLI commands.
///
/// Domain applications implement this trait to add custom subcommands
/// beyond the built-in base commands.
pub trait CliExtension: Send + Sync {
    /// The domain-specific command type.
    type Command: Send + Sync;

    /// Handle a domain-specific command.
    fn handle_command(
        &self,
        command: Self::Command,
    ) -> impl std::future::Future<Output = fabryk_core::Result<()>> + Send;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_args_default() {
        let args = CliArgs::parse_from(["test"]);
        assert!(args.config.is_none());
        assert!(!args.verbose);
        assert!(!args.quiet);
        assert!(args.command.is_none());
    }

    #[test]
    fn test_cli_args_verbose() {
        let args = CliArgs::parse_from(["test", "--verbose"]);
        assert!(args.verbose);
        assert!(!args.quiet);
    }

    #[test]
    fn test_cli_args_quiet() {
        let args = CliArgs::parse_from(["test", "--quiet"]);
        assert!(!args.verbose);
        assert!(args.quiet);
    }

    #[test]
    fn test_cli_args_config() {
        let args = CliArgs::parse_from(["test", "--config", "/path/to/config.toml"]);
        assert_eq!(args.config, Some("/path/to/config.toml".to_string()));
    }

    #[test]
    fn test_serve_command() {
        let args = CliArgs::parse_from(["test", "serve"]);
        match args.command {
            Some(BaseCommand::Serve { port }) => assert_eq!(port, 3000),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_serve_command_custom_port() {
        let args = CliArgs::parse_from(["test", "serve", "--port", "8080"]);
        match args.command {
            Some(BaseCommand::Serve { port }) => assert_eq!(port, 8080),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_index_command() {
        let args = CliArgs::parse_from(["test", "index"]);
        match args.command {
            Some(BaseCommand::Index { force, check }) => {
                assert!(!force);
                assert!(!check);
            }
            _ => panic!("Expected Index command"),
        }
    }

    #[test]
    fn test_index_command_force() {
        let args = CliArgs::parse_from(["test", "index", "--force"]);
        match args.command {
            Some(BaseCommand::Index { force, check }) => {
                assert!(force);
                assert!(!check);
            }
            _ => panic!("Expected Index command with force"),
        }
    }

    #[test]
    fn test_version_command() {
        let args = CliArgs::parse_from(["test", "version"]);
        assert!(matches!(args.command, Some(BaseCommand::Version)));
    }

    #[test]
    fn test_health_command() {
        let args = CliArgs::parse_from(["test", "health"]);
        assert!(matches!(args.command, Some(BaseCommand::Health)));
    }

    #[test]
    fn test_graph_build_command() {
        let args = CliArgs::parse_from(["test", "graph", "build"]);
        match args.command {
            Some(BaseCommand::Graph(GraphCommand {
                command: GraphSubcommand::Build { output, dry_run },
            })) => {
                assert!(output.is_none());
                assert!(!dry_run);
            }
            _ => panic!("Expected Graph Build command"),
        }
    }

    #[test]
    fn test_graph_build_dry_run() {
        let args = CliArgs::parse_from(["test", "graph", "build", "--dry-run"]);
        match args.command {
            Some(BaseCommand::Graph(GraphCommand {
                command: GraphSubcommand::Build { dry_run, .. },
            })) => {
                assert!(dry_run);
            }
            _ => panic!("Expected Graph Build command with dry_run"),
        }
    }

    #[test]
    fn test_graph_validate_command() {
        let args = CliArgs::parse_from(["test", "graph", "validate"]);
        match args.command {
            Some(BaseCommand::Graph(GraphCommand {
                command: GraphSubcommand::Validate,
            })) => {}
            _ => panic!("Expected Graph Validate command"),
        }
    }

    #[test]
    fn test_graph_stats_command() {
        let args = CliArgs::parse_from(["test", "graph", "stats"]);
        match args.command {
            Some(BaseCommand::Graph(GraphCommand {
                command: GraphSubcommand::Stats,
            })) => {}
            _ => panic!("Expected Graph Stats command"),
        }
    }

    #[test]
    fn test_graph_query_command() {
        let args = CliArgs::parse_from(["test", "graph", "query", "--id", "node-1"]);
        match args.command {
            Some(BaseCommand::Graph(GraphCommand {
                command: GraphSubcommand::Query { id, query_type, to },
            })) => {
                assert_eq!(id, "node-1");
                assert_eq!(query_type, "related");
                assert!(to.is_none());
            }
            _ => panic!("Expected Graph Query command"),
        }
    }

    #[test]
    fn test_graph_query_path() {
        let args = CliArgs::parse_from([
            "test",
            "graph",
            "query",
            "--id",
            "a",
            "--query-type",
            "path",
            "--to",
            "b",
        ]);
        match args.command {
            Some(BaseCommand::Graph(GraphCommand {
                command: GraphSubcommand::Query { id, query_type, to },
            })) => {
                assert_eq!(id, "a");
                assert_eq!(query_type, "path");
                assert_eq!(to, Some("b".to_string()));
            }
            _ => panic!("Expected Graph Query path command"),
        }
    }

    // ------------------------------------------------------------------------
    // Config command tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_config_path_command() {
        let args = CliArgs::parse_from(["test", "config", "path"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Path,
            })) => {}
            _ => panic!("Expected Config Path command"),
        }
    }

    #[test]
    fn test_config_get_command() {
        let args = CliArgs::parse_from(["test", "config", "get", "server.port"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Get { key },
            })) => {
                assert_eq!(key, Some("server.port".to_string()));
            }
            _ => panic!("Expected Config Get command"),
        }
    }

    #[test]
    fn test_config_get_no_key_dumps_all() {
        let args = CliArgs::parse_from(["test", "config", "get"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Get { key },
            })) => {
                assert!(key.is_none());
            }
            _ => panic!("Expected Config Get command"),
        }
    }

    #[test]
    fn test_config_set_command() {
        let args = CliArgs::parse_from(["test", "config", "set", "server.port", "8080"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Set { key, value },
            })) => {
                assert_eq!(key, "server.port");
                assert_eq!(value, "8080");
            }
            _ => panic!("Expected Config Set command"),
        }
    }

    #[test]
    fn test_config_init_command() {
        let args = CliArgs::parse_from(["test", "config", "init"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Init { file, force },
            })) => {
                assert!(file.is_none());
                assert!(!force);
            }
            _ => panic!("Expected Config Init command"),
        }
    }

    #[test]
    fn test_config_init_force() {
        let args = CliArgs::parse_from(["test", "config", "init", "--force"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Init { force, .. },
            })) => {
                assert!(force);
            }
            _ => panic!("Expected Config Init command with force"),
        }
    }

    #[test]
    fn test_config_export_command() {
        let args = CliArgs::parse_from(["test", "config", "export"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Export { docker_env, file },
            })) => {
                assert!(!docker_env);
                assert!(file.is_none());
            }
            _ => panic!("Expected Config Export command"),
        }
    }

    #[test]
    fn test_config_export_docker_env() {
        let args = CliArgs::parse_from(["test", "config", "export", "--docker-env"]);
        match args.command {
            Some(BaseCommand::Config(ConfigCommand {
                command: ConfigAction::Export { docker_env, file },
            })) => {
                assert!(docker_env);
                assert!(file.is_none());
            }
            _ => panic!("Expected Config Export command with docker_env"),
        }
    }

    // ------------------------------------------------------------------------
    // Vectordb command tests (feature-gated)
    // ------------------------------------------------------------------------

    #[cfg(feature = "vector-fastembed")]
    #[test]
    fn test_vectordb_get_model_command() {
        use crate::cli::{VectordbAction, VectordbCommand};

        let args = CliArgs::parse_from(["test", "vectordb", "get-model"]);
        match args.command {
            Some(BaseCommand::Vectordb(VectordbCommand {
                command: VectordbAction::GetModel { model, cache_dir },
            })) => {
                assert!(model.is_none());
                assert!(cache_dir.is_none());
            }
            _ => panic!("Expected Vectordb GetModel command"),
        }
    }

    #[cfg(feature = "vector-fastembed")]
    #[test]
    fn test_vectordb_get_model_with_overrides() {
        use crate::cli::{VectordbAction, VectordbCommand};

        let args = CliArgs::parse_from([
            "test",
            "vectordb",
            "get-model",
            "--model",
            "bge-large-en-v1.5",
            "--cache-dir",
            "/tmp/models",
        ]);
        match args.command {
            Some(BaseCommand::Vectordb(VectordbCommand {
                command: VectordbAction::GetModel { model, cache_dir },
            })) => {
                assert_eq!(model.as_deref(), Some("bge-large-en-v1.5"));
                assert_eq!(cache_dir.as_deref(), Some("/tmp/models"));
            }
            _ => panic!("Expected Vectordb GetModel command with overrides"),
        }
    }
}
