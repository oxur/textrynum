# Fabryk Phase 1 — Core

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 42-69. Original bytes are recoverable from import commit 2d006a7. -->

## Phase 1: fabryk-core (1–2 weeks)

**Goal:** Create the Fabryk workspace and extract shared types, traits, errors,
and utilities. This is the foundation that every other crate depends on.

**Crate:** `fabryk-core`
**Dependency level:** 0 (no internal Fabryk dependencies)
**Risk:** Low

| # | Milestone | Description | Key Deliverables | Audit Ref |
|---|-----------|-------------|------------------|-----------|
| 1.1 | Workspace scaffold | Create Fabryk repo, workspace Cargo.toml with all 10 crate stubs, CI config, LICENSE, README skeleton | Workspace compiles (`cargo check`), all stubs present | §6.1, §7 Phase 1 |
| 1.2 | Error types & Result | Extract `error.rs` to `fabryk-core`. Define `fabryk_core::Error` and `fabryk_core::Result<T>`. No changes needed — file is already generic | `fabryk-core` compiles, error types exported | §4.1 |
| 1.3 | File utilities | Extract `util/files.rs` to `fabryk-core::util::files`. Functions: `find_file_by_id()`, `find_all_files()`, `FindOptions`. Already generic | All file utility tests pass in fabryk-core | §4.1 |
| 1.4 | Path utilities | Extract `util/paths.rs` to `fabryk-core::util::paths`. Parameterise: replace `MUSIC_THEORY_*` env var prefix with configurable project name. Add `compute_id()` utility (Amendment §2f-i) | Path tests pass with parameterised env vars | §4.1, Amend §2f-i |
| 1.5 | ConfigProvider trait | Define `ConfigProvider` trait in `fabryk-core::traits`. Methods: `base_path()`, `content_path()`, `project_name()`. Extract `state.rs` as `AppState<C: ConfigProvider>` | Trait defined, AppState generic, compiles | §4.1 |
| 1.6 | Resource serving | Extract `resources/mod.rs` to `fabryk-core::resources` | Resource serving tests pass | §4.1 |
| 1.7 | Music theory integration | Add `fabryk-core` dependency to ai-music-theory. Update all imports from local modules to `fabryk_core::`. Implement `ConfigProvider` for music theory `Config`. Remove extracted files from music-theory repo | `cargo test --all-features` passes in both repos | §7 Phase 2 |

**Phase 1 exit criteria:**

- `fabryk-core` compiles and all tests pass
- ai-music-theory depends on `fabryk-core` for errors, utilities, paths, state
- No duplicate code between repos for extracted files
- `cargo clippy` clean in both repos

---


## Reconstructed slice locations

- [CC Prompt: Fabryk 1.0 & 1.1 — Cleanup & Workspace Scaffold](slice01-cleanup-workspace-scaffold/slice-plan.md)
- [CC Prompt: Fabryk 1.2 — Error Types & Result (Full Extraction)](slice02-error-types-result-full-extraction/slice-plan.md)
- [CC Prompt: Fabryk 1.3 — File & Path Utilities](slice03-file-path-utilities/slice-plan.md)
- [CC Prompt: Fabryk 1.4 — ID Utilities & PathResolver](slice04-id-utilities-pathresolver/slice-plan.md)
- [CC Prompt: Fabryk 1.5 — ConfigProvider Trait & AppState](slice05-configprovider-trait-appstate/slice-plan.md)
- [CC Prompt: Fabryk 1.6 — Phase 1 Completion & Verification](slice06-phase-1-completion-verification/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
