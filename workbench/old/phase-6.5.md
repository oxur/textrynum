# Phase 6.5: Configuration Infrastructure

## Context

Phase 6 is complete (fabryk-cli with 43 tests, committed as 5efa7c8). The CLI framework has `--config` flag parsing but it's unused — there's no actual config loading, no config file format, and no `config` subcommand. Phase 6.5 adds configuration infrastructure following the taproot pattern: TOML config files loaded via the `confyg` crate with env var overlay, plus a `config` CLI subcommand for init/get/set/path/export operations.

---

## Milestone 6.5.1 — FabrykConfig Struct & Loading

### What to create

**File:** `crates/fabryk-cli/src/config.rs`

### FabrykConfig struct

A TOML-deserializable config struct that also implements `ConfigProvider`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FabrykConfig {
    pub project_name: String,
    pub base_path: Option<String>,
    pub content: ContentConfig,
    pub graph: GraphConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentConfig {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphConfig {
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,          // default 3000
    pub host: String,       // default "127.0.0.1"
}
```

All sub-structs implement `Default` with sensible values.

### Config loading with confyg

```rust
impl FabrykConfig {
    pub fn load(config_path: Option<&str>) -> Result<Self> { ... }
    pub fn resolve_config_path(explicit: Option<&str>) -> Option<PathBuf> { ... }
    pub fn default_config_path() -> Option<PathBuf> { ... }
    pub fn to_toml_string(&self) -> Result<String> { ... }
}
```

**Loading priority** (following taproot pattern):
1. If `--config <path>` provided: use that file
2. Else check env var `FABRYK_CONFIG`
3. Else check XDG default: `~/.config/fabryk/config.toml` (via `dirs::config_dir()`)
4. If no file found: use defaults (all `#[serde(default)]`)

**confyg 0.3 API** (confirmed from source):
```rust
let mut builder = Confygery::new();           // returns Confygery (not Result)
if let Some(path) = resolved_path {
    builder.add_file(path.to_string_lossy()); // returns &mut Confygery
}
let env_opts = env::Options {                 // plain struct, no builder methods
    top_level: "FABRYK".to_string(),
    sections: vec!["content".into(), "graph".into(), "server".into()],
};
builder.add_env(env_opts);                    // returns &mut Confygery
let config: FabrykConfig = builder.build()    // returns Result<T, toml::de::Error>
    .map_err(|e| Error::config(e.to_string()))?;
```

### ConfigProvider implementation

```rust
impl ConfigProvider for FabrykConfig {
    fn project_name(&self) -> &str { &self.project_name }
    fn base_path(&self) -> Result<PathBuf> { ... }      // self.base_path or cwd
    fn content_path(&self, content_type: &str) -> Result<PathBuf> {
        // self.content.path or base_path/content_type
    }
}
```

### Cargo.toml additions

```toml
confyg = { workspace = true }
toml = { workspace = true }
dirs = { workspace = true }
```

### Tests

- `test_fabryk_config_default` — defaults are sensible
- `test_fabryk_config_from_toml` — deserialize from TOML string
- `test_fabryk_config_to_toml` — round-trip serialization
- `test_fabryk_config_load_from_file` — load from temp file
- `test_fabryk_config_load_defaults` — load with no file
- `test_fabryk_config_load_env_overlay` — env vars override file values
- `test_fabryk_config_resolve_config_path_explicit` — --config flag wins
- `test_fabryk_config_resolve_config_path_env` — env var fallback
- `test_fabryk_config_resolve_config_path_default` — XDG default
- `test_fabryk_config_provider_project_name` — ConfigProvider impl
- `test_fabryk_config_provider_base_path` — ConfigProvider impl
- `test_fabryk_config_provider_content_path` — ConfigProvider impl

---

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

## Milestone 6.5.3 — Integration: Wire Config into FabrykCli

### Update `app.rs`

Change `FabrykCli` to optionally hold a loaded `FabrykConfig`:

```rust
pub struct FabrykCli<C: ConfigProvider> {
    name: String,
    config: Arc<C>,
    version: String,
}
```

The existing generic `C: ConfigProvider` approach stays as-is — domain apps can still use their own config type. But we add a convenience constructor:

```rust
impl FabrykCli<FabrykConfig> {
    /// Create from CLI args, loading config from file/env.
    pub fn from_args(name: impl Into<String>, args: &CliArgs) -> Result<Self> {
        let config = FabrykConfig::load(args.config.as_deref())?;
        Ok(Self::new(name, config))
    }
}
```

### Tests

- `test_fabryk_cli_from_args_default` — no config file, uses defaults
- `test_fabryk_cli_from_args_with_file` — loads from temp config file
- `test_fabryk_cli_config_command_dispatch` — end-to-end config path command

---

## Key files to create/modify

| File | Action |
|------|--------|
| `crates/fabryk-cli/Cargo.toml` | Add confyg, toml, dirs deps |
| `crates/fabryk-cli/src/config.rs` | **Create** — FabrykConfig, loading, ConfigProvider impl |
| `crates/fabryk-cli/src/config_handlers.rs` | **Create** — config CLI handlers, TOML helpers |
| `crates/fabryk-cli/src/cli.rs` | **Modify** — add ConfigCommand, ConfigAction |
| `crates/fabryk-cli/src/app.rs` | **Modify** — wire config dispatch, add from_args |
| `crates/fabryk-cli/src/lib.rs` | **Modify** — add modules, re-exports |

### Reference files (patterns to reuse)

- `~/lab/banyan/taproot/crates/taproot-server/src/cli.rs` — ConfigAction enum, handler functions, get_nested_value/set_nested_value/parse_value/format_toml_value helpers, EnvGuard test helper
- `~/lab/banyan/taproot/crates/taproot-server/src/config.rs` — Config::load() with Confygery, resolve_config_path(), to_toml_string(), to_env_vars()
- `~/lab/oxur/confyg/src/conf/confygery.rs` — Confygery::new(), add_file(), add_env(), build()
- `~/lab/oxur/confyg/src/env/options.rs` — env::Options { top_level, sections }
- `crates/fabryk-core/src/traits.rs` — ConfigProvider trait
- `crates/fabryk-core/src/error.rs` — Error::config(), Error::operation()

### Workspace deps to use

- `confyg = "0.3"` (workspace)
- `toml = "0.8"` (workspace)
- `dirs = "5"` (workspace)

---

## Verification

```bash
cargo check -p fabryk-cli
cargo test -p fabryk-cli
cargo clippy -p fabryk-cli -- -D warnings
cargo fmt --check -p fabryk-cli
cargo test --workspace  # no regressions
```

---

## Out of scope

- Domain-specific config (music-theory fields, etc.) — belongs in domain repos
- Config file watching / hot reload
- Config migration between versions
