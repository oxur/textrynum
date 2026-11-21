# CC Prompt: Fabryk 1.5 — ConfigProvider Trait & AppState — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-core/src/traits.rs` exists with `ConfigProvider` trait
- [ ] `ConfigProvider` has at minimum: `project_name()`, `base_path()`, `content_path()`
- [ ] Trait bounds include `Send + Sync + Clone + 'static`
- [ ] `fabryk-core/src/state.rs` exists with `AppState<C: ConfigProvider>`
- [ ] `AppState` is `Clone` and provides `config()` and `config_arc()` accessors
- [ ] `AppState` does NOT hold search or graph state (that's for later phases)
- [ ] Test config implements `ConfigProvider` and tests pass
- [ ] `cargo test -p fabryk-core` passes
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean

