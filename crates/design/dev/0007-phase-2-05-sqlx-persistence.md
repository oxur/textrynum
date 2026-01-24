# Stage 2.5: SQLx Persistence Layer

**Version:** 1.0
**Date:** 2026-01-23
**Phase:** 2 - Step Abstraction Framework
**Stage:** 2.5 - SQLx Persistence Layer
**Estimated Duration:** 5-6 hours
**Status:** Ready for Implementation

---

## Overview

This stage adds database persistence for workflow state and metadata using SQLx. The persistence layer enables workflow resumption, audit trails, and execution history tracking.

**Goal**: Add robust database persistence for workflow state and metadata with compile-time query validation.

**Key Features**:
- Database schema for workflows, step executions, and artifacts
- SQLx migrations supporting both SQLite (dev) and Postgres (prod)
- Repository pattern for clean data access
- Compile-time query validation via SQLx CLI
- In-memory SQLite for fast testing

---

## Dependencies

**Prerequisite Stages**:
- ✅ **Stage 2.1**: Step trait definition provides execution model
- ✅ **Stage 2.2**: Step executor provides StepExecution type
- ✅ **Stage 2.3**: Toy workflow steps provide concrete implementations
- ✅ **Stage 2.4**: Workflow orchestrator provides WorkflowDefinition

**Required Types from Previous Stages**:
- `WorkflowId`, `StepId` from Stage 2.1
- `StepExecution` from Stage 2.2
- `WorkflowDefinition`, `WorkflowState` from Stage 2.4

---

## Detailed Implementation Steps

### Step 1: Database Schema Design

Create comprehensive schema supporting all persistence needs.

**Files to Create**:

#### `migrations/001_initial_schema.sql`

```sql
-- ============================================================================
-- ECL Workflow Persistence Schema
-- Version: 1.0
-- Target: SQLite and PostgreSQL (compatible subset)
-- ============================================================================

-- Workflows table: Core workflow state and metadata
CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,                    -- UUID as TEXT for SQLite compat
    definition_id TEXT NOT NULL,            -- References WorkflowDefinition
    state TEXT NOT NULL,                    -- WorkflowState: Pending/Running/Completed/Failed
    input TEXT NOT NULL,                    -- JSON serialized input
    output TEXT,                            -- JSON serialized output (nullable)
    error TEXT,                             -- Error message if failed (nullable)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),  -- ISO 8601 timestamp
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),  -- ISO 8601 timestamp
    completed_at TEXT,                      -- Completion timestamp (nullable)
    metadata TEXT                           -- JSON metadata (tags, labels, etc.)
);

-- Step executions table: Detailed execution history for each step
CREATE TABLE IF NOT EXISTS step_executions (
    id TEXT PRIMARY KEY,                    -- UUID as TEXT
    workflow_id TEXT NOT NULL,              -- Foreign key to workflows
    step_id TEXT NOT NULL,                  -- Step identifier from definition
    attempt INTEGER NOT NULL DEFAULT 1,     -- Retry attempt number (1-indexed)
    state TEXT NOT NULL,                    -- Pending/Running/Succeeded/Failed
    input TEXT NOT NULL,                    -- JSON serialized step input
    output TEXT,                            -- JSON serialized step output (nullable)
    error TEXT,                             -- Error message if failed (nullable)
    started_at TEXT NOT NULL,               -- Execution start timestamp
    completed_at TEXT,                      -- Execution completion timestamp (nullable)
    duration_ms INTEGER,                    -- Execution duration in milliseconds
    tokens_used INTEGER,                    -- LLM tokens consumed (nullable)
    metadata TEXT,                          -- JSON metadata (traces, logs, etc.)
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);

-- Artifacts table: File artifacts produced by workflow steps
-- Placeholder for Phase 3 full implementation
CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY,                    -- UUID as TEXT
    workflow_id TEXT NOT NULL,              -- Foreign key to workflows
    step_id TEXT NOT NULL,                  -- Step that created artifact
    name TEXT NOT NULL,                     -- Artifact name/key
    content_type TEXT NOT NULL,             -- MIME type (text/plain, application/json, etc.)
    path TEXT NOT NULL,                     -- File system or S3 path
    size_bytes INTEGER,                     -- Artifact size
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata TEXT,                          -- JSON metadata
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_workflows_definition_id ON workflows(definition_id);
CREATE INDEX IF NOT EXISTS idx_workflows_state ON workflows(state);
CREATE INDEX IF NOT EXISTS idx_workflows_created_at ON workflows(created_at);
CREATE INDEX IF NOT EXISTS idx_step_executions_workflow_id ON step_executions(workflow_id);
CREATE INDEX IF NOT EXISTS idx_step_executions_step_id ON step_executions(step_id);
CREATE INDEX IF NOT EXISTS idx_step_executions_state ON step_executions(state);
CREATE INDEX IF NOT EXISTS idx_artifacts_workflow_id ON artifacts(workflow_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_step_id ON artifacts(step_id);
```

**Design Notes**:
- Uses TEXT for UUIDs and timestamps to ensure SQLite/Postgres compatibility
- All JSON stored as TEXT (parsed in Rust layer)
- Cascade deletes ensure referential integrity
- Indexes optimize common query patterns (filtering by state, workflow lookup, etc.)

---

### Step 2: SQLx Migration Setup

Configure SQLx migrations for both databases.

**Files to Create**:

#### `.sqlx/migrations.toml`

```toml
# SQLx migration configuration
# Supports both SQLite and PostgreSQL

[migrations]
# Migration file directory
migrations_dir = "migrations"

# Reversible migrations disabled (use forward-only migrations)
reversible = false
```

**Configuration in `ecl-core/Cargo.toml`**:

Add SQLx build-time verification:

```toml
[dependencies]
sqlx = { workspace = true, features = ["runtime-tokio", "sqlite", "postgres", "json", "uuid", "chrono"] }

[build-dependencies]
# Optional: Enable compile-time query verification
# Requires DATABASE_URL set in .env
```

**Environment Configuration**:

Create `.env.example`:

```env
# Development: SQLite
DATABASE_URL=sqlite://./ecl.db

# Production: PostgreSQL
# DATABASE_URL=postgres://user:password@localhost/ecl
```

**Migration Commands**:

```bash
# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features sqlite,postgres

# Create database (SQLite)
sqlx database create

# Run migrations
sqlx migrate run

# Revert last migration (if needed during dev)
sqlx migrate revert

# Prepare offline query metadata (for CI/CD without database)
cargo sqlx prepare
```

---

### Step 3: Database Error Types

Define custom error types for database operations.

**Files to Create**:

#### `ecl-core/src/db/error.rs`

```rust
use thiserror::Error;

/// Database operation errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DbError {
    /// Database connection failed
    #[error("database connection failed: {0}")]
    ConnectionFailed(#[from] sqlx::Error),

    /// Entity not found by ID
    #[error("entity not found: {entity_type} with id {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    /// Constraint violation (e.g., unique constraint, foreign key)
    #[error("database constraint violation: {0}")]
    ConstraintViolation(String),

    /// Serialization error when converting to/from JSON
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Database query execution error
    #[error("query execution failed: {0}")]
    QueryFailed(String),

    /// Transaction error
    #[error("transaction failed: {0}")]
    TransactionFailed(String),

    /// Migration error
    #[error("migration failed: {0}")]
    MigrationFailed(String),
}

/// Type alias for database operation results
pub type DbResult<T> = Result<T, DbError>;

impl DbError {
    /// Check if error is a not-found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, DbError::NotFound { .. })
    }

    /// Check if error is a constraint violation
    pub fn is_constraint_violation(&self) -> bool {
        matches!(self, DbError::ConstraintViolation(_))
    }
}
```

---

### Step 4: WorkflowRepository Implementation

Implement repository for workflow CRUD operations.

**Files to Create**:

#### `ecl-core/src/db/workflow_repo.rs`

```rust
use crate::db::error::{DbError, DbResult};
use crate::types::{WorkflowId, StepId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};
use uuid::Uuid;

/// Workflow state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowState {
    /// Workflow is pending execution
    Pending,
    /// Workflow is currently running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed with error
    Failed,
}

impl WorkflowState {
    /// Convert to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            WorkflowState::Pending => "pending",
            WorkflowState::Running => "running",
            WorkflowState::Completed => "completed",
            WorkflowState::Failed => "failed",
        }
    }

    /// Parse from database string
    pub fn from_db_string(s: &str) -> Result<Self, DbError> {
        match s {
            "pending" => Ok(WorkflowState::Pending),
            "running" => Ok(WorkflowState::Running),
            "completed" => Ok(WorkflowState::Completed),
            "failed" => Ok(WorkflowState::Failed),
            _ => Err(DbError::QueryFailed(format!("invalid workflow state: {}", s))),
        }
    }
}

/// Workflow entity stored in database
#[derive(Debug, Clone, FromRow)]
pub struct WorkflowRecord {
    pub id: String,
    pub definition_id: String,
    pub state: String,
    pub input: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub metadata: Option<String>,
}

/// Workflow data transfer object for application layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: WorkflowId,
    pub definition_id: String,
    pub state: WorkflowState,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

impl Workflow {
    /// Convert from database record
    fn from_record(record: WorkflowRecord) -> DbResult<Self> {
        Ok(Self {
            id: WorkflowId::from(Uuid::parse_str(&record.id).map_err(|e| {
                DbError::QueryFailed(format!("invalid UUID: {}", e))
            })?),
            definition_id: record.definition_id,
            state: WorkflowState::from_db_string(&record.state)?,
            input: serde_json::from_str(&record.input)?,
            output: record.output.as_deref().map(serde_json::from_str).transpose()?,
            error: record.error,
            created_at: DateTime::parse_from_rfc3339(&record.created_at)
                .map_err(|e| DbError::QueryFailed(format!("invalid timestamp: {}", e)))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&record.updated_at)
                .map_err(|e| DbError::QueryFailed(format!("invalid timestamp: {}", e)))?
                .with_timezone(&Utc),
            completed_at: record.completed_at
                .as_deref()
                .map(|s| DateTime::parse_from_rfc3339(s))
                .transpose()
                .map_err(|e| DbError::QueryFailed(format!("invalid timestamp: {}", e)))?
                .map(|dt| dt.with_timezone(&Utc)),
            metadata: record.metadata.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

/// Repository for workflow persistence operations
pub struct WorkflowRepository {
    pool: Pool<Sqlite>,
}

impl WorkflowRepository {
    /// Create new repository with database connection pool
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create new workflow record
    pub async fn create(&self, workflow: &Workflow) -> DbResult<()> {
        let id = workflow.id.to_string();
        let state = workflow.state.to_db_string();
        let input = serde_json::to_string(&workflow.input)?;
        let output = workflow.output.as_ref().map(|v| serde_json::to_string(v)).transpose()?;
        let created_at = workflow.created_at.to_rfc3339();
        let updated_at = workflow.updated_at.to_rfc3339();
        let completed_at = workflow.completed_at.map(|dt| dt.to_rfc3339());
        let metadata = workflow.metadata.as_ref().map(|v| serde_json::to_string(v)).transpose()?;

        sqlx::query!(
            r#"
            INSERT INTO workflows (id, definition_id, state, input, output, error, created_at, updated_at, completed_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            id,
            workflow.definition_id,
            state,
            input,
            output,
            workflow.error,
            created_at,
            updated_at,
            completed_at,
            metadata,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to insert workflow: {}", e)))?;

        Ok(())
    }

    /// Get workflow by ID
    pub async fn get(&self, id: &WorkflowId) -> DbResult<Option<Workflow>> {
        let id_str = id.to_string();

        let record = sqlx::query_as!(
            WorkflowRecord,
            r#"
            SELECT id, definition_id, state, input, output, error, created_at, updated_at, completed_at, metadata
            FROM workflows
            WHERE id = ?1
            "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to query workflow: {}", e)))?;

        record.map(Workflow::from_record).transpose()
    }

    /// Update workflow state
    pub async fn update_state(
        &self,
        id: &WorkflowId,
        state: WorkflowState,
        error: Option<&str>,
    ) -> DbResult<()> {
        let id_str = id.to_string();
        let state_str = state.to_db_string();
        let updated_at = Utc::now().to_rfc3339();
        let completed_at = matches!(state, WorkflowState::Completed | WorkflowState::Failed)
            .then(|| Utc::now().to_rfc3339());

        let result = sqlx::query!(
            r#"
            UPDATE workflows
            SET state = ?1, error = ?2, updated_at = ?3, completed_at = ?4
            WHERE id = ?5
            "#,
            state_str,
            error,
            updated_at,
            completed_at,
            id_str
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to update workflow state: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                entity_type: "workflow",
                id: id_str,
            });
        }

        Ok(())
    }

    /// Update workflow output
    pub async fn update_output(
        &self,
        id: &WorkflowId,
        output: &serde_json::Value,
    ) -> DbResult<()> {
        let id_str = id.to_string();
        let output_str = serde_json::to_string(output)?;
        let updated_at = Utc::now().to_rfc3339();

        let result = sqlx::query!(
            r#"
            UPDATE workflows
            SET output = ?1, updated_at = ?2
            WHERE id = ?3
            "#,
            output_str,
            updated_at,
            id_str
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to update workflow output: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                entity_type: "workflow",
                id: id_str,
            });
        }

        Ok(())
    }

    /// List workflows by state
    pub async fn list_by_state(&self, state: WorkflowState) -> DbResult<Vec<Workflow>> {
        let state_str = state.to_db_string();

        let records = sqlx::query_as!(
            WorkflowRecord,
            r#"
            SELECT id, definition_id, state, input, output, error, created_at, updated_at, completed_at, metadata
            FROM workflows
            WHERE state = ?1
            ORDER BY created_at DESC
            "#,
            state_str
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to list workflows: {}", e)))?;

        records.into_iter().map(Workflow::from_record).collect()
    }

    /// List workflows by definition ID
    pub async fn list_by_definition(&self, definition_id: &str) -> DbResult<Vec<Workflow>> {
        let records = sqlx::query_as!(
            WorkflowRecord,
            r#"
            SELECT id, definition_id, state, input, output, error, created_at, updated_at, completed_at, metadata
            FROM workflows
            WHERE definition_id = ?1
            ORDER BY created_at DESC
            "#,
            definition_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to list workflows: {}", e)))?;

        records.into_iter().map(Workflow::from_record).collect()
    }

    /// Delete workflow and all associated data (cascade)
    pub async fn delete(&self, id: &WorkflowId) -> DbResult<()> {
        let id_str = id.to_string();

        let result = sqlx::query!(
            r#"
            DELETE FROM workflows WHERE id = ?1
            "#,
            id_str
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to delete workflow: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                entity_type: "workflow",
                id: id_str,
            });
        }

        Ok(())
    }
}
```

---

### Step 5: StepExecutionRepository Implementation

Implement repository for step execution history.

**Files to Create**:

#### `ecl-core/src/db/step_execution_repo.rs`

```rust
use crate::db::error::{DbError, DbResult};
use crate::types::{WorkflowId, StepId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};
use uuid::Uuid;

/// Step execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StepExecutionState {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl StepExecutionState {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            StepExecutionState::Pending => "pending",
            StepExecutionState::Running => "running",
            StepExecutionState::Succeeded => "succeeded",
            StepExecutionState::Failed => "failed",
        }
    }

    pub fn from_db_string(s: &str) -> Result<Self, DbError> {
        match s {
            "pending" => Ok(StepExecutionState::Pending),
            "running" => Ok(StepExecutionState::Running),
            "succeeded" => Ok(StepExecutionState::Succeeded),
            "failed" => Ok(StepExecutionState::Failed),
            _ => Err(DbError::QueryFailed(format!("invalid step state: {}", s))),
        }
    }
}

/// Step execution record from database
#[derive(Debug, Clone, FromRow)]
pub struct StepExecutionRecord {
    pub id: String,
    pub workflow_id: String,
    pub step_id: String,
    pub attempt: i64,
    pub state: String,
    pub input: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub tokens_used: Option<i64>,
    pub metadata: Option<String>,
}

/// Step execution data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecution {
    pub id: Uuid,
    pub workflow_id: WorkflowId,
    pub step_id: StepId,
    pub attempt: u32,
    pub state: StepExecutionState,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub tokens_used: Option<u64>,
    pub metadata: Option<serde_json::Value>,
}

impl StepExecution {
    fn from_record(record: StepExecutionRecord) -> DbResult<Self> {
        Ok(Self {
            id: Uuid::parse_str(&record.id)
                .map_err(|e| DbError::QueryFailed(format!("invalid UUID: {}", e)))?,
            workflow_id: WorkflowId::from(
                Uuid::parse_str(&record.workflow_id)
                    .map_err(|e| DbError::QueryFailed(format!("invalid UUID: {}", e)))?,
            ),
            step_id: StepId::from(record.step_id),
            attempt: record.attempt as u32,
            state: StepExecutionState::from_db_string(&record.state)?,
            input: serde_json::from_str(&record.input)?,
            output: record.output.as_deref().map(serde_json::from_str).transpose()?,
            error: record.error,
            started_at: DateTime::parse_from_rfc3339(&record.started_at)
                .map_err(|e| DbError::QueryFailed(format!("invalid timestamp: {}", e)))?
                .with_timezone(&Utc),
            completed_at: record
                .completed_at
                .as_deref()
                .map(|s| DateTime::parse_from_rfc3339(s))
                .transpose()
                .map_err(|e| DbError::QueryFailed(format!("invalid timestamp: {}", e)))?
                .map(|dt| dt.with_timezone(&Utc)),
            duration_ms: record.duration_ms,
            tokens_used: record.tokens_used.map(|t| t as u64),
            metadata: record.metadata.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

/// Repository for step execution persistence
pub struct StepExecutionRepository {
    pool: Pool<Sqlite>,
}

impl StepExecutionRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Record new step execution
    pub async fn create(&self, execution: &StepExecution) -> DbResult<()> {
        let id = execution.id.to_string();
        let workflow_id = execution.workflow_id.to_string();
        let step_id = execution.step_id.as_str();
        let attempt = execution.attempt as i64;
        let state = execution.state.to_db_string();
        let input = serde_json::to_string(&execution.input)?;
        let output = execution.output.as_ref().map(|v| serde_json::to_string(v)).transpose()?;
        let started_at = execution.started_at.to_rfc3339();
        let completed_at = execution.completed_at.map(|dt| dt.to_rfc3339());
        let tokens_used = execution.tokens_used.map(|t| t as i64);
        let metadata = execution.metadata.as_ref().map(|v| serde_json::to_string(v)).transpose()?;

        sqlx::query!(
            r#"
            INSERT INTO step_executions
            (id, workflow_id, step_id, attempt, state, input, output, error, started_at, completed_at, duration_ms, tokens_used, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            id,
            workflow_id,
            step_id,
            attempt,
            state,
            input,
            output,
            execution.error,
            started_at,
            completed_at,
            execution.duration_ms,
            tokens_used,
            metadata,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to insert step execution: {}", e)))?;

        Ok(())
    }

    /// Get step execution by ID
    pub async fn get(&self, id: &Uuid) -> DbResult<Option<StepExecution>> {
        let id_str = id.to_string();

        let record = sqlx::query_as!(
            StepExecutionRecord,
            r#"
            SELECT id, workflow_id, step_id, attempt, state, input, output, error,
                   started_at, completed_at, duration_ms, tokens_used, metadata
            FROM step_executions
            WHERE id = ?1
            "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to query step execution: {}", e)))?;

        record.map(StepExecution::from_record).transpose()
    }

    /// Get all step executions for a workflow
    pub async fn get_for_workflow(&self, workflow_id: &WorkflowId) -> DbResult<Vec<StepExecution>> {
        let workflow_id_str = workflow_id.to_string();

        let records = sqlx::query_as!(
            StepExecutionRecord,
            r#"
            SELECT id, workflow_id, step_id, attempt, state, input, output, error,
                   started_at, completed_at, duration_ms, tokens_used, metadata
            FROM step_executions
            WHERE workflow_id = ?1
            ORDER BY started_at ASC
            "#,
            workflow_id_str
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to list step executions: {}", e)))?;

        records.into_iter().map(StepExecution::from_record).collect()
    }

    /// Get step executions for specific step in workflow
    pub async fn get_for_step(
        &self,
        workflow_id: &WorkflowId,
        step_id: &StepId,
    ) -> DbResult<Vec<StepExecution>> {
        let workflow_id_str = workflow_id.to_string();
        let step_id_str = step_id.as_str();

        let records = sqlx::query_as!(
            StepExecutionRecord,
            r#"
            SELECT id, workflow_id, step_id, attempt, state, input, output, error,
                   started_at, completed_at, duration_ms, tokens_used, metadata
            FROM step_executions
            WHERE workflow_id = ?1 AND step_id = ?2
            ORDER BY attempt ASC
            "#,
            workflow_id_str,
            step_id_str
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to list step executions: {}", e)))?;

        records.into_iter().map(StepExecution::from_record).collect()
    }

    /// Update step execution state and completion details
    pub async fn update_completion(
        &self,
        id: &Uuid,
        state: StepExecutionState,
        output: Option<&serde_json::Value>,
        error: Option<&str>,
        duration_ms: i64,
    ) -> DbResult<()> {
        let id_str = id.to_string();
        let state_str = state.to_db_string();
        let output_str = output.map(|v| serde_json::to_string(v)).transpose()?;
        let completed_at = Utc::now().to_rfc3339();

        let result = sqlx::query!(
            r#"
            UPDATE step_executions
            SET state = ?1, output = ?2, error = ?3, completed_at = ?4, duration_ms = ?5
            WHERE id = ?6
            "#,
            state_str,
            output_str,
            error,
            completed_at,
            duration_ms,
            id_str
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to update step execution: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                entity_type: "step_execution",
                id: id_str,
            });
        }

        Ok(())
    }

    /// Get execution statistics for a workflow
    pub async fn get_stats(&self, workflow_id: &WorkflowId) -> DbResult<ExecutionStats> {
        let workflow_id_str = workflow_id.to_string();

        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_executions,
                SUM(CASE WHEN state = 'succeeded' THEN 1 ELSE 0 END) as succeeded_count,
                SUM(CASE WHEN state = 'failed' THEN 1 ELSE 0 END) as failed_count,
                SUM(duration_ms) as total_duration_ms,
                SUM(tokens_used) as total_tokens
            FROM step_executions
            WHERE workflow_id = ?1
            "#,
            workflow_id_str
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryFailed(format!("failed to query execution stats: {}", e)))?;

        Ok(ExecutionStats {
            total_executions: stats.total_executions as u64,
            succeeded_count: stats.succeeded_count.unwrap_or(0) as u64,
            failed_count: stats.failed_count.unwrap_or(0) as u64,
            total_duration_ms: stats.total_duration_ms.unwrap_or(0),
            total_tokens: stats.total_tokens.unwrap_or(0) as u64,
        })
    }
}

/// Execution statistics for a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub succeeded_count: u64,
    pub failed_count: u64,
    pub total_duration_ms: i64,
    pub total_tokens: u64,
}
```

---

### Step 6: Database Module Organization

Create module structure and exports.

**Files to Create**:

#### `ecl-core/src/db/mod.rs`

```rust
//! Database persistence layer for ECL workflows
//!
//! This module provides SQLx-based repositories for persisting workflow state,
//! step execution history, and artifacts. Supports both SQLite (development)
//! and PostgreSQL (production).
//!
//! # Architecture
//!
//! - **WorkflowRepository**: CRUD operations for workflows
//! - **StepExecutionRepository**: Execution history tracking
//! - **ArtifactRepository**: File artifact metadata (Phase 3)
//!
//! # Example
//!
//! ```no_run
//! use ecl_core::db::{WorkflowRepository, connect_sqlite};
//! use ecl_core::types::WorkflowId;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pool = connect_sqlite("./ecl.db").await?;
//!     let repo = WorkflowRepository::new(pool);
//!
//!     // Query workflow
//!     let workflow_id = WorkflowId::new();
//!     if let Some(workflow) = repo.get(&workflow_id).await? {
//!         println!("Workflow state: {:?}", workflow.state);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod workflow_repo;
pub mod step_execution_repo;
pub mod artifact_repo;

pub use error::{DbError, DbResult};
pub use workflow_repo::{WorkflowRepository, Workflow, WorkflowState, WorkflowRecord};
pub use step_execution_repo::{
    StepExecutionRepository, StepExecution, StepExecutionState,
    StepExecutionRecord, ExecutionStats
};

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::time::Duration;

