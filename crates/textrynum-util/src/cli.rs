//! CLI argument parsing and command definitions.

use clap::{Parser, Subcommand, ValueEnum};

/// Textrynum workspace utility.
#[derive(Parser, Debug)]
#[command(
    name = "textyl",
    author,
    version,
    about = "Textrynum workspace utility"
)]
pub struct Args {
    /// Working directory (defaults to current directory).
    #[arg(long, global = true)]
    pub workspace_root: Option<String>,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// List workspace crates and their internal dependency versions.
    Crates(CratesArgs),

    /// Operations on a single crate.
    Crate(CrateArgs),

    /// Set version and sync dependencies.
    ///
    /// With a positional version: sets both project and dep versions (workspace mode).
    /// With --project-version / --deps-version: can target each independently.
    /// Works in any Rust project (workspace or single crate).
    SetVersion {
        /// Version for both project and deps (shorthand for existing behavior).
        version: Option<String>,

        /// Project's own version (workspace.package.version or package.version).
        #[arg(long)]
        project_version: Option<String>,

        /// Version to set on all fabryk-*/ecl-* dependencies.
        #[arg(long)]
        deps_version: Option<String>,

        /// Only check for mismatches; do not write. Exit non-zero if mismatches found.
        #[arg(long)]
        check: bool,
    },

    /// Check that all internal dep versions match the workspace version.
    CheckVersions,
}

/// Arguments for the `crates` command.
#[derive(clap::Args, Debug)]
pub struct CratesArgs {
    /// Output format.
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,

    /// Print publishable crates in dependency order (space-separated).
    #[arg(long)]
    pub publish_order: bool,

    /// Comma-separated list of crate names to exclude from output.
    #[arg(long, value_delimiter = ',')]
    pub exclude: Vec<String>,

    /// Subcommand (e.g., update).
    #[command(subcommand)]
    pub action: Option<CratesAction>,
}

/// Subcommands for `crates`.
#[derive(Subcommand, Debug)]
pub enum CratesAction {
    /// Apply version updates from JSON on stdin or --data argument.
    Update {
        /// JSON data string. If omitted, reads from stdin.
        #[arg(long)]
        data: Option<String>,
    },
}

/// Arguments for the `crate` command.
#[derive(clap::Args, Debug)]
pub struct CrateArgs {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub action: CrateAction,
}

/// Subcommands for `crate`.
#[derive(Subcommand, Debug)]
pub enum CrateAction {
    /// Update a dependency version for a specific crate.
    Update {
        /// Crate whose Cargo.toml to modify.
        crate_name: String,

        /// Dependency name to update.
        #[arg(long)]
        dep: Option<String>,

        /// New version string.
        #[arg(long)]
        version: Option<String>,

        /// JSON data for bulk update of deps.
        #[arg(long)]
        data: Option<String>,
    },
}

