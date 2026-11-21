# Milestone 6.5.2 — Config CLI Commands

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0041-phase-6.5-config-infra.md, lines 121-249. Original bytes are recoverable from import commit 3a39aea. -->

## Milestone 6.5.2 — Config CLI Commands

### What to create

**File:** `crates/fabryk-cli/src/config_handlers.rs`

### CLI additions in `cli.rs`

Add `Config(ConfigCommand)` variant to `BaseCommand`:

```rust
pub enum BaseCommand {
    // ... existing variants ...
    /// Configuration operations.
    Config(ConfigCommand),
}

#[derive(Parser, Debug)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show the resolved config file path.
    Path,
    /// Get a configuration value by dotted key.
    Get {
        /// Dotted key (e.g., "server.port").
        key: String,
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
    },
}
```

### Handler functions in `config_handlers.rs`

Following taproot pattern from `~/lab/banyan/taproot/crates/taproot-server/src/cli.rs`:

```
handle_config_command(config, action) -> Result<()>
  dispatches to:
    cmd_config_path(config_path: Option<&str>) -> Result<()>
    cmd_config_get(config_path: Option<&str>, key: &str) -> Result<()>
    cmd_config_set(config_path: Option<&str>, key: &str, value: &str) -> Result<()>
    cmd_config_init(file: Option<&str>, force: bool) -> Result<()>
    cmd_config_export(config: &FabrykConfig, docker_env: bool) -> Result<()>
```

**TOML dotted-key helpers** (following taproot's get_nested_value/set_nested_value/parse_value/format_toml_value):

```
get_nested_value(table: &toml::Value, key: &str) -> Option<&toml::Value>
set_nested_value(table: &mut toml::Value, key: &str, value: toml::Value) -> Result<()>
parse_value(s: &str) -> toml::Value    // auto-detect bool/int/float/string
format_toml_value(value: &toml::Value) -> String
```

### Wire into app.rs

Add config command dispatch in `FabrykCli::run()`:

```rust
Some(BaseCommand::Config(config_cmd)) => {
    config_handlers::handle_config_command(
        args.config.as_deref(),
        config_cmd.command,
    ).await
}
```

Note: config commands receive the raw `--config` path, not a loaded config, because some commands (path, init) work before a config file exists.

### Update lib.rs

Add `pub mod config` and `pub mod config_handlers`. Re-export `FabrykConfig`, `ConfigCommand`, `ConfigAction`.

### Tests

**config_handlers.rs:**
- `test_cmd_config_path_default` — shows default path
- `test_cmd_config_path_explicit` — shows --config path
- `test_cmd_config_get_simple_key` — e.g., "project_name"
- `test_cmd_config_get_nested_key` — e.g., "server.port"
- `test_cmd_config_get_missing_key` — error for unknown key
- `test_cmd_config_set_simple_key` — set top-level value
- `test_cmd_config_set_nested_key` — set "server.port" value
- `test_cmd_config_init_creates_file` — creates default config
- `test_cmd_config_init_no_overwrite` — errors without --force
- `test_cmd_config_init_force_overwrites` — overwrites with --force
- `test_cmd_config_export_env_vars` — prints FABRYK_ env vars
- `test_cmd_config_export_docker_env` — prints --env flags
- `test_get_nested_value` — dotted-key traversal
- `test_set_nested_value` — dotted-key mutation
- `test_parse_value_types` — bool/int/float/string detection
- `test_format_toml_value` — value formatting

**cli.rs additions:**
- `test_config_path_command` — parse "config path"
- `test_config_get_command` — parse "config get server.port"
- `test_config_set_command` — parse "config set server.port 8080"
- `test_config_init_command` — parse "config init"
- `test_config_init_force` — parse "config init --force"
- `test_config_export_command` — parse "config export"

---

