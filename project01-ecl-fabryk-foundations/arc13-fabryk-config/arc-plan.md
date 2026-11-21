# Fabryk Phase 6.5 — Configuration

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/fabryk/0041-phase-6.5-config-infra.md, lines 1-8. Original bytes are recoverable from import commit 3a39aea. -->

# Phase 6.5: Configuration Infrastructure

## Context

Phase 6 is complete (fabryk-cli with 43 tests, committed as 5efa7c8). The CLI framework has `--config` flag parsing but it's unused — there's no actual config loading, no config file format, and no `config` subcommand. Phase 6.5 adds configuration infrastructure following the taproot pattern: TOML config files loaded via the `confyg` crate with env var overlay, plus a `config` CLI subcommand for init/get/set/path/export operations.

---


Historical milestone moved to [Milestone 6.5.1 — FabrykConfig Struct & Loading](slice01-1-fabrykconfig-struct-loading/slice-plan.md).

Historical milestone moved to [Milestone 6.5.2 — Config CLI Commands](slice02-2-config-cli-commands/slice-plan.md).

Historical milestone moved to [Milestone 6.5.3 — Integration: Wire Config into FabrykCli](slice03-3-integration-wire-config-into-fabrykcli/slice-plan.md).

<!-- Historical source: crates/design/dev/fabryk/0041-phase-6.5-config-infra.md, lines 284-328. Original bytes are recoverable from import commit 3a39aea. -->

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

## Reconstructed slice locations

- [Milestone 6.5.1 — FabrykConfig Struct & Loading](slice01-1-fabrykconfig-struct-loading/slice-plan.md)
- [Milestone 6.5.2 — Config CLI Commands](slice02-2-config-cli-commands/slice-plan.md)
- [Milestone 6.5.3 — Integration: Wire Config into FabrykCli](slice03-3-integration-wire-config-into-fabrykcli/slice-plan.md)
