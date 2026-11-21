# 7. Milestone 9.6: Pipeline Chaining

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 1168-1274. Original bytes are recoverable from import commit 2d006a7. -->

## 7. Milestone 9.6: Pipeline Chaining

### 7.1 Scope

Allow a pipeline to trigger one or more child pipelines on completion.

### 7.2 Spec Extension

#### File: `crates/ecl-pipeline-spec/src/lib.rs`

```rust
pub struct PipelineSpec {
    // ... existing fields ...
    /// Pipeline chaining: trigger other pipelines on completion.
    #[serde(default)]
    pub triggers: Option<TriggersSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggersSpec {
    /// Pipeline TOML config paths to trigger on success.
    #[serde(default)]
    pub on_success: Vec<String>,
    /// Pipeline TOML config paths to trigger on failure.
    #[serde(default)]
    pub on_failure: Vec<String>,
}
```

### 7.3 Runner Integration

#### File: `crates/ecl-pipeline/src/runner.rs`

After `run()` completes:

```rust
pub async fn run(&mut self) -> Result<&PipelineState> {
    // ... existing execution ...

    // Pipeline chaining.
    if let Some(triggers) = &self.topology.spec.triggers {
        match &self.state.status {
            PipelineStatus::Completed { .. } => {
                for child_path in &triggers.on_success {
                    self.trigger_child_pipeline(child_path).await?;
                }
            }
            PipelineStatus::Failed { .. } => {
                for child_path in &triggers.on_failure {
                    self.trigger_child_pipeline(child_path).await?;
                }
            }
            _ => {}
        }
    }

    Ok(&self.state)
}

async fn trigger_child_pipeline(&self, config_path: &str) -> Result<()> {
    tracing::info!(child = %config_path, "triggering child pipeline");

    // Spawn as a child process so it has its own memory space,
    // state store, and checkpoint.
    let status = tokio::process::Command::new(std::env::current_exe()?)
        .args(["pipeline", "run", config_path])
        .status()
        .await
        .map_err(|e| PipelineError::PushSource {
            source_name: "trigger".to_string(),
            detail: format!("failed to spawn child pipeline: {e}"),
        })?;

    if !status.success() {
        tracing::warn!(
            child = %config_path,
            exit_code = ?status.code(),
            "child pipeline exited with error"
        );
    }

    Ok(())
}
```

**Alternative (simpler, better for testing):** Instead of spawning a process, load and run the child pipeline in-process with a fresh `PipelineRunner`:

```rust
async fn trigger_child_pipeline(&self, config_path: &str) -> Result<()> {
    let toml_str = tokio::fs::read_to_string(config_path).await?;
    let spec = PipelineSpec::from_toml(&toml_str)?;
    // ... resolve topology, create runner, run ...
}
```

**Decision:** Support both. Default is process-level (isolation). Add `trigger_mode = "in_process"` option for testing.

### 7.4 Tests

1. `test_triggers_spec_serde`
2. `test_triggers_on_success_called` — mock child pipeline
3. `test_triggers_on_failure_called`
4. `test_triggers_none_no_error`
5. `test_triggers_child_failure_logged_not_fatal`

---

