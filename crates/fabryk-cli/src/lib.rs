//! CLI framework for Fabryk-based applications.
//!
//! This crate provides a generic CLI structure that domain applications
//! can extend with their own commands.
//!
//! # Key Abstractions
//!
//! - [`FabrykCli<C>`]: Generic CLI parameterized over config provider
//! - [`CliExtension`]: Trait for adding domain-specific subcommands
//! - Built-in graph commands (validate, stats, query)

pub mod app;
pub mod cli;
pub mod config;
pub mod config_handlers;
pub mod config_loader;
pub mod config_sections;
pub mod config_utils;
pub mod graph_handlers;
pub mod sources_handlers;
#[cfg(feature = "vector-fastembed")]
pub mod vectordb_handlers;

// Re-exports — CLI types
pub use cli::{
    BaseCommand, CliArgs, CliExtension, ConfigAction, ConfigCommand, GraphCommand, GraphSubcommand,
};
pub use sources_handlers::SourcesCommand;
#[cfg(feature = "vector-fastembed")]
pub use cli::{VectordbAction, VectordbCommand};

// Re-exports — application
pub use app::FabrykCli;

// Re-exports — configuration
pub use config::FabrykConfig;
pub use config_loader::ConfigLoaderBuilder;
pub use config_sections::{OAuthConfig, TlsConfig};

// Re-exports — graph handler types
pub use graph_handlers::{BuildOptions, QueryOptions};
