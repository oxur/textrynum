# Milestone 6.5.3 — Integration: Wire Config into FabrykCli

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0041-phase-6.5-config-infra.md, lines 250-283. Original bytes are recoverable from import commit 2d006a7. -->

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