/// Output format for listing commands.
#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Text,
    Json,
    Toml,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_crates_default_format() {
        let args = Args::parse_from(["textyl", "crates"]);
        match args.command {
            Command::Crates(crates_args) => {
                assert!(matches!(crates_args.format, OutputFormat::Text));
                assert!(crates_args.action.is_none());
            }
            _ => panic!("Expected Crates command"),
        }
    }

    #[test]
    fn test_args_crates_publish_order() {
        let args = Args::parse_from(["textyl", "crates", "--publish-order"]);
        match args.command {
            Command::Crates(crates_args) => {
                assert!(crates_args.publish_order);
                assert!(crates_args.action.is_none());
            }
            _ => panic!("Expected Crates command"),
        }
    }

    #[test]
    fn test_args_crates_publish_order_with_exclude() {
        let args = Args::parse_from(["textyl", "crates", "--publish-order", "--exclude=foo,bar"]);
        match args.command {
            Command::Crates(crates_args) => {
                assert!(crates_args.publish_order);
                assert_eq!(crates_args.exclude, vec!["foo", "bar"]);
            }
            _ => panic!("Expected Crates command"),
        }
    }

    #[test]
    fn test_args_crates_json_format() {
        let args = Args::parse_from(["textyl", "crates", "--format", "json"]);
        match args.command {
            Command::Crates(crates_args) => {
                assert!(matches!(crates_args.format, OutputFormat::Json));
            }
            _ => panic!("Expected Crates command"),
        }
    }

    #[test]
    fn test_args_crates_update_with_data() {
        let args = Args::parse_from(["textyl", "crates", "update", "--data", "{}"]);
        match args.command {
            Command::Crates(crates_args) => match crates_args.action {
                Some(CratesAction::Update { data }) => {
                    assert_eq!(data, Some("{}".to_string()));
                }
                _ => panic!("Expected Update action"),
            },
            _ => panic!("Expected Crates command"),
        }
    }

    #[test]
    fn test_args_crate_update_with_flags() {
        let args = Args::parse_from([
            "textyl",
            "crate",
            "update",
            "fabryk-core",
            "--dep",
            "alpha",
            "--version",
            "0.2.0",
        ]);
        match args.command {
            Command::Crate(crate_args) => match crate_args.action {
                CrateAction::Update {
                    crate_name,
                    dep,
                    version,
                    data,
                } => {
                    assert_eq!(crate_name, "fabryk-core");
                    assert_eq!(dep, Some("alpha".to_string()));
                    assert_eq!(version, Some("0.2.0".to_string()));
                    assert!(data.is_none());
                }
            },
            _ => panic!("Expected Crate command"),
        }
    }

    #[test]
    fn test_args_crate_update_with_json_data() {
        let args = Args::parse_from(["textyl", "crate", "update", "fabryk-core", "--data", "{}"]);
        match args.command {
            Command::Crate(crate_args) => match crate_args.action {
                CrateAction::Update {
                    crate_name, data, ..
                } => {
                    assert_eq!(crate_name, "fabryk-core");
                    assert_eq!(data, Some("{}".to_string()));
                }
            },
            _ => panic!("Expected Crate command"),
        }
    }

    #[test]
    fn test_args_set_version_positional() {
        let args = Args::parse_from(["textyl", "set-version", "0.2.0"]);
        match args.command {
            Command::SetVersion { version, check, .. } => {
                assert_eq!(version, Some("0.2.0".to_string()));
                assert!(!check);
            }
            _ => panic!("Expected SetVersion command"),
        }
    }

    #[test]
    fn test_args_set_version_check_mode() {
        let args = Args::parse_from(["textyl", "set-version", "0.2.0", "--check"]);
        match args.command {
            Command::SetVersion { version, check, .. } => {
                assert_eq!(version, Some("0.2.0".to_string()));
                assert!(check);
            }
            _ => panic!("Expected SetVersion command"),
        }
    }

    #[test]
    fn test_args_set_version_project_and_deps_flags() {
        let args = Args::parse_from([
            "textyl",
            "set-version",
            "--project-version",
            "2.0.0",
            "--deps-version",
            "0.1.2",
        ]);
        match args.command {
            Command::SetVersion {
                version,
                project_version,
                deps_version,
                check,
            } => {
                assert!(version.is_none());
                assert_eq!(project_version, Some("2.0.0".to_string()));
                assert_eq!(deps_version, Some("0.1.2".to_string()));
                assert!(!check);
            }
            _ => panic!("Expected SetVersion command"),
        }
    }

    #[test]
    fn test_args_set_version_deps_only() {
        let args = Args::parse_from(["textyl", "set-version", "--deps-version", "0.1.2"]);
        match args.command {
            Command::SetVersion {
                version,
                project_version,
                deps_version,
                ..
            } => {
                assert!(version.is_none());
                assert!(project_version.is_none());
                assert_eq!(deps_version, Some("0.1.2".to_string()));
            }
            _ => panic!("Expected SetVersion command"),
        }
    }

    #[test]
    fn test_args_set_version_positional_with_deps_override() {
        let args = Args::parse_from(["textyl", "set-version", "0.2.0", "--deps-version", "0.1.2"]);
        match args.command {
            Command::SetVersion {
                version,
                deps_version,
                ..
            } => {
                assert_eq!(version, Some("0.2.0".to_string()));
                assert_eq!(deps_version, Some("0.1.2".to_string()));
            }
            _ => panic!("Expected SetVersion command"),
        }
    }

    #[test]
    fn test_args_check_versions() {
        let args = Args::parse_from(["textyl", "check-versions"]);
        assert!(matches!(args.command, Command::CheckVersions));
    }

    #[test]
    fn test_args_workspace_root_global() {
        let args = Args::parse_from(["textyl", "--workspace-root", "/tmp/ws", "check-versions"]);
        assert_eq!(args.workspace_root, Some("/tmp/ws".to_string()));
    }
}
