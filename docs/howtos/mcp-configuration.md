# MCP Server Configuration

How to set up and manage configuration for Fabryk-based MCP servers.

## Overview

Fabryk provides a complete configuration management system that handles:

- **Loading** from TOML files, environment variables, and defaults (via `confyg`)
- **CLI commands** for inspecting, modifying, initializing, and exporting config
- **Path resolution** that converts relative paths to absolute
- **Environment variable export** for Docker/Cloud Run deployment
- **Reusable config sections** for TLS, OAuth2, and other common concerns

The system is built on two traits in `fabryk-core` and a set of utilities in `fabryk-cli`.

## Architecture

### Core Traits (`fabryk-core`)

**`ConfigProvider`** — Domain-specific config for Fabryk crates:
- `project_name()` — used for env var prefixes and default paths
- `base_path()` — root directory for all project data
- `content_path(content_type)` — path for a specific content type
- `cache_path(cache_type)` — path for caches (default: `{base}/.cache/{type}`)

**`ConfigManager`** — CLI config management operations:
- `load(config_path)` — load from file/env/defaults
- `resolve_config_path(explicit)` — resolve which config file to use
- `default_config_path()` — XDG default path
- `project_name()` — project name for CLI output
- `to_toml_string()` — serialize to TOML
- `to_env_vars()` — export as `(KEY, VALUE)` pairs

### CLI Framework (`fabryk-cli`)

**`ConfigLoaderBuilder`** — Eliminates loading boilerplate:
```rust
let (config, path) = ConfigLoaderBuilder::new("myapp")
    .section("server")
    .section("logging")
    .port_env_override("PORT")
    .build::<MyConfig>(None, MyConfig::resolve_config_path)?;
```

**`ConfigAction`** — Shared CLI subcommands (clap-based):
- `path` — show resolved config file path
- `get [KEY]` — show full config or a specific dotted key
- `set KEY VALUE` — set a value in the config file
- `init [--file PATH] [--force]` — create default config file
- `export [--docker-env] [--file PATH]` — export as env vars

**`config_utils`** — Path resolution and TOML flattening:
- `resolve_base_dir(env_var, config_path)` — determine base directory
- `resolve_path(base, path)` — resolve a single path
- `resolve_opt_path(field, base, name)` — resolve an `Option<String>` in place
- `flatten_toml_value(value, prefix, out)` — TOML → env var pairs

**`config_sections`** — Reusable config structs:
- `TlsConfig` — cert/key path management with validation
- `OAuthConfig` — Google OAuth2 with validation

## Setting Up Configuration

### Step 1: Define Your Config Struct

```rust
use serde::{Deserialize, Serialize};
use fabryk_cli::config_sections::{OAuthConfig, TlsConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_transport")]
    pub transport: String,

    #[serde(default)]
    pub oauth: OAuthConfig,    // from fabryk-cli

    #[serde(default)]
    pub tls: TlsConfig,        // from fabryk-cli

    // Domain-specific sections:
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Computed at load time, not serialized.
    #[serde(skip)]
    pub base_dir: std::path::PathBuf,
}
```

### Step 2: Implement `ConfigManager`

```rust
use fabryk_cli::config_utils::{flatten_toml_value, resolve_base_dir};
use fabryk_cli::ConfigLoaderBuilder;

impl fabryk::core::ConfigManager for Config {
    fn load(config_path: Option<&str>) -> fabryk::core::Result<Self> {
        let loader = ConfigLoaderBuilder::new("myapp")
            .section("oauth")
            .section("tls")
            .section("database")
            .section("logging")
            .port_env_override("PORT");

        let (mut config, resolved_path): (Self, _) =
            loader.build(config_path, |explicit| {
                if let Some(path) = explicit {
                    return Some(std::path::PathBuf::from(path));
                }
                Self::resolve_config_path()
            })?;

        // Cloud Run PORT override
        if let Ok(port) = std::env::var("PORT") {
            if let Ok(p) = port.parse::<u16>() {
                config.port = p;
            }
        }

        // Resolve relative paths
        let base = resolve_base_dir(
            "MYAPP_BASE_DIR",
            resolved_path.as_deref(),
        );
        config.base_dir = base.clone();
        config.resolve_all_paths(&base);

        Ok(config)
    }

    fn resolve_config_path(explicit: Option<&str>) -> Option<std::path::PathBuf> {
        if let Some(path) = explicit {
            return Some(std::path::PathBuf::from(path));
        }
        Self::resolve_config_path()
    }

    fn default_config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|d| d.join("myapp").join("config.toml"))
    }

    fn project_name() -> &'static str {
        "myapp"
    }

    fn to_toml_string(&self) -> fabryk::core::Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| fabryk::core::Error::config(e.to_string()))
    }

    fn to_env_vars(&self) -> fabryk::core::Result<Vec<(String, String)>> {
        let value = toml::Value::try_from(self)
            .map_err(|e| fabryk::core::Error::config(e.to_string()))?;
        let mut vars = Vec::new();
        flatten_toml_value(&value, "MYAPP", &mut vars);
        Ok(vars)
    }
}
```

### Step 3: Wire Up CLI Commands

```rust
use clap::{Parser, Subcommand};
use fabryk_cli::{ConfigAction, VectordbAction};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    Vectordb {
        #[command(subcommand)]
        action: VectordbAction,
    },
}

pub fn handle_config_command(action: Option<ConfigAction>) -> anyhow::Result<()> {
    match action {
        None => fabryk_cli::config_handlers::cmd_config_path::<Config>(None)
            .map_err(|e| anyhow::anyhow!("{e}")),
        Some(ConfigAction::Path) => {
            fabryk_cli::config_handlers::cmd_config_path::<Config>(None)
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
        Some(ConfigAction::Get { key }) => {
            fabryk_cli::config_handlers::cmd_config_get_or_dump::<Config>(
                None, key.as_deref(),
            ).map_err(|e| anyhow::anyhow!("{e}"))
        }
        Some(ConfigAction::Set { key, value }) => {
            fabryk_cli::config_handlers::cmd_config_set::<Config>(
                None, &key, &value,
            ).map_err(|e| anyhow::anyhow!("{e}"))
        }
        Some(ConfigAction::Init { file, force }) => {
            fabryk_cli::config_handlers::cmd_config_init::<Config>(
                file.as_deref(), force,
            ).map_err(|e| anyhow::anyhow!("{e}"))
        }
        Some(ConfigAction::Export { docker_env, file }) => {
            let config = Config::load()?;
            fabryk_cli::config_handlers::cmd_config_export(
                &config, docker_env, file.as_deref(),
            ).map_err(|e| anyhow::anyhow!("{e}"))
        }
    }
}
```

## CLI Config Commands

Once wired up, your server gets these commands automatically:

```bash
# Show config file location
myapp-server config
myapp-server config path

# Show full config
myapp-server config get

# Show specific value
myapp-server config get database.host

# Set a value
myapp-server config set database.host localhost

# Create default config file
myapp-server config init
myapp-server config init --file ~/my-config.toml
myapp-server config init --force  # overwrite existing

# Export for Docker
myapp-server config export --docker-env              # to stdout
myapp-server config export --docker-env --file .env   # to file
```

## Path Resolution

The path resolution system ensures relative paths in config files resolve
correctly regardless of CWD. After loading, call `resolve_opt_path` on each
path field:

```rust
use fabryk_cli::config_utils::resolve_opt_path;

impl Config {
    fn resolve_all_paths(&mut self, base: &std::path::Path) {
        resolve_opt_path(&mut self.tls.cert_path, base, "tls.cert_path");
        resolve_opt_path(&mut self.tls.key_path, base, "tls.key_path");
        resolve_opt_path(&mut self.database.socket_path, base, "database.socket_path");
    }
}
```

**Base directory priority:**
1. `{PREFIX}_BASE_DIR` env var
2. Config file's parent directory
3. Current working directory

## Environment Variables

Every config field is accessible via environment variables using the pattern
`{PREFIX}_{SECTION}_{KEY}`:

```bash
# Top-level fields
export MYAPP_PORT=8080
export MYAPP_TRANSPORT=http

# Nested sections
export MYAPP_DATABASE_HOST=localhost
export MYAPP_OAUTH_ENABLED=true
export MYAPP_TLS_CERT_PATH=/etc/ssl/cert.pem
```

The `flatten_toml_value` function handles this mapping automatically,
including JSON serialization for array values.

## Docker / Cloud Run Deployment

Generate a `.env` file from your config:

```bash
myapp-server config export --docker-env --file .env
```

This creates:

```env
# Generated by myapp — do not edit manually
# Regenerate with: myapp config export --docker-env --file .env

MYAPP_PORT=8080
MYAPP_TRANSPORT=http
MYAPP_DATABASE_HOST=localhost
...
```

For Cloud Run, the `PORT` env var (set by the platform) overrides the
configured port when you use `ConfigLoaderBuilder::port_env_override("PORT")`.

## Validation Patterns

Implement a `validate()` method on your Config struct. Use two severity levels:

- **Hard errors** (`bail!` / return `Err`) for invalid configurations that
  prevent the server from operating correctly
- **Soft warnings** (`log::warn!`) for missing optional settings that degrade
  functionality but don't prevent startup

```rust
impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        // Hard error: invalid transport
        if self.transport != "http" && self.transport != "stdio" {
            anyhow::bail!("transport must be \"http\" or \"stdio\"");
        }

        // Soft warning: missing optional setting
        if self.database.host.is_empty() {
            log::warn!("database.host not set — database features disabled");
        }

        // Delegate to shared section validators
        self.oauth.validate("MYAPP")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.tls.validate()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(())
    }
}
```

## Reference Implementation

See **taproot** (`/lab/banyan/taproot/crates/taproot-server/src/config.rs`)
for a complete, production-tested example with BigQuery, Redis, LanceDB,
OAuth, and TLS configuration sections.
