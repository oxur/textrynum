# CC Prompt: Fabryk 1.3 — File & Path Utilities — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-core/src/util/files.rs` exists with all file utility functions
- [ ] `fabryk-core/src/util/paths.rs` exists with generic path utilities
- [ ] `FindOptions`, `FileInfo`, `find_file_by_id()`, `find_all_files()` are public exports
- [ ] `binary_path()`, `binary_dir()`, `find_dir_with_marker()`, `expand_tilde()` are public exports
- [ ] No domain-specific code (no music-theory markers, env vars, or hardcoded paths)
- [ ] All original tests pass in the new location
- [ ] `cargo test -p fabryk-core` passes
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean

