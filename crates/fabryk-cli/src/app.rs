//! FabrykCli application framework.
//!
//! Provides the generic CLI application that domain crates instantiate
//! with their own [`ConfigProvider`] implementation.

use crate::cli::{BaseCommand, CliArgs, GraphSubcommand};
use crate::config::FabrykConfig;
use crate::{config_handlers, graph_handlers};
use fabryk_core::Result;
use fabryk_core::traits::ConfigProvider;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

// ============================================================================
// FabrykCli
// ============================================================================

/// Generic CLI application parameterized over a config provider.
///
/// Domain applications create a `FabrykCli<MyConfig>` and call `run()`.
pub struct FabrykCli<C: ConfigProvider> {
    name: String,
    config: Arc<C>,
    version: String,
}

impl FabrykCli<FabrykConfig> {
    /// Create from CLI args, loading config from file/env.
    pub fn from_args(name: impl Into<String>, args: &CliArgs) -> Result<Self> {
        let config = FabrykConfig::load(args.config.as_deref())?;
        Ok(Self::new(name, config))
    }
}

impl<C: ConfigProvider> FabrykCli<C> {
    /// Create a new CLI application.
    pub fn new(name: impl Into<String>, config: C) -> Self {
        Self {
            name: name.into(),
            config: Arc::new(config),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Override the version string.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Get a reference to the config provider.
    pub fn config(&self) -> &C {
        &self.config
    }

    /// Initialise tracing-based logging.
    ///
    /// Uses `RUST_LOG` env var if set, otherwise defaults based on verbosity flags.
    pub fn init_logging(&self, verbose: bool, quiet: bool) {
        let filter = if std::env::var("RUST_LOG").is_ok() {
            EnvFilter::from_default_env()
        } else if quiet {
            EnvFilter::new("warn")
        } else if verbose {
            EnvFilter::new("debug")
        } else {
            EnvFilter::new("info")
        };

        // Ignore error if a subscriber is already set (e.g. in tests).
        let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    }

    /// Run the CLI with the given arguments.
    pub async fn run(&self, args: CliArgs) -> Result<()> {
        self.init_logging(args.verbose, args.quiet);

        match args.command {
            Some(BaseCommand::Version) => {
                println!("{} {}", self.name, self.version);
                Ok(())
            }
            Some(BaseCommand::Health) => {
                println!("{}: healthy", self.name);
                Ok(())
            }
            Some(BaseCommand::Serve { port }) => {
                println!("Starting {} server on port {}...", self.name, port);
                // Placeholder — domain applications override serve behaviour.
                Ok(())
            }
            Some(BaseCommand::Index { force, check }) => {
                if check {
                    println!("Checking index freshness...");
                } else {
                    println!("Building index{}...", if force { " (forced)" } else { "" });
                }
                // Placeholder — domain applications override index behaviour.
                Ok(())
            }
            Some(BaseCommand::Graph(graph_cmd)) => self.handle_graph(graph_cmd.command).await,
            Some(BaseCommand::Config(config_cmd)) => {
                config_handlers::handle_config_command(args.config.as_deref(), config_cmd.command)
            }
            Some(BaseCommand::Sources(_)) => {
                println!(
                    "Sources commands require domain-specific configuration. \
                     Override run() in your domain CLI to call \
                     sources_handlers::handle_sources()."
                );
                Ok(())
            }
            #[cfg(feature = "vector-fastembed")]
            Some(BaseCommand::Vectordb(cmd)) => {
                crate::vectordb_handlers::handle_vectordb_command(cmd.command)
            }
            None => {
                println!("{} {} — use --help for usage", self.name, self.version);
                Ok(())
            }
        }
    }

    /// Dispatch graph subcommands to handlers.
    async fn handle_graph(&self, command: GraphSubcommand) -> Result<()> {
        match command {
            GraphSubcommand::Build {
                output: _,
                dry_run: _,
            } => {
                // Build requires a domain-specific GraphExtractor, so we print
                // a message indicating that the domain application should override.
                println!(
                    "Graph build requires a domain-specific extractor. \
                     Override handle_graph() in your domain CLI."
                );
                Ok(())
            }
            GraphSubcommand::Validate => graph_handlers::handle_validate(&*self.config).await,
            GraphSubcommand::Stats => graph_handlers::handle_stats(&*self.config).await,
            GraphSubcommand::Query { id, query_type, to } => {
                let options = graph_handlers::QueryOptions { id, query_type, to };
                graph_handlers::handle_query(&*self.config, options).await
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CliArgs;
    use clap::Parser;
    use std::path::PathBuf;

    #[derive(Clone)]
    struct TestConfig {
        base: PathBuf,
    }

    impl ConfigProvider for TestConfig {
        fn project_name(&self) -> &str {
            "test-app"
        }

        fn base_path(&self) -> Result<PathBuf> {
            Ok(self.base.clone())
        }

        fn content_path(&self, content_type: &str) -> Result<PathBuf> {
            Ok(self.base.join(content_type))
        }
    }

    fn test_config() -> TestConfig {
        TestConfig {
            base: PathBuf::from("/tmp/test"),
        }
    }

    #[test]
    fn test_fabryk_cli_new() {
        let cli = FabrykCli::new("my-app", test_config());
        assert_eq!(cli.name, "my-app");
        assert_eq!(cli.config().project_name(), "test-app");
    }

    #[test]
    fn test_fabryk_cli_with_version() {
        let cli = FabrykCli::new("my-app", test_config()).with_version("1.2.3");
        assert_eq!(cli.version, "1.2.3");
    }

    #[test]
    fn test_fabryk_cli_config_access() {
        let cli = FabrykCli::new("app", test_config());
        assert_eq!(
            cli.config().base_path().unwrap(),
            PathBuf::from("/tmp/test")
        );
    }

    #[tokio::test]
    async fn test_run_version_command() {
        let cli = FabrykCli::new("test-app", test_config()).with_version("0.1.0");
        let args = CliArgs::parse_from(["test", "version"]);
        let result = cli.run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_health_command() {
        let cli = FabrykCli::new("test-app", test_config());
        let args = CliArgs::parse_from(["test", "health"]);
        let result = cli.run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_no_command() {
        let cli = FabrykCli::new("test-app", test_config()).with_version("0.1.0");
        let args = CliArgs::parse_from(["test"]);
        let result = cli.run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_serve_command() {
        let cli = FabrykCli::new("test-app", test_config());
        let args = CliArgs::parse_from(["test", "serve", "--port", "9090"]);
        let result = cli.run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_index_command() {
        let cli = FabrykCli::new("test-app", test_config());
        let args = CliArgs::parse_from(["test", "index"]);
        let result = cli.run(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_index_check() {
        let cli = FabrykCli::new("test-app", test_config());
        let args = CliArgs::parse_from(["test", "index", "--check"]);
        let result = cli.run(args).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_logging_default() {
        let cli = FabrykCli::new("test", test_config());
        // Should not panic
        cli.init_logging(false, false);
    }

    #[test]
    fn test_init_logging_verbose() {
        let cli = FabrykCli::new("test", test_config());
        cli.init_logging(true, false);
    }

    #[test]
    fn test_init_logging_quiet() {
        let cli = FabrykCli::new("test", test_config());
        cli.init_logging(false, true);
    }

    // ------------------------------------------------------------------------
    // FabrykConfig integration tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_cli_from_args_default() {
        let args = CliArgs::parse_from(["test"]);
        let cli = FabrykCli::from_args("test-app", &args).unwrap();
        assert_eq!(cli.config().project_name(), "fabryk");
    }

    #[test]
    fn test_fabryk_cli_from_args_with_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
                project_name = "from-file"
                [server]
                port = 9090
            "#,
        )
        .unwrap();

        let args = CliArgs::parse_from(["test", "--config", path.to_str().unwrap()]);
        let cli = FabrykCli::from_args("test-app", &args).unwrap();
        assert_eq!(cli.config().project_name(), "from-file");
    }

    #[tokio::test]
    async fn test_fabryk_cli_config_command_dispatch() {
        let cli = FabrykCli::new("test-app", test_config());
        let args = CliArgs::parse_from(["test", "config", "path"]);
        let result = cli.run(args).await;
        assert!(result.is_ok());
    }
}