/// Create SQLite connection pool
///
/// # Arguments
/// * `database_url` - SQLite database path (e.g., "sqlite://./ecl.db")
///
/// # Example
/// ```no_run
/// # use ecl_core::db::connect_sqlite;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = connect_sqlite("sqlite://./ecl.db").await?;
/// # Ok(())
/// # }
/// ```
pub async fn connect_sqlite(database_url: &str) -> DbResult<Pool<Sqlite>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await?;

    // Enable foreign key constraints (required for CASCADE deletes)
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(pool)
}

/// Run database migrations
///
/// # Example
/// ```no_run
/// # use ecl_core::db::{connect_sqlite, run_migrations};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = connect_sqlite("sqlite://./ecl.db").await?;
/// run_migrations(&pool).await?;
/// # Ok(())
/// # }
/// ```
pub async fn run_migrations(pool: &Pool<Sqlite>) -> DbResult<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| DbError::MigrationFailed(e.to_string()))?;
    Ok(())
}

/// Create in-memory SQLite database for testing
///
/// # Example
/// ```
/// # use ecl_core::db::connect_in_memory;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = connect_in_memory().await?;
/// // Use for fast integration tests
/// # Ok(())
/// # }
/// ```
pub async fn connect_in_memory() -> DbResult<Pool<Sqlite>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    // Run migrations on in-memory database
    run_migrations(&pool).await?;

    Ok(pool)
}
```

---

### Step 7: Artifact Repository Placeholder

Create placeholder for Phase 3 artifact persistence.

**Files to Create**:

#### `ecl-core/src/db/artifact_repo.rs`

```rust
//! Artifact repository (Phase 3 implementation)
//!
//! This module will provide persistence for workflow artifacts (files, reports, etc.)
//! produced by workflow steps. Full implementation in Phase 3.

use crate::db::error::{DbError, DbResult};
use crate::types::{WorkflowId, StepId};
use sqlx::{Pool, Sqlite};

/// Artifact repository (placeholder)
pub struct ArtifactRepository {
    pool: Pool<Sqlite>,
}

impl ArtifactRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    // TODO: Phase 3 - Implement full artifact CRUD operations
    // - create_artifact
    // - get_artifact
    // - list_artifacts_for_workflow
    // - list_artifacts_for_step
    // - delete_artifact
}
```

---

## Testing Strategy

### Unit Tests

Test repositories in isolation with in-memory SQLite.

**File**: `ecl-core/src/db/workflow_repo.rs` (add tests module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_in_memory;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_and_get_workflow() {
        let pool = connect_in_memory().await.unwrap();
        let repo = WorkflowRepository::new(pool);

        let workflow = Workflow {
            id: WorkflowId::new(),
            definition_id: "test_workflow".to_string(),
            state: WorkflowState::Pending,
            input: serde_json::json!({"topic": "test"}),
            output: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            metadata: None,
        };

        repo.create(&workflow).await.unwrap();

        let retrieved = repo.get(&workflow.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, workflow.id);
        assert_eq!(retrieved.state, WorkflowState::Pending);
    }

    #[tokio::test]
    async fn test_update_workflow_state() {
        let pool = connect_in_memory().await.unwrap();
        let repo = WorkflowRepository::new(pool);

        let workflow = create_test_workflow();
        repo.create(&workflow).await.unwrap();

        repo.update_state(&workflow.id, WorkflowState::Running, None)
            .await
            .unwrap();

        let updated = repo.get(&workflow.id).await.unwrap().unwrap();
        assert_eq!(updated.state, WorkflowState::Running);
    }

    #[tokio::test]
    async fn test_list_by_state() {
        let pool = connect_in_memory().await.unwrap();
        let repo = WorkflowRepository::new(pool);

        let workflow1 = create_test_workflow();
        let mut workflow2 = create_test_workflow();
        workflow2.state = WorkflowState::Running;

        repo.create(&workflow1).await.unwrap();
        repo.create(&workflow2).await.unwrap();

        let pending = repo.list_by_state(WorkflowState::Pending).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, workflow1.id);
    }

    fn create_test_workflow() -> Workflow {
        Workflow {
            id: WorkflowId::new(),
            definition_id: "test".to_string(),
            state: WorkflowState::Pending,
            input: serde_json::json!({}),
            output: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            metadata: None,
        }
    }
}
```

### Integration Tests

Test full repository workflows with real database operations.

**File**: `ecl-core/tests/db_integration_tests.rs`

```rust
use ecl_core::db::{
    connect_in_memory, WorkflowRepository, StepExecutionRepository,
    Workflow, WorkflowState, StepExecution, StepExecutionState,
};
use ecl_core::types::{WorkflowId, StepId};
use chrono::Utc;

#[tokio::test]
async fn test_workflow_lifecycle() {
    let pool = connect_in_memory().await.unwrap();
    let workflow_repo = WorkflowRepository::new(pool.clone());
    let step_repo = StepExecutionRepository::new(pool);

    // Create workflow
    let workflow = create_test_workflow();
    workflow_repo.create(&workflow).await.unwrap();

    // Create step execution
    let step_execution = create_test_step_execution(&workflow.id);
    step_repo.create(&step_execution).await.unwrap();

    // Query executions
    let executions = step_repo.get_for_workflow(&workflow.id).await.unwrap();
    assert_eq!(executions.len(), 1);

    // Update workflow state
    workflow_repo
        .update_state(&workflow.id, WorkflowState::Completed, None)
        .await
        .unwrap();

    // Verify state changed
    let updated = workflow_repo.get(&workflow.id).await.unwrap().unwrap();
    assert_eq!(updated.state, WorkflowState::Completed);
    assert!(updated.completed_at.is_some());
}

#[tokio::test]
async fn test_cascade_delete() {
    let pool = connect_in_memory().await.unwrap();
    let workflow_repo = WorkflowRepository::new(pool.clone());
    let step_repo = StepExecutionRepository::new(pool);

    let workflow = create_test_workflow();
    workflow_repo.create(&workflow).await.unwrap();

    let step_execution = create_test_step_execution(&workflow.id);
    step_repo.create(&step_execution).await.unwrap();

    // Delete workflow should cascade to step executions
    workflow_repo.delete(&workflow.id).await.unwrap();

    let executions = step_repo.get_for_workflow(&workflow.id).await.unwrap();
    assert_eq!(executions.len(), 0);
}

fn create_test_workflow() -> Workflow {
    Workflow {
        id: WorkflowId::new(),
        definition_id: "test_workflow".to_string(),
        state: WorkflowState::Pending,
        input: serde_json::json!({"topic": "test"}),
        output: None,
        error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
        metadata: None,
    }
}

fn create_test_step_execution(workflow_id: &WorkflowId) -> StepExecution {
    StepExecution {
        id: uuid::Uuid::new_v4(),
        workflow_id: workflow_id.clone(),
        step_id: StepId::from("test_step"),
        attempt: 1,
        state: StepExecutionState::Succeeded,
        input: serde_json::json!({"input": "test"}),
        output: Some(serde_json::json!({"output": "result"})),
        error: None,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        duration_ms: Some(100),
        tokens_used: Some(50),
        metadata: None,
    }
}
```

### Property Tests

Test repository invariants with proptest.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_workflow_state_roundtrip(state in prop_workflow_state()) {
        let db_string = state.to_db_string();
        let parsed = WorkflowState::from_db_string(db_string).unwrap();
        assert_eq!(state, parsed);
    }
}

fn prop_workflow_state() -> impl Strategy<Value = WorkflowState> {
    prop_oneof![
        Just(WorkflowState::Pending),
        Just(WorkflowState::Running),
        Just(WorkflowState::Completed),
        Just(WorkflowState::Failed),
    ]
}
```

---

## Compile-Time Query Validation

### Setup SQLx Offline Mode

Enable compile-time query checking without database connection.

**Steps**:

1. **Set DATABASE_URL in `.env`**:
   ```env
   DATABASE_URL=sqlite://./ecl.db
   ```

2. **Create database and run migrations**:
   ```bash
   sqlx database create
   sqlx migrate run
   ```

3. **Prepare offline query metadata**:
   ```bash
   cargo sqlx prepare
   ```

   This generates `.sqlx/query-*.json` files containing type information.

4. **Add to CI/CD**:
   ```yaml
   # .github/workflows/ci.yml
   - name: Check SQLx queries
     run: cargo sqlx prepare --check
   ```

5. **Build without database** (for CI):
   ```bash
   SQLX_OFFLINE=true cargo build
   ```

---

## Configuration

### Database Configuration in `ecl-core`

**File**: `ecl-core/src/config.rs` (add database section)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    /// Examples:
    /// - SQLite: "sqlite://./ecl.db"
    /// - Postgres: "postgres://user:password@localhost/ecl"
    pub url: String,

    /// Maximum number of connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_max_connections() -> u32 {
    5
}

fn default_timeout_secs() -> u64 {
    3
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://./ecl.db".to_string(),
            max_connections: default_max_connections(),
            timeout_secs: default_timeout_secs(),
        }
    }
}
```

---

## Potential Blockers

### 1. SQLx Version Compatibility

**Risk**: SQLx 0.8 may have breaking changes from earlier versions.

**Mitigation**:
- Verify SQLx 0.8 is stable and available on crates.io
- Test migrations on both SQLite and Postgres early
- Keep offline query metadata in version control

### 2. SQLite/Postgres Schema Differences

**Risk**: Schema uses TEXT for UUIDs and timestamps, may not leverage Postgres native types.

**Mitigation**:
- Current schema uses compatible subset of SQL
- Can add Postgres-specific migrations later for optimization
- Start with TEXT-based schema for maximum compatibility

### 3. JSON Serialization Performance

**Risk**: Storing input/output as JSON TEXT may impact performance on large workflows.

**Mitigation**:
- Profile queries with realistic data sizes
- Consider using BLOB for large payloads in future
- Index queries don't need to deserialize JSON

### 4. Database Migration Conflicts

**Risk**: Multiple developers creating migrations simultaneously.

**Mitigation**:
- Use timestamp-based migration numbering: `YYYYMMDD_HHMMSS_description.sql`
- Review migrations in PRs carefully
- Keep migrations idempotent (use IF NOT EXISTS)

### 5. Test Database Cleanup

**Risk**: Tests may leave stale data or interfere with each other.

**Mitigation**:
- Use in-memory SQLite for all unit/integration tests
- Each test creates fresh `connect_in_memory()` pool
- No shared state between tests

---

## Acceptance Criteria

- [ ] **Schema Created**: Migrations run successfully on SQLite and Postgres
- [ ] **Compile-Time Validation**: SQLx queries validated with `cargo sqlx prepare`
- [ ] **WorkflowRepository Complete**: All CRUD operations implemented and tested
- [ ] **StepExecutionRepository Complete**: History tracking works correctly
- [ ] **Error Handling**: All database errors use `DbError` enum
- [ ] **Foreign Keys Enabled**: CASCADE deletes work in tests
- [ ] **In-Memory Testing**: Tests use `connect_in_memory()` for fast execution
- [ ] **Unit Tests**: Repository methods covered ≥95%
- [ ] **Integration Tests**: Full workflow lifecycle tested
- [ ] **Property Tests**: State serialization roundtrip verified
- [ ] **Documentation**: All public API items have rustdoc
- [ ] **No Panics**: No `unwrap()` or `expect()` in repository code
- [ ] **Configuration**: Database URL configurable via config/env

---

## Files to Create

```
ecl-core/src/db/
├── mod.rs                      # Module exports and connection functions
├── error.rs                    # DbError type and DbResult alias
├── workflow_repo.rs            # WorkflowRepository implementation
├── step_execution_repo.rs      # StepExecutionRepository implementation
└── artifact_repo.rs            # Placeholder for Phase 3

migrations/
└── 001_initial_schema.sql      # Database schema for all tables

.sqlx/
└── query-*.json                # Generated query metadata (via sqlx prepare)

ecl-core/tests/
└── db_integration_tests.rs     # Integration tests for repositories

.env.example                     # Example environment configuration
```

---

## Integration with Previous Stages

### Stage 2.2: Step Executor

The executor can now persist execution results:

```rust
impl StepExecutor {
    pub async fn execute_with_persistence(
        &self,
        step: &dyn Step,
        context: &StepContext,
        step_repo: &StepExecutionRepository,
    ) -> Result<StepExecution, ExecutionError> {
        let execution = self.execute(step, context).await?;

        // Persist execution record
        step_repo.create(&execution).await?;

        Ok(execution)
    }
}
```

### Stage 2.4: Workflow Orchestrator

The orchestrator can now save/restore workflow state:

```rust
impl WorkflowOrchestrator {
    pub async fn resume_workflow(
        &self,
        workflow_id: &WorkflowId,
        workflow_repo: &WorkflowRepository,
        step_repo: &StepExecutionRepository,
    ) -> Result<Workflow, OrchestratorError> {
        // Load workflow state
        let workflow = workflow_repo.get(workflow_id).await?
            .ok_or(OrchestratorError::WorkflowNotFound)?;

        // Load execution history
        let executions = step_repo.get_for_workflow(workflow_id).await?;

        // Resume from last checkpoint
        self.resume(workflow, executions).await
    }
}
```

---

## Next Steps

After completing this stage:

1. Verify migrations run on both SQLite and Postgres
2. Run `cargo sqlx prepare` to generate offline query metadata
3. Ensure all tests pass with `just test`
4. Check test coverage with `just coverage` (target: ≥95%)
5. Move to **Stage 2.6: Observability Infrastructure**

---

## SQLx Best Practices Applied

1. **Compile-Time Verification**: Using `sqlx::query!` macro for type-checked queries
2. **Connection Pooling**: `SqlitePoolOptions` with appropriate limits
3. **Migrations**: Versioned migrations in `migrations/` directory
4. **Offline Mode**: Support for CI/CD with `SQLX_OFFLINE=true`
5. **Error Handling**: All errors wrapped in custom `DbError` type
6. **Repository Pattern**: Clean separation of persistence logic
7. **Testing**: In-memory SQLite for fast, isolated tests
8. **Foreign Keys**: Enabled for referential integrity
9. **Transactions**: Prepared for future transaction support
10. **JSON Handling**: Explicit serialization with error handling

---

**Estimated Effort**: 5-6 hours

**Confidence**: High - Well-defined schema, proven SQLx patterns, comprehensive tests.

---

**Next:** [Stage 2.6: Observability Infrastructure](./0008-phase-2-06-observability.md)
