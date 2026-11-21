# Pipeline Phase 5 — CLI and observability

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 470-533. Original bytes are recoverable from import commit 2d006a7. -->

## Phase 5: CLI & Polish

### Milestone 5.1: CLI Commands

**Goal:** `ecl pipeline run|resume|status|inspect|items` commands work.

**Crate:** `crates/ecl-cli/` (extend)

**Key files to create/modify:**

- Modify: `src/main.rs` — add pipeline subcommand group
- Create: `src/pipeline.rs` — pipeline CLI handler
- Create: `src/pipeline/run.rs` — `ecl pipeline run <config.toml>`
- Create: `src/pipeline/resume.rs` — `ecl pipeline resume [--force] <output-dir>`
- Create: `src/pipeline/inspect.rs` — `ecl pipeline inspect <output-dir>` (full JSON state)
- Create: `src/pipeline/status.rs` — `ecl pipeline status <output-dir>` (human-readable summary)
- Create: `src/pipeline/items.rs` — `ecl pipeline items <output-dir>` (item listing)
- Create: `src/pipeline/diff.rs` — `ecl pipeline diff <dir1> <dir2>` (compare two runs)

**Key deliverables:**

- [ ] `ecl pipeline run <config.toml>` — parse TOML, run pipeline, print summary
- [ ] `ecl pipeline resume <output-dir>` — load checkpoint, resume execution
- [ ] `ecl pipeline resume --force <output-dir>` — resume despite config drift
- [ ] `ecl pipeline inspect <output-dir>` — pretty-print full state as JSON
- [ ] `ecl pipeline status <output-dir>` — human-readable one-line-per-stage summary
- [ ] `ecl pipeline items <output-dir>` — table of items with status, source, hash
- [ ] `ecl pipeline diff <dir1> <dir2>` — compare two runs (new/changed/removed items, stage outcome differences)
- [ ] Structured tracing output during execution (stage started/completed/failed)
- [ ] Exit codes: 0 = success, 1 = failure, 2 = partial (some items failed)
- [ ] Test: CLI integration tests using assert_cmd or similar

**Test strategy:**

- CLI flag parsing tests
- End-to-end: write TOML, run CLI, verify output dir
- Inspect output matches expected JSON structure
- Status output is human-readable
- Resume after ctrl-c simulation

---

### Milestone 5.2: Observability & Reporting

**Goal:** Pipeline state output is rich enough for Claude to analyze.
Tracing integration provides structured execution logs.

**Files to modify:**

- `ecl-pipeline/src/runner.rs` — add tracing spans and events
- `ecl-pipeline-state/src/types.rs` — add timing fields, improve Display impls

**Key deliverables:**

- [ ] Tracing spans: one per batch, one per stage, one per item
- [ ] Structured fields: stage_id, item_id, duration_ms, status, error
- [ ] `PipelineState` JSON output matches design doc section 6 exactly
- [ ] Timing: per-stage started_at/completed_at, per-item duration_ms
- [ ] Stats: totals computed and cached in `PipelineStats`
- [ ] Human-readable Display impl for `PipelineStatus`, `ItemStatus`, `StageStatus`
- [ ] Test: verify JSON output structure matches design doc example

---


## Reconstructed slice locations

- [Milestone 5.1: CLI Commands (`ecl-cli` pipeline subcommands)](slice01-cli-commands/slice-plan.md)
- [Milestone 5.2: Observability & Reporting](slice02-observability-reporting/slice-plan.md)
