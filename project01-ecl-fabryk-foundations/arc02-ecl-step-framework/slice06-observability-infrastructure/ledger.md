# Stage 2.6: Observability Infrastructure — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Criteria and checkbox states below are transcribed, not re-evaluated. No new row statuses, verification results or sign-offs are assigned.

<!-- Historical source: crates/design/dev/0008-phase-2-06-observability.md, lines 1086-1149. Original bytes are recoverable from import commit 3a39aea. -->

## Acceptance Criteria

### Must Have

- [ ] `LogConfig` implemented with dev/prod presets
- [ ] Span builders (`WorkflowSpan`, `StepSpan`, `LlmSpan`, `DatabaseSpan`) implemented
- [ ] Logging initialized in `ecl-workflows/src/main.rs`
- [ ] All workflow operations log with `workflow_id` correlation
- [ ] All step operations log with `step_id` and `workflow_id`
- [ ] LLM calls log token usage and duration
- [ ] Retry attempts logged with attempt number
- [ ] Performance metrics logged (duration, tokens)
- [ ] Validation failures logged with field and reason
- [ ] Log output readable in dev (Pretty format)
- [ ] Log output parseable in prod (JSON format)
- [ ] Performance overhead <5% measured via benchmark
- [ ] All logging code has ≥95% test coverage
- [ ] Documentation includes logging patterns guide

### Nice to Have

- [ ] SQL exporter proof-of-concept implemented
- [ ] SQL schema for log storage defined
- [ ] Performance benchmarks for SQL export
- [ ] Log sampling for high-volume scenarios
- [ ] Metrics exported to external system (Prometheus, etc.)
- [ ] PR submitted to twyg project

### Verification

1. **Functionality**:

   ```bash
   # Run workflow with pretty logging
   ECL_LOG_LEVEL=debug cargo run --bin ecl-workflows

   # Run with JSON output
   ECL_LOG_FORMAT=json cargo run --bin ecl-workflows | jq .

   # Filter by workflow_id
   cargo run --bin ecl-workflows | grep workflow_id=abc123
   ```

2. **Performance**:

   ```bash
   # Benchmark with minimal logging
   ECL_LOG_LEVEL=error cargo run --bin ecl-workflows --release

   # Benchmark with full logging
   ECL_LOG_LEVEL=debug cargo run --bin ecl-workflows --release

   # Compare execution times
   ```

3. **Test Coverage**:

   ```bash
   cargo tarpaulin --out Xml --exclude-files "*/tests/*"
   # Verify logging module coverage ≥95%
   ```

---

