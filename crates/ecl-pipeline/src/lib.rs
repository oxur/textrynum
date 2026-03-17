//! Pipeline execution engine for the ECL pipeline runner.
//!
//! This crate contains the `PipelineRunner` — the orchestrator that
//! executes a resolved pipeline topology. It handles:
//!
//! - Source enumeration
//! - Incremental processing (content hash comparison)
//! - Batch execution with concurrent stages
//! - Per-item bounded concurrency within stages
//! - Retry with exponential backoff
//! - Checkpointing at batch boundaries
//! - Resume from checkpoint after interruption
//!
//! # Usage
//!
//! ```ignore
//! let topology = /* resolve from PipelineSpec */;
//! let store = Box::new(InMemoryStateStore::new());
//! let mut runner = PipelineRunner::new(topology, store).await?;
//! let state = runner.run().await?;
//! ```

pub mod batch;
pub mod error;
pub mod registry;
pub mod runner;

pub use batch::{
    RetryResult, StageItemFailure, StageItemSkipped, StageItemSuccess, StageResult,
    execute_stage_items, execute_with_retry,
};
pub use error::{PipelineError, Result};
pub use registry::{AdapterRegistry, StageRegistry};
pub use runner::PipelineRunner;
