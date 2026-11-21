# CC Prompt: Fabryk 1.2 — Error Types & Result (Full Extraction) — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-core/src/error.rs` has full `Error` enum with all variants
- [ ] Uses `thiserror` derive (not custom struct)
- [ ] Has `#[backtrace]` on `#[from]` conversions
- [ ] Constructor helpers: `io()`, `io_with_path()`, `config()`, `not_found()`,
      `not_found_msg()`, `file_not_found()`, `invalid_path()`, `parse()`,
      `operation()`
- [ ] Inspector methods: `is_io()`, `is_not_found()`, `is_config()`,
      `is_path_error()`, `is_parse()`
- [ ] `From<std::io::Error>` and `From<serde_json::Error>` implementations
- [ ] No MCP dependencies in `fabryk-core`
- [ ] Note added for `fabryk-mcp` error extension (future milestone)
- [ ] `cargo test -p fabryk-core` passes (all tests from music-theory adapted)
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean
- [ ] `cargo doc -p fabryk-core --no-deps` generates clean documentation

