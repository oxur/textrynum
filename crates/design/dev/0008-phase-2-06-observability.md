# Stage 2.6: Observability Infrastructure

**Version:** 1.0
**Date:** 2026-01-23
**Phase:** 2.6
**Status:** Ready for Implementation

---

## Goal

Implement structured logging and tracing infrastructure to provide comprehensive observability across workflow and step execution. Build upon the existing `tracing` usage to create consistent, correlatable logging patterns with structured fields.

---

## Dependencies

This stage requires completion of:

- **Stage 2.1**: Step trait definition (for step metadata)
- **Stage 2.2**: Step executor (for execution tracing)
- **Stage 2.3**: Toy workflow steps (for step-specific logging)
- **Stage 2.4**: Workflow orchestrator (for workflow-level tracing)
- **Stage 2.5**: SQLx persistence layer (for database operation logging)

---

## Overview

The observability infrastructure provides:

1. **Structured Logging**: Rich, parseable log messages using `twyg`
2. **Tracing Spans**: skip for now
3. **Log Correlation**: Track operations across the system using `workflow_id` and `step_id`
4. **Performance Monitoring**: skip for now
5. **Optional SQL Exporter**: Investigate and potentially implement SQL-based log storage

---

## Architecture

### Module Structure

```
ecl-core/src/logging/
├── mod.rs              # Public API, re-exports, initialization
├── config.rs           # LogConfig, output formats, filtering
├── spans.rs            # Tracing span builders and helpers
└── exporter.rs         # Optional: SQL exporter for twyg
```

### Logging Layers

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│  (Workflows, Steps, Services)           │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│       Structured Logging Layer          │
│             (twyg)                      │
└──────────────┬──────────────────────────┘
               │
     ┌─────────┴─────────┐
     ▼                   ▼
┌─────────┐      ┌──────────────┐
│ Console │      │ SQL Exporter │
│ Output  │      │  (Optional)  │
└─────────┘      └──────────────┘
```

---

## Detailed Implementation

### 1. Logging Configuration (`logging/config.rs`)

Define configuration for logging behavior. ECL-wide configuration in TOML should be used. See the following for examples on how to integrate:

- <https://github.com/oxur/twyg/blob/release/0.6.x/examples/from-confyg.rs>

```rust
use serde::{Deserialize, Serialize};

/// Configuration for the logging system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LogConfig {
    /// Log level filter (trace, debug, info, warn, error)
    pub level: LogLevel,

    /// Output format
    pub format: LogFormat,

    /// Whether to include timestamps
    pub include_timestamps: bool,

    /// Whether to include source location (file:line)
    pub include_location: bool,

    /// Whether to use colored output (dev mode)
    pub use_colors: bool,

    /// Optional SQL exporter configuration
    pub sql_exporter: Option<SqlExporterConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LogFormat {
    /// Human-readable format for development
    Pretty,

    /// JSON format for production parsing
    Json,

    /// Compact format with structured fields
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SqlExporterConfig {
    /// Database connection string
    pub database_url: String,

    /// Whether to export asynchronously
    pub async_export: bool,

    /// Buffer size for batched writes
    pub buffer_size: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            include_timestamps: true,
            include_location: false,
            use_colors: true,
            sql_exporter: None,
        }
    }
}

impl LogConfig {
    /// Create a development configuration (pretty, colored output)
    pub fn dev() -> Self {
        Self {
            level: LogLevel::Debug,
            format: LogFormat::Pretty,
            use_colors: true,
            ..Default::default()
        }
    }

    /// Create a production configuration (JSON, no colors)
    pub fn prod() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
            use_colors: false,
            include_location: true,
            ..Default::default()
        }
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}
```

### 2. Tracing Spans (`logging/spans.rs`)

Provide span builders for consistent tracing:

```rust
use crate::types::{WorkflowId, StepId};
use tracing::{Span, Level};

/// Builder for workflow execution spans.
pub struct WorkflowSpan {
    workflow_id: WorkflowId,
    workflow_name: String,
}

impl WorkflowSpan {
    pub fn new(workflow_id: WorkflowId, workflow_name: impl Into<String>) -> Self {
        Self {
            workflow_id,
            workflow_name: workflow_name.into(),
        }
    }

    /// Create and enter the workflow span
    pub fn enter(self) -> Span {
        tracing::info_span!(
            "workflow",
            workflow_id = %self.workflow_id,
            workflow_name = %self.workflow_name,
        )
    }
}

/// Builder for step execution spans.
pub struct StepSpan {
    workflow_id: WorkflowId,
    step_id: StepId,
    step_name: String,
    attempt: u32,
}

impl StepSpan {
    pub fn new(
        workflow_id: WorkflowId,
        step_id: StepId,
        step_name: impl Into<String>,
    ) -> Self {
        Self {
            workflow_id,
            step_id,
            step_name: step_name.into(),
            attempt: 1,
        }
    }

    /// Set the retry attempt number
    pub fn attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    /// Create and enter the step span
    pub fn enter(self) -> Span {
        tracing::info_span!(
            "step",
            workflow_id = %self.workflow_id,
            step_id = %self.step_id,
            step_name = %self.step_name,
            attempt = self.attempt,
        )
    }
}

/// Builder for LLM operation spans.
pub struct LlmSpan {
    workflow_id: WorkflowId,
    step_id: Option<StepId>,
    model: String,
}

impl LlmSpan {
    pub fn new(workflow_id: WorkflowId, model: impl Into<String>) -> Self {
        Self {
            workflow_id,
            step_id: None,
            model: model.into(),
        }
    }

    /// Associate with a specific step
    pub fn step_id(mut self, step_id: StepId) -> Self {
        self.step_id = Some(step_id);
        self
    }

    /// Create and enter the LLM span
    pub fn enter(self) -> Span {
        match self.step_id {
            Some(step_id) => tracing::debug_span!(
                "llm_call",
                workflow_id = %self.workflow_id,
                step_id = %step_id,
                model = %self.model,
            ),
            None => tracing::debug_span!(
                "llm_call",
                workflow_id = %self.workflow_id,
                model = %self.model,
            ),
        }
    }
}

/// Builder for database operation spans.
pub struct DatabaseSpan {
    operation: String,
    table: Option<String>,
}

impl DatabaseSpan {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            table: None,
        }
    }

    /// Specify the table being operated on
    pub fn table(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }

    /// Create and enter the database span
    pub fn enter(self) -> Span {
        match self.table {
            Some(table) => tracing::debug_span!(
                "database",
                operation = %self.operation,
                table = %table,
            ),
            None => tracing::debug_span!(
                "database",
                operation = %self.operation,
            ),
        }
    }
}

/// Record performance metrics in the current span.
pub fn record_performance(duration_ms: u64, tokens_used: Option<u32>) {
    if let Some(tokens) = tokens_used {
        tracing::info!(
            duration_ms = duration_ms,
            tokens_used = tokens,
            "Operation completed"
        );
    } else {
        tracing::info!(
            duration_ms = duration_ms,
            "Operation completed"
        );
    }
}

/// Record retry attempt in the current span.
pub fn record_retry(attempt: u32, max_attempts: u32, error: &str) {
    tracing::warn!(
        attempt = attempt,
        max_attempts = max_attempts,
        error = %error,
        "Retrying after failure"
    );
}

/// Record validation failure in the current span.
pub fn record_validation_failure(field: &str, reason: &str) {
    tracing::warn!(
        field = %field,
        reason = %reason,
        "Validation failed"
    );
}
```

### 3. Module Root (`logging/mod.rs`)

Public API and initialization:

```rust
//! Structured logging and tracing infrastructure.
//!
//! This module provides:
//! - Structured logging with twyg and tracing
//! - Hierarchical spans for workflow and step execution
//! - Log correlation by workflow_id
//! - Performance monitoring
//!
//! # Examples
//!
//! ```no_run
//! use ecl_core::logging::{init_logging, LogConfig, WorkflowSpan};
//! use ecl_core::types::WorkflowId;
//!
//! // Initialize logging at application start
//! let config = LogConfig::dev();
//! init_logging(config).expect("Failed to initialize logging");
//!
//! // Create workflow span
//! let workflow_id = WorkflowId::new();
//! let span = WorkflowSpan::new(workflow_id, "critique-revise").enter();
//! let _guard = span.enter();
//!
//! // All logging within this scope will be correlated
//! tracing::info!("Workflow started");
//! ```

use crate::error::{EclError, EclResult};

mod config;
mod spans;

pub use config::{LogConfig, LogFormat, LogLevel, SqlExporterConfig};
pub use spans::{
    DatabaseSpan, LlmSpan, StepSpan, WorkflowSpan,
    record_performance, record_retry, record_validation_failure,
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the logging system with the given configuration.
///
/// This should be called once at application startup before any logging occurs.
///
/// # Errors
///
/// Returns an error if the logging system cannot be initialized (e.g., if
/// already initialized or if the configuration is invalid).
pub fn init_logging(config: LogConfig) -> EclResult<()> {
    // Build the tracing subscriber based on format
    let subscriber = tracing_subscriber::registry();

    // Create environment filter from config level
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(format!("{:?}", config.level).to_lowercase()))
        .map_err(|e| EclError::Configuration(format!("Invalid log level: {}", e)))?;

    match config.format {
        LogFormat::Pretty => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(config.include_location)
                .with_line_number(config.include_location)
                .with_ansi(config.use_colors)
                .pretty();

            subscriber
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| EclError::Configuration(format!("Failed to initialize logging: {}", e)))?;
        }
        LogFormat::Json => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(config.include_location)
                .with_line_number(config.include_location)
                .with_ansi(false)
                .json();

            subscriber
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| EclError::Configuration(format!("Failed to initialize logging: {}", e)))?;
        }
        LogFormat::Compact => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(config.include_location)
                .with_line_number(config.include_location)
                .with_ansi(config.use_colors)
                .compact();

            subscriber
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| EclError::Configuration(format!("Failed to initialize logging: {}", e)))?;
        }
    }

    tracing::info!(
        level = ?config.level,
        format = ?config.format,
        "Logging initialized"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert!(matches!(config.level, LogLevel::Info));
        assert!(matches!(config.format, LogFormat::Pretty));
        assert!(config.use_colors);
    }

    #[test]
    fn test_log_config_dev() {
        let config = LogConfig::dev();
        assert!(matches!(config.level, LogLevel::Debug));
        assert!(matches!(config.format, LogFormat::Pretty));
        assert!(config.use_colors);
    }

    #[test]
    fn test_log_config_prod() {
        let config = LogConfig::prod();
        assert!(matches!(config.level, LogLevel::Info));
        assert!(matches!(config.format, LogFormat::Json));
        assert!(!config.use_colors);
        assert!(config.include_location);
    }
}
```

### 4. SQL Exporter (Optional, `logging/exporter.rs`)

Initial investigation and optional implementation:

```rust
//! SQL exporter for structured logs (optional feature).
//!
//! This module provides an experimental SQL-based log exporter that stores
//! structured log messages in a database for long-term retention and analysis.
//!
//! # Status
//!
//! This is an experimental feature. The implementation should be evaluated for:
//! - Performance overhead
//! - Compatibility with twyg
//! - Production readiness
//!
//! Consider submitting enhancements to the twyg project if this proves valuable.

use crate::error::EclResult;
use sqlx::{Pool, Postgres};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// SQL exporter layer for tracing.
///
/// This is a proof-of-concept implementation that writes log events to a database.
#[derive(Clone)]
pub struct SqlExporterLayer {
    pool: Pool<Postgres>,
}

impl SqlExporterLayer {
    /// Create a new SQL exporter layer.
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

// TODO: Implement tracing_subscriber::Layer trait
// This is intentionally incomplete - needs evaluation and design

impl<S> Layer<S> for SqlExporterLayer
where
    S: tracing::Subscriber,
{
    // Basic implementation skeleton
    // Would need to:
    // 1. Capture log events
    // 2. Extract structured fields
    // 3. Batch writes to database
    // 4. Handle backpressure
    // 5. Ensure minimal performance overhead
}

/// Initialize database schema for log storage.
pub async fn init_log_schema(pool: &Pool<Postgres>) -> EclResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS log_events (
            id BIGSERIAL PRIMARY KEY,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            level TEXT NOT NULL,
            target TEXT NOT NULL,
            message TEXT NOT NULL,

            -- Structured fields
            workflow_id UUID,
            step_id UUID,

            -- Metadata
            fields JSONB,

            -- Performance
            duration_ms BIGINT,
            tokens_used INTEGER,

            -- Indexing
            INDEX idx_workflow_id (workflow_id),
            INDEX idx_timestamp (timestamp),
            INDEX idx_level (level)
        );
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

// TODO: Evaluate if this should be contributed to twyg
// See: https://github.com/oxur/twyg
```

### 5. Integration with Step Executor

Update `ecl-core/src/step/executor.rs` to use structured logging:

```rust
use crate::logging::{StepSpan, record_performance, record_retry};

impl StepExecutor {
    pub async fn execute<S: Step>(&self, step: &S, ctx: &StepContext) -> StepExecution {
        let start = std::time::Instant::now();

        // Create step span for correlation
        let span = StepSpan::new(
            ctx.workflow_id,
            S::step_id(),
            S::name(),
        ).enter();
        let _guard = span.enter();

        tracing::info!("Step execution started");

        // Execute with retry logic
        let result = self.execute_with_retry(step, ctx).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match &result {
            Ok(output) => {
                record_performance(duration_ms, None);
                tracing::info!("Step execution succeeded");
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    duration_ms = duration_ms,
                    "Step execution failed"
                );
            }
        }

        result
    }

    async fn execute_with_retry<S: Step>(&self, step: &S, ctx: &StepContext) -> StepExecution {
        let mut attempts = 0;
        let max_attempts = self.retry_policy.max_attempts;

        loop {
            attempts += 1;

            let span = StepSpan::new(
                ctx.workflow_id,
                S::step_id(),
                S::name(),
            )
            .attempt(attempts)
            .enter();
            let _guard = span.enter();

            match step.execute(ctx).await {
                Ok(output) => {
                    if attempts > 1 {
                        tracing::info!(
                            attempts = attempts,
                            "Step succeeded after retry"
                        );
                    }
                    return Ok(output);
                }
                Err(e) if attempts < max_attempts && self.should_retry(&e) => {
                    record_retry(attempts, max_attempts, &e.to_string());

                    // Exponential backoff
                    let delay = self.retry_policy.backoff_base_ms * 2u64.pow(attempts - 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
                Err(e) => {
                    tracing::error!(
                        attempts = attempts,
                        error = %e,
                        "Step failed after all retry attempts"
                    );
                    return Err(e);
                }
            }
        }
    }
}
```

### 6. Integration with Workflow Orchestrator

Update `ecl-core/src/workflow/orchestrator.rs`:

```rust
use crate::logging::{WorkflowSpan, record_performance};

impl WorkflowOrchestrator {
    pub async fn execute(&self, workflow_id: WorkflowId) -> EclResult<()> {
        let start = std::time::Instant::now();

        // Create workflow span
        let span = WorkflowSpan::new(
            workflow_id,
            &self.definition.name,
        ).enter();
        let _guard = span.enter();

        tracing::info!(
            step_count = self.execution_order.len(),
            "Workflow execution started"
        );

        // Execute workflow
        let result = self.execute_steps(workflow_id).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match &result {
            Ok(_) => {
                record_performance(duration_ms, None);
                tracing::info!("Workflow execution completed successfully");
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    duration_ms = duration_ms,
                    "Workflow execution failed"
                );
            }
        }

        result
    }
}
```

### 7. Integration with LLM Provider

Update `ecl-core/src/llm/provider.rs`:

```rust
use crate::logging::{LlmSpan, record_performance};

impl ClaudeProvider {
    pub async fn complete(&self, request: CompletionRequest) -> EclResult<CompletionResponse> {
        let start = std::time::Instant::now();

        // Create LLM span
        let span = LlmSpan::new(request.workflow_id, &self.model).enter();
        let _guard = span.enter();

        tracing::debug!(
            model = %self.model,
            message_count = request.messages.len(),
            "LLM request started"
        );

        // Make API call
        let result = self.call_api(&request).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match &result {
            Ok(response) => {
                record_performance(duration_ms, Some(response.tokens_used.total()));
                tracing::debug!(
                    tokens_input = response.tokens_used.input,
                    tokens_output = response.tokens_used.output,
                    "LLM request completed"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    duration_ms = duration_ms,
                    "LLM request failed"
                );
            }
        }

        result
    }
}
```

### 8. Logging Patterns

Establish consistent patterns throughout the codebase:

#### Workflow Start/End

```rust
tracing::info!(
    workflow_id = %workflow_id,
    workflow_name = %name,
    "Workflow execution started"
);
```

#### Step Execution

```rust
tracing::info!(
    workflow_id = %workflow_id,
    step_id = %step_id,
    step_name = %name,
    attempt = attempt,
    "Step execution started"
);
```

#### Validation Failures

```rust
tracing::warn!(
    field = %field_name,
    reason = %reason,
    "Validation failed"
);
```

#### Retry Attempts

```rust
tracing::warn!(
    attempt = attempt,
    max_attempts = max_attempts,
    error = %error,
    "Retrying after failure"
);
```

#### Performance Metrics

```rust
tracing::info!(
    duration_ms = duration,
    tokens_used = tokens,
    "Operation completed"
);
```

#### Error Logging

```rust
tracing::error!(
    error = %e,
    context = %additional_info,
    "Operation failed"
);
```

---

## Testing Strategy

### Unit Tests

1. **Configuration Tests** (`logging/config.rs`):

   ```rust
   #[test]
   fn test_log_config_defaults() {
       let config = LogConfig::default();
       assert!(matches!(config.level, LogLevel::Info));
   }

   #[test]
   fn test_log_config_dev_vs_prod() {
       let dev = LogConfig::dev();
       let prod = LogConfig::prod();
       assert!(dev.use_colors);
       assert!(!prod.use_colors);
       assert!(matches!(prod.format, LogFormat::Json));
   }

   #[test]
   fn test_log_level_conversion() {
       let level: tracing::Level = LogLevel::Debug.into();
       assert_eq!(level, tracing::Level::DEBUG);
   }
   ```

2. **Span Builder Tests** (`logging/spans.rs`):

   ```rust
   #[test]
   fn test_workflow_span_creation() {
       let workflow_id = WorkflowId::new();
       let span = WorkflowSpan::new(workflow_id, "test-workflow");
       // Verify span can be created without panicking
       let _guard = span.enter().enter();
   }

   #[test]
   fn test_step_span_with_attempt() {
       let workflow_id = WorkflowId::new();
       let step_id = StepId::new();
       let span = StepSpan::new(workflow_id, step_id, "test-step")
           .attempt(2);
       // Verify attempt tracking
       let _guard = span.enter().enter();
   }
   ```

### Integration Tests

1. **End-to-End Logging Test**:

   ```rust
   #[tokio::test]
   async fn test_workflow_logging_correlation() {
       // Initialize test logging
       let config = LogConfig::dev();
       let _ = init_logging(config);

       // Create workflow and execute
       let workflow_id = WorkflowId::new();
       let span = WorkflowSpan::new(workflow_id, "test").enter();
       let _guard = span.enter();

       // Log at various levels
       tracing::info!("Workflow started");
       tracing::debug!("Debug information");
       tracing::warn!("Warning message");

       // All logs should be correlated by workflow_id
       // (Manual verification in test output)
   }
   ```

2. **Performance Overhead Test**:

   ```rust
   #[tokio::test]
   async fn test_logging_performance_overhead() {
       let config = LogConfig::prod();
       let _ = init_logging(config);

       // Measure execution time with logging
       let start = std::time::Instant::now();
       for i in 0..1000 {
           tracing::info!(iteration = i, "Test log");
       }
       let with_logging = start.elapsed();

       // Overhead should be minimal (<5% of operation time)
       // In practice, measure against actual workflow execution
       assert!(with_logging.as_millis() < 100);
   }
   ```

3. **Span Hierarchy Test**:

   ```rust
   #[tokio::test]
   async fn test_nested_span_hierarchy() {
       let config = LogConfig::dev();
       let _ = init_logging(config);

       let workflow_id = WorkflowId::new();

       // Workflow span
       let workflow_span = WorkflowSpan::new(workflow_id, "test").enter();
       let _wf_guard = workflow_span.enter();
       tracing::info!("Workflow level");

       // Step span (nested)
       let step_id = StepId::new();
       let step_span = StepSpan::new(workflow_id, step_id, "step1").enter();
       let _step_guard = step_span.enter();
       tracing::info!("Step level");

       // LLM span (nested further)
       let llm_span = LlmSpan::new(workflow_id, "claude")
           .step_id(step_id)
           .enter();
       let _llm_guard = llm_span.enter();
       tracing::info!("LLM level");

       // Verify hierarchical structure in output
   }
   ```

### Manual Testing

1. **Console Output Verification**:
   - Run workflow with `LogFormat::Pretty` in dev
   - Verify colored, readable output
   - Check that structured fields are visible

2. **JSON Format Validation**:
   - Run workflow with `LogFormat::Json` in prod
   - Parse output with `jq` to verify structure
   - Confirm all fields are present and parseable

3. **Log Correlation**:
   - Run multiple concurrent workflows
   - Verify logs can be filtered by `workflow_id`
   - Confirm no mixing of workflow logs

4. **Performance Measurement**:
   - Run workflow with and without detailed logging
   - Measure execution time difference
   - Ensure overhead is <5%

---

## SQL Exporter Investigation

### Tasks

1. **Evaluate twyg SQL Capabilities**:
   - Review twyg documentation for SQL export features
   - Test existing SQL exporters (if any)
   - Identify gaps or limitations

2. **Prototype SQL Layer**:
   - Implement basic `SqlExporterLayer` as shown above
   - Test with SQLite for development
   - Measure performance overhead

3. **Performance Benchmarking**:
   - Compare async vs sync writes
   - Test batch sizes (100, 1000, 10000 logs)
   - Measure query performance for log retrieval

4. **Determine Viability**:
   - If overhead <5%: Implement fully
   - If overhead 5-10%: Make optional feature
   - If overhead >10%: Document as future work

5. **Potential PR to twyg**:
   - If implementation is solid and useful
   - Document findings and approach
   - Submit enhancement PR to twyg project
   - Link: <https://github.com/oxur/twyg>

---

## Potential Blockers

1. **Tracing Initialization Conflicts**:
   - **Issue**: Multiple calls to `init_logging` will fail
   - **Mitigation**: Use `std::sync::Once` or document single initialization
   - **Solution**: Add `try_init()` that returns Ok if already initialized

2. **Performance Overhead**:
   - **Issue**: Excessive logging may slow execution
   - **Mitigation**: Use appropriate log levels, async exporters
   - **Solution**: Benchmark and optimize hot paths

3. **Log Volume in Production**:
   - **Issue**: High-volume workflows may generate excessive logs
   - **Mitigation**: Use sampling, log rotation, external aggregation
   - **Solution**: Document log retention policies

4. **SQL Exporter Complexity**:
   - **Issue**: Implementing robust SQL export is non-trivial
   - **Mitigation**: Start with proof-of-concept, make optional
   - **Solution**: Focus on core observability first, SQL as stretch goal

5. **Structured Field Consistency**:
   - **Issue**: Inconsistent field names across codebase
   - **Mitigation**: Document field naming conventions
   - **Solution**: Use span builders to enforce consistency

---

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

## Estimated Effort

**Total: 3-4 hours**

Breakdown:

- Logging configuration: 30 minutes
- Span builders: 1 hour
- Integration with executor/orchestrator/LLM: 1 hour
- Testing and documentation: 1 hour
- SQL exporter investigation: 30 minutes (optional)

---

## Implementation Order

1. Create `ecl-core/src/logging/` module structure
2. Implement `LogConfig` and `init_logging()`
3. Implement span builders (`spans.rs`)
4. Update `main.rs` to initialize logging
5. Integrate with `StepExecutor`
6. Integrate with `WorkflowOrchestrator`
7. Integrate with `ClaudeProvider`
8. Write unit tests for config and spans
9. Write integration tests for correlation
10. Measure and document performance overhead
11. (Optional) Investigate SQL exporter
12. Update documentation with logging patterns

---

## Files to Create/Modify

### Create

- `crates/ecl-core/src/logging/mod.rs`
- `crates/ecl-core/src/logging/config.rs`
- `crates/ecl-core/src/logging/spans.rs`
- `crates/ecl-core/src/logging/exporter.rs` (optional)
- `crates/ecl-core/tests/logging_integration.rs`

### Modify

- `crates/ecl-core/src/lib.rs` (add `pub mod logging;`)
- `crates/ecl-core/src/step/executor.rs` (add span integration)
- `crates/ecl-core/src/workflow/orchestrator.rs` (add span integration)
- `crates/ecl-core/src/llm/claude.rs` (add LLM span integration)
- `crates/ecl-workflows/src/main.rs` (initialize logging)
- `crates/ecl-workflows/src/simple.rs` (use structured logging)
- `crates/ecl-workflows/src/critique_loop.rs` (use structured logging)

---

## Documentation

### Rustdoc Examples

All public items should include examples:

```rust
/// Initialize the logging system.
///
/// # Examples
///
/// ```no_run
/// use ecl_core::logging::{init_logging, LogConfig};
///
/// let config = LogConfig::dev();
/// init_logging(config).expect("Failed to initialize logging");
/// ```
pub fn init_logging(config: LogConfig) -> EclResult<()> { ... }
```

### Logging Patterns Guide

Document in `crates/ecl-core/src/logging/mod.rs`:

```rust
//! # Logging Patterns
//!
//! ## Workflow-Level Logging
//!
//! ```no_run
//! use ecl_core::logging::WorkflowSpan;
//!
//! let span = WorkflowSpan::new(workflow_id, "my-workflow").enter();
//! let _guard = span.enter();
//! tracing::info!("Workflow started");
//! ```
//!
//! ## Step-Level Logging
//!
//! ```no_run
//! use ecl_core::logging::StepSpan;
//!
//! let span = StepSpan::new(workflow_id, step_id, "my-step")
//!     .attempt(retry_count)
//!     .enter();
//! let _guard = span.enter();
//! tracing::info!("Step executing");
//! ```
```

---

## Success Metrics

1. **Coverage**: Logging module test coverage ≥95%
2. **Performance**: Overhead <5% in production workflows
3. **Correlation**: 100% of operations log workflow_id
4. **Usability**: Dev logs readable, prod logs parseable by machines
5. **Consistency**: All workflow/step/LLM operations follow patterns

---

## Next Steps

After completing Stage 2.6:

1. Verify logging works end-to-end in Critique-Revise workflow
2. Measure performance overhead and optimize if needed
3. Document logging patterns for team
4. Proceed to **Stage 2.7: Toy Workflow Integration**

---

**Next:** [Stage 2.7: Toy Workflow Integration](./0009-phase-2-07-integration.md)

---

**Document Version:** 1.0
**Last Updated:** 2026-01-23
**Status:** Ready for Implementation
