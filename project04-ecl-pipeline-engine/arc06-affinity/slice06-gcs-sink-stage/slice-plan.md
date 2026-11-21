# 7. Milestone 7.6: GCS Sink Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 1110-1163. Original bytes are recoverable from import commit 3a39aea. -->

## 7. Milestone 7.6: GCS Sink Stage

### 7.1 Scope

New crate `ecl-sink-gcs` for writing records/content to GCS as files.

### 7.2 Crate Structure

```
crates/ecl-sink-gcs/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 7.3 Implementation

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct GcsSinkConfig {
    pub bucket: String,
    pub prefix: String,
    /// Output format: "json_lines", "csv", "raw"
    #[serde(default = "default_json_lines")]
    pub format: String,
    /// Filename template. Default: "{timestamp}_{batch}"
    #[serde(default)]
    pub filename_template: Option<String>,
    /// Filter: "all", "valid_only", "errors_only"
    #[serde(default = "default_filter_all")]
    pub filter: String,
}
```

**Key behavior:**

- Collects all items from a batch, groups by filter criteria
- Writes one file per batch (all items serialized together)
- For `json_lines`: one JSON object per line
- For `csv`: header row + data rows
- For `errors_only`: only items with `_validation_status == "failed"`

**The stage accumulates items and writes on flush.** Since `Stage::process` receives one item at a time, the GCS sink buffers internally and writes on the last item or when the batch completes. Alternative: write per-item files (simpler, more files).

**Simpler approach (recommended for Phase 1):** Write one small JSON file per failed record. Path: `{prefix}/{timestamp}/{item_id}.json`. This avoids batch accumulation complexity.

**Tests:**

1. `test_gcs_sink_config_deserialize`
2. `test_gcs_sink_filter_errors_only`
3. `test_gcs_sink_json_format`

---

