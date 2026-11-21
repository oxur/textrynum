# 8. Milestone 7.7: File Lifecycle Management

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 1164-1256. Original bytes are recoverable from import commit 3a39aea. -->

## 8. Milestone 7.7: File Lifecycle Management

### 8.1 Scope

Manage the `input/ → staging/ → historical/error/` file lifecycle for GCS-based pipelines.

### 8.2 Design

Add lifecycle configuration to PipelineSpec:

#### File: `crates/ecl-pipeline-spec/src/lib.rs`

```rust
pub struct PipelineSpec {
    // ... existing fields ...

    /// File lifecycle management for cloud storage sources.
    #[serde(default)]
    pub lifecycle: Option<LifecycleSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleSpec {
    /// GCS bucket for lifecycle operations.
    pub bucket: String,
    /// Prefix for staging files.
    #[serde(default = "default_staging")]
    pub staging_prefix: String,
    /// Prefix for successfully processed files.
    #[serde(default = "default_historical")]
    pub historical_prefix: String,
    /// Prefix for failed processing files.
    #[serde(default = "default_error")]
    pub error_prefix: String,
    /// What to do on pipeline success.
    #[serde(default = "default_move_historical")]
    pub on_success: String,  // "move_to_historical", "delete", "none"
    /// What to do on pipeline failure.
    #[serde(default = "default_move_input")]
    pub on_failure: String,  // "move_to_input", "move_to_error", "none"
}
```

### 8.3 Runner Hooks

#### File: `crates/ecl-pipeline/src/runner.rs`

Add pre-run and post-run lifecycle hooks:

```rust
impl PipelineRunner {
    async fn run(&mut self) -> Result<&PipelineState> {
        // ... existing phases ...

        // NEW: Post-run lifecycle
        if let Some(lifecycle) = &self.topology.spec.lifecycle {
            match &self.state.status {
                PipelineStatus::Completed { .. } => {
                    self.lifecycle_on_success(lifecycle).await?;
                }
                PipelineStatus::Failed { .. } => {
                    self.lifecycle_on_failure(lifecycle).await?;
                }
                _ => {}
            }
        }

        Ok(&self.state)
    }

    async fn lifecycle_on_success(&self, spec: &LifecycleSpec) -> Result<()> {
        // Move files from staging/ to historical/{run_id}/
        // Uses GCS client to copy + delete
    }

    async fn lifecycle_on_failure(&self, spec: &LifecycleSpec) -> Result<()> {
        // Move files from staging/ back to input/
    }
}
```

**Phase 1 simplification:** Since the runner doesn't currently own a GCS client, the lifecycle operations can be implemented as a special "lifecycle" stage that runs at the end of the pipeline. Or: the runner can construct a minimal GCS client from the lifecycle spec credentials. Recommend the latter for cleanliness.

**Tests:**

1. `test_lifecycle_spec_serde_roundtrip`
2. `test_lifecycle_defaults`
3. `test_runner_calls_lifecycle_on_success` (mock GCS)
4. `test_runner_calls_lifecycle_on_failure` (mock GCS)
5. `test_runner_no_lifecycle_no_error` — backward compatible

---

