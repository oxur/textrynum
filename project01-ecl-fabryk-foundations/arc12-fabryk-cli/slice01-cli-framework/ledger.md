# CC Prompt: Fabryk 6.1 — CLI Framework — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `fabryk-cli` crate created
- [ ] `CliArgs` with global options (config, verbose, quiet)
- [ ] `BaseCommand` enum with serve, index, version, health, graph
- [ ] `GraphCommand` enum with build, validate, stats, query
- [ ] `FabrykCli<C: ConfigProvider>` generic application
- [ ] `CliExtension` trait for domain-specific commands
- [ ] Logging initialization based on verbosity flags
- [ ] All tests pass

