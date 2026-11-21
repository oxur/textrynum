# CC Prompt: Fabryk 6.3 — Music Theory CLI Integration — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-cli` dependency added to music-theory
- [ ] `MusicTheoryCli` extends `FabrykCli<Config>`
- [ ] Graph build uses `MusicTheoryExtractor`
- [ ] Extracted CLI files removed
- [ ] All CLI commands work:
  - [ ] `--help` shows correct commands
  - [ ] `version` shows correct version
  - [ ] `health` reports status
  - [ ] `graph build` produces correct graph
  - [ ] `graph validate` reports issues
  - [ ] `graph stats` shows statistics
  - [ ] `graph query` works for all query types
- [ ] `cargo test --all-features` passes

