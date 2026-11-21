# Pipeline Phase 3 — Filesystem pipeline

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 322-398. Original bytes are recoverable from import commit 3a39aea. -->

## Phase 3: First Adapter (Filesystem — Test Harness)

The filesystem adapter is the simplest possible adapter. It turns a local
directory into a source, letting us test the full pipeline end-to-end without
network calls or credentials.

### Milestone 3.1: Filesystem Adapter + Built-in Stages

**Goal:** `ecl-adapter-fs` enumerates/fetches local files. `ecl-stages`
provides extract, normalize (passthrough), and emit stages. Full end-to-end
pipeline works with local files.

**Crates:** `crates/ecl-adapter-fs/`, `crates/ecl-stages/`

**Key files to create:**

- `ecl-adapter-fs/Cargo.toml`
- `ecl-adapter-fs/src/lib.rs` — `FilesystemAdapter` implementing `SourceAdapter`
- `ecl-stages/Cargo.toml`
- `ecl-stages/src/lib.rs` — re-exports, stage registration
- `ecl-stages/src/extract.rs` — `ExtractStage` (delegates to SourceAdapter)
- `ecl-stages/src/normalize.rs` — `NormalizeStage` (passthrough for PoC; later: format conversion)
- `ecl-stages/src/filter.rs` — `FilterStage` (glob-based include/exclude filtering)
- `ecl-stages/src/emit.rs` — `EmitStage` (writes PipelineItem content to output_dir)

**Key deliverables:**

- [ ] `FilesystemAdapter::enumerate()` — walks directory, returns `SourceItem` per file
- [ ] `FilesystemAdapter::fetch()` — reads file content, computes blake3 hash
- [ ] Filter support: extension filter, glob patterns
- [ ] `ExtractStage::process()` — calls adapter.fetch() for each item
- [ ] `NormalizeStage::process()` — passes through content (placeholder)
- [ ] `FilterStage::process()` — applies glob include/exclude rules, returns empty vec for excluded items
- [ ] `EmitStage::process()` — writes content to `output_dir/{subdir}/{filename}`
- [ ] End-to-end test: create temp dir with files, write TOML config, run pipeline, verify output files
- [ ] Test: incrementality — run twice, second run skips unchanged files
- [ ] Test: modified file detected and re-processed
- [ ] Register both crates as workspace members

**Test strategy:**

- Create tempdir with 5 text files
- Write a TOML config pointing at it
- Run pipeline, verify output files match input
- Modify one file, run again, verify only that file is re-processed
- Add a new file, run again, verify it's picked up
- Delete a file, run again, verify it's not in output

---

### Milestone 3.2: Integration Test Suite

**Goal:** Comprehensive integration tests exercising the full pipeline
through realistic scenarios.

**Crate:** `crates/ecl-pipeline/` (integration tests)

**Key files to create:**

- `crates/ecl-pipeline/tests/integration/` — integration test directory
- `tests/integration/full_pipeline.rs` — end-to-end with filesystem adapter
- `tests/integration/checkpoint_resume.rs` — crash and resume scenarios
- `tests/integration/incrementality.rs` — cross-run hash comparison
- `tests/integration/error_handling.rs` — retry, skip_on_error, stage failure

**Key deliverables:**

- [ ] Happy path: 10 files, 3 stages, verify all items completed
- [ ] Checkpoint/resume: interrupt after batch 0, resume, verify batch 1+ runs
- [ ] Incrementality: run 1 processes all, run 2 skips unchanged
- [ ] Error handling: one file causes stage failure, skip_on_error continues
- [ ] Concurrent stages: two independent extract stages run in parallel
- [ ] Fan-in aggregation: stage reading multiple resources receives combined item set
- [ ] State inspection: verify JSON output matches expected structure

---


## Reconstructed slice locations

- [Milestone 3.1: Filesystem Adapter + Built-in Stages (`ecl-adapter-fs`, `ecl-stages`)](slice01-filesystem-adapter-built-in-stages/slice-plan.md)
- [Milestone 3.2: Integration Test Suite](slice02-integration-test-suite/slice-plan.md)
