# CC Prompt: Fabryk 1.6 — Phase 1 Completion & Verification — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] fabryk-core module structure matches specification
- [ ] All public exports in lib.rs verified
- [ ] `cargo test -p fabryk-core` passes (all tests)
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean
- [ ] `cargo doc -p fabryk-core --no-deps` builds without warnings
- [ ] Doc tests pass (`cargo test --doc -p fabryk-core`)
- [ ] Error type has all variants and constructors from milestone 1.2
- [ ] ConfigProvider trait defined with required methods
- [ ] PathResolver supports builder pattern and all resolution methods
- [ ] AppState<C: ConfigProvider> is Clone and provides config access

