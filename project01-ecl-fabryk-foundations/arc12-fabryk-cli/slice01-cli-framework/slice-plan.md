# CC Prompt: Fabryk 6.1 — CLI Framework

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

Create `fabryk-cli` crate with:

1. `FabrykCli<C: ConfigProvider>` - Generic CLI application
2. Common subcommands (serve, index, version, health)
3. Subcommand registration mechanism for domain-specific commands

