# Fabryk Phase 6 — CLI

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 194-216. Original bytes are recoverable from import commit 2d006a7. -->

## Phase 6: fabryk-cli (1 week)

**Goal:** Extract the CLI framework.

**Crate:** `fabryk-cli`
**Dependency level:** 3
**Risk:** Low

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 6.1 | CLI framework | Extract `cli.rs`, `main.rs`, `lib.rs` to `fabryk-cli`. Parameterise: `FabrykCli<C: ConfigProvider>`, configurable project name and commands. Support domain-specific subcommand registration | CLI framework compiles with mock config | §4.10 |
| 6.2 | Graph CLI commands | Extract `graph/cli.rs` to `fabryk-cli`. Parameterise: `handle_build<E: GraphExtractor>()`, `handle_validate()`. Pass extractor instance from domain code | Graph CLI commands work via fabryk-cli | §4.10 |
| 6.3 | Music theory CLI integration | Update ai-music-theory `main.rs` to use `fabryk-cli`. Register music-theory-specific subcommands. Remove extracted CLI files | `cargo run -- --help` shows correct commands | §7 Phase 7 |

**Phase 6 exit criteria:**

- `fabryk-cli` compiles and passes tests
- ai-music-theory uses `fabryk-cli` for all CLI operations
- All CLI commands work: `serve`, `index`, `graph build`, `graph validate`, etc.
- `cargo test --all-features` passes in both repos

---


## Reconstructed slice locations

- [CC Prompt: Fabryk 6.1 — CLI Framework](slice01-cli-framework/slice-plan.md)
- [CC Prompt: Fabryk 6.2 — Graph CLI Commands](slice02-graph-cli-commands/slice-plan.md)
- [CC Prompt: Fabryk 6.3 — Music Theory CLI Integration](slice03-music-theory-cli-integration/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
