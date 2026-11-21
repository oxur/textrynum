# CC Prompt: Fabryk 1.4 — ID Utilities & PathResolver — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-core/src/util/ids.rs` exists with `normalize_id()` and `id_from_path()`
- [ ] `fabryk-core/src/util/resolver.rs` exists with `PathResolver` struct
- [ ] `PathResolver` uses generic utilities from milestone 1.3
- [ ] `PathResolver` supports env var, marker search, and fallback resolution
- [ ] No hardcoded domain-specific strings (MUSIC_THEORY, etc.)
- [ ] Convenience re-exports at crate root: `normalize_id`, `id_from_path`, `PathResolver`
- [ ] All tests pass
- [ ] `cargo test -p fabryk-core` passes
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean

