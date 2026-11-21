# Milestone 6.5.1 — FabrykConfig Struct & Loading

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0041-phase-6.5-config-infra.md, lines 9-120. Original bytes are recoverable from import commit 3a39aea. -->

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

