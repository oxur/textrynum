# 8. Milestone 9.7: Scheduling & Cron Triggers

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 1275-1360. Original bytes are recoverable from import commit 3a39aea. -->

## 8. Milestone 9.7: Scheduling & Cron Triggers

### 8.1 Scope

Add `ecl pipeline schedule <config.toml>` CLI command that runs pipelines on a cron schedule.

### 8.2 Implementation

#### File: `crates/ecl-cli/src/pipeline/schedule.rs` (new)

```rust
use cron::Schedule;
use std::str::FromStr;

pub async fn execute(config_path: PathBuf, cron_expr: Option<String>) -> Result<()> {
    let toml_str = tokio::fs::read_to_string(&config_path).await?;
    let spec = PipelineSpec::from_toml(&toml_str)?;

    // Get cron expression from CLI arg or spec.
    let cron_str = cron_expr
        .or_else(|| spec.schedule.as_ref().map(|s| s.cron.clone()))
        .ok_or_else(|| anyhow::anyhow!("no cron schedule specified"))?;

    let schedule = Schedule::from_str(&cron_str)
        .map_err(|e| anyhow::anyhow!("invalid cron expression: {e}"))?;

    tracing::info!(cron = %cron_str, pipeline = %spec.name, "starting scheduler");

    loop {
        let next = schedule.upcoming(chrono::Utc).next()
            .ok_or_else(|| anyhow::anyhow!("no upcoming schedule"))?;

        let wait_duration = (next - chrono::Utc::now())
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(1));

        tracing::info!(
            next_run = %next,
            wait_secs = wait_duration.as_secs(),
            "waiting for next scheduled run"
        );

        tokio::time::sleep(wait_duration).await;

        tracing::info!(pipeline = %spec.name, "starting scheduled run");
        match super::run::execute(config_path.clone()).await {
            Ok(()) => tracing::info!(pipeline = %spec.name, "scheduled run completed"),
            Err(e) => tracing::error!(pipeline = %spec.name, error = %e, "scheduled run failed"),
        }
    }
}
```

### 8.3 Spec Extension

```rust
pub struct PipelineSpec {
    // ... existing fields ...
    /// Optional schedule configuration.
    #[serde(default)]
    pub schedule: Option<ScheduleSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    /// Cron expression (5 or 7 fields). Example: "30 21 * * *"
    pub cron: String,
}
```

### 8.4 Dependencies

```toml
# In ecl-cli/Cargo.toml:
cron = "0.15"
```

### 8.5 Tests

1. `test_schedule_spec_serde`
2. `test_cron_parse_valid` — "30 21 ** *"
3. `test_cron_parse_invalid` — error handling
4. `test_schedule_computes_next_run` — verify upcoming time calculation

---

