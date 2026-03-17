//! Batch execution: concurrent stage processing with bounded concurrency.
//!
//! This module contains the free functions that run as independent tokio tasks.
//! They take owned data (no shared mutable state) and return results.

use std::sync::Arc;

use ecl_pipeline_state::StageId;
use ecl_pipeline_topo::{PipelineItem, ResolvedStage, Stage, StageContext, StageError};
use tracing::Instrument;

use crate::error::PipelineError;

/// The result of executing a single stage across all its items.
#[derive(Debug)]
pub struct StageResult {
    /// Which stage produced these results.
    pub stage_id: StageId,
    /// Items that completed successfully, with their output items.
    pub successes: Vec<StageItemSuccess>,
    /// Items that were skipped due to `skip_on_error`.
    pub skipped: Vec<StageItemSkipped>,
    /// Items that failed (non-recoverable or exhausted retries).
    pub failures: Vec<StageItemFailure>,
}

/// A successful item processing result.
#[derive(Debug)]
pub struct StageItemSuccess {
    /// The input item ID.
    pub item_id: String,
    /// The output items produced by the stage.
    pub outputs: Vec<PipelineItem>,
    /// Processing duration in milliseconds.
    pub duration_ms: u64,
}

/// An item that was skipped due to `skip_on_error`.
#[derive(Debug)]
pub struct StageItemSkipped {
    /// The input item ID.
    pub item_id: String,
    /// The error that caused the skip.
    pub error: StageError,
}

/// An item that failed processing.
#[derive(Debug)]
pub struct StageItemFailure {
    /// The input item ID.
    pub item_id: String,
    /// The error.
    pub error: StageError,
    /// Number of attempts made (including the initial attempt).
    pub attempts: u32,
}

impl StageResult {
    /// Create a new empty StageResult for the given stage.
    pub fn new(stage_id: StageId) -> Self {
        Self {
            stage_id,
            successes: Vec::new(),
            skipped: Vec::new(),
            failures: Vec::new(),
        }
    }

    /// Record a successful item processing.
    pub fn record_success(
        &mut self,
        item_id: String,
        outputs: Vec<PipelineItem>,
        duration_ms: u64,
    ) {
        self.successes.push(StageItemSuccess {
            item_id,
            outputs,
            duration_ms,
        });
    }

    /// Record a skipped item (due to `skip_on_error`).
    pub fn record_skipped(&mut self, item_id: String, error: StageError) {
        self.skipped.push(StageItemSkipped { item_id, error });
    }

    /// Record a failed item.
    pub fn record_failure(&mut self, item_id: String, error: StageError, attempts: u32) {
        self.failures.push(StageItemFailure {
            item_id,
            error,
            attempts,
        });
    }

    /// Returns true if any items failed (not skipped — actually failed).
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }
}

/// Execute a single stage's items with bounded concurrency.
///
/// Runs as an independent task — no shared mutable state. The caller
/// provides owned data; the function returns a `StageResult` with
/// per-item outcomes.
///
/// Uses a `tokio::sync::Semaphore` to limit concurrent item processing
/// to `concurrency` parallel operations. Each item is spawned as a
/// separate tokio task within a `JoinSet`.
pub async fn execute_stage_items(
    stage: ResolvedStage,
    items: Vec<PipelineItem>,
    ctx: StageContext,
    concurrency: usize,
) -> std::result::Result<StageResult, PipelineError> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut join_set = tokio::task::JoinSet::new();

    let stage_name = stage.id.as_str().to_string();
    let item_count = items.len();
    tracing::info!(stage = %stage_name, items = item_count, "starting stage");

    for item in items {
        let permit = semaphore.clone().acquire_owned().await?;
        let handler = stage.handler.clone();
        let ctx = ctx.clone();
        let retry = stage.retry.clone();
        let skip_on_error = stage.skip_on_error;
        let s_name = stage_name.clone();
        let item_id = item.id.clone();

        let item_span = tracing::info_span!("item", stage = %s_name, item_id = %item_id);
        join_set.spawn(
            async move {
                let _permit = permit; // held until task completes
                let start = std::time::Instant::now();
                let retry_result = execute_with_retry(&handler, item.clone(), &ctx, &retry).await;
                let duration_ms = start.elapsed().as_millis() as u64;
                (
                    item.id.clone(),
                    retry_result.result,
                    retry_result.attempts,
                    skip_on_error,
                    duration_ms,
                )
            }
            .instrument(item_span),
        );
    }

    let mut stage_result = StageResult::new(stage.id.clone());
    while let Some(result) = join_set.join_next().await {
        let (item_id, result, attempts, skip_on_error, duration_ms) = result?;
        match result {
            Ok(outputs) => {
                tracing::debug!(item_id = %item_id, duration_ms, attempts, status = "ok", "item completed");
                stage_result.record_success(item_id, outputs, duration_ms);
            }
            Err(e) if skip_on_error => {
                tracing::warn!(item_id = %item_id, duration_ms, attempts, error = %e, "item skipped");
                stage_result.record_skipped(item_id, e);
            }
            Err(e) => {
                tracing::error!(item_id = %item_id, duration_ms, attempts, error = %e, "item failed");
                stage_result.record_failure(item_id, e, attempts);
            }
        }
    }

    tracing::info!(
        stage = %stage_name,
        succeeded = stage_result.successes.len(),
        skipped = stage_result.skipped.len(),
        failed = stage_result.failures.len(),
        "stage completed"
    );

    Ok(stage_result)
}

/// Execute a stage handler with retry and exponential backoff.
///
/// Uses the `backon` crate for retry logic. The backoff parameters
/// come from the `RetryPolicy` which was resolved from the stage's
/// retry configuration merged with global defaults.
///
/// Only retries on error — successful results are returned immediately.
/// Result of a retried execution, including the attempt count.
pub struct RetryResult {
    /// The outcome (success or final error).
    pub result: std::result::Result<Vec<PipelineItem>, StageError>,
    /// Number of attempts made (1 = succeeded on first try).
    pub attempts: u32,
}

/// Execute a stage handler with retry and exponential backoff.
///
/// Uses the `backon` crate for retry logic. The backoff parameters
/// come from the `RetryPolicy` which was resolved from the stage's
/// retry configuration merged with global defaults.
///
/// Only retries on error — successful results are returned immediately.
/// Returns both the result and the number of attempts made.
pub async fn execute_with_retry(
    handler: &Arc<dyn Stage>,
    item: PipelineItem,
    ctx: &StageContext,
    retry: &ecl_pipeline_topo::RetryPolicy,
) -> RetryResult {
    use backon::{ExponentialBuilder, Retryable};
    use std::sync::atomic::{AtomicU32, Ordering};

    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();

    let backoff = ExponentialBuilder::default()
        .with_min_delay(retry.initial_backoff)
        .with_factor(retry.backoff_multiplier as f32)
        .with_max_delay(retry.max_backoff)
        .with_max_times(retry.max_attempts.saturating_sub(1) as usize);

    let result = (|| async {
        attempts_clone.fetch_add(1, Ordering::SeqCst);
        handler.process(item.clone(), ctx).await
    })
    .retry(backoff)
    .await;

    RetryResult {
        result,
        attempts: attempts.load(Ordering::SeqCst),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use ecl_pipeline_spec::{DefaultsSpec, PipelineSpec};
    use ecl_pipeline_state::Blake3Hash;
    use ecl_pipeline_state::ItemProvenance;
    use ecl_pipeline_topo::RetryPolicy;

    // ── Mock types ──────────────────────────────────────────────────────

    #[derive(Debug)]
    struct MockStage {
        name: String,
        suffix: String,
    }

    impl MockStage {
        fn new(name: &str, suffix: &str) -> Self {
            Self {
                name: name.to_string(),
                suffix: suffix.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl Stage for MockStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> std::result::Result<Vec<PipelineItem>, StageError> {
            let mut new_content = item.content.to_vec();
            new_content.extend_from_slice(self.suffix.as_bytes());
            Ok(vec![PipelineItem {
                content: Arc::from(new_content.as_slice()),
                ..item
            }])
        }
    }

    #[derive(Debug)]
    struct FailingStage {
        name: String,
        fail_count: AtomicU32,
        fail_until: u32,
    }

    impl FailingStage {
        fn new(name: &str, fail_until: u32) -> Self {
            Self {
                name: name.to_string(),
                fail_count: AtomicU32::new(0),
                fail_until,
            }
        }
    }

    #[async_trait::async_trait]
    impl Stage for FailingStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> std::result::Result<Vec<PipelineItem>, StageError> {
            let count = self.fail_count.fetch_add(1, Ordering::SeqCst);
            if count < self.fail_until {
                Err(StageError::Transient {
                    stage: self.name.clone(),
                    item_id: item.id.clone(),
                    message: format!("attempt {} failed", count + 1),
                })
            } else {
                Ok(vec![item])
            }
        }
    }

    #[derive(Debug)]
    struct AlwaysFailingStage {
        name: String,
    }

    impl AlwaysFailingStage {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl Stage for AlwaysFailingStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> std::result::Result<Vec<PipelineItem>, StageError> {
            Err(StageError::Permanent {
                stage: self.name.clone(),
                item_id: item.id.clone(),
                message: "always fails".to_string(),
            })
        }
    }

    // ── Test helpers ────────────────────────────────────────────────────

    fn make_pipeline_item(id: &str) -> PipelineItem {
        PipelineItem {
            id: id.to_string(),
            display_name: format!("Item {id}"),
            content: Arc::from(format!("content-{id}").as_bytes()),
            mime_type: "text/plain".to_string(),
            source_name: "test-source".to_string(),
            source_content_hash: Blake3Hash::new(""),
            provenance: ItemProvenance {
                source_kind: "test".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
        }
    }

    fn make_stage_context() -> StageContext {
        StageContext {
            spec: Arc::new(PipelineSpec {
                name: "test".to_string(),
                version: 1,
                output_dir: PathBuf::from("/tmp/test"),
                sources: BTreeMap::new(),
                stages: BTreeMap::new(),
                defaults: DefaultsSpec::default(),
            }),
            output_dir: PathBuf::from("/tmp/test"),
            params: serde_json::Value::Null,
            span: tracing::info_span!("test"),
        }
    }

    fn fast_retry_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1),
            backoff_multiplier: 1.0,
            max_backoff: Duration::from_millis(10),
        }
    }

    fn make_resolved_stage(
        name: &str,
        handler: Arc<dyn Stage>,
        skip_on_error: bool,
    ) -> ResolvedStage {
        ResolvedStage {
            id: ecl_pipeline_state::StageId::new(name),
            handler,
            retry: fast_retry_policy(),
            skip_on_error,
            timeout: None,
            source: None,
            condition: None,
        }
    }

    // ── StageResult tests ───────────────────────────────────────────────

    #[test]
    fn test_stage_result_new_is_empty() {
        let result = StageResult::new(StageId::new("test"));
        assert!(result.successes.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_stage_result_record_success() {
        let mut result = StageResult::new(StageId::new("test"));
        result.record_success("item-1".to_string(), vec![], 42);
        assert_eq!(result.successes.len(), 1);
        assert_eq!(result.successes[0].item_id, "item-1");
        assert_eq!(result.successes[0].duration_ms, 42);
    }

    #[test]
    fn test_stage_result_record_skipped() {
        let mut result = StageResult::new(StageId::new("test"));
        let err = StageError::Transient {
            stage: "s".to_string(),
            item_id: "i".to_string(),
            message: "skip".to_string(),
        };
        result.record_skipped("item-1".to_string(), err);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].item_id, "item-1");
    }

    #[test]
    fn test_stage_result_record_failure() {
        let mut result = StageResult::new(StageId::new("test"));
        let err = StageError::Permanent {
            stage: "s".to_string(),
            item_id: "i".to_string(),
            message: "fail".to_string(),
        };
        result.record_failure("item-1".to_string(), err, 3);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].item_id, "item-1");
        assert_eq!(result.failures[0].attempts, 3);
    }

    #[test]
    fn test_stage_result_has_failures_false_when_empty() {
        let result = StageResult::new(StageId::new("test"));
        assert!(!result.has_failures());
    }

    #[test]
    fn test_stage_result_has_failures_true_when_failure_exists() {
        let mut result = StageResult::new(StageId::new("test"));
        let err = StageError::Permanent {
            stage: "s".to_string(),
            item_id: "i".to_string(),
            message: "fail".to_string(),
        };
        result.record_failure("item-1".to_string(), err, 1);
        assert!(result.has_failures());
    }

    #[test]
    fn test_stage_result_has_failures_false_when_only_skipped() {
        let mut result = StageResult::new(StageId::new("test"));
        let err = StageError::Transient {
            stage: "s".to_string(),
            item_id: "i".to_string(),
            message: "skip".to_string(),
        };
        result.record_skipped("item-1".to_string(), err);
        assert!(!result.has_failures());
    }

    // ── execute_with_retry tests ────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_with_retry_succeeds_first_attempt() {
        let handler: Arc<dyn Stage> = Arc::new(MockStage::new("mock", "-done"));
        let item = make_pipeline_item("a");
        let ctx = make_stage_context();
        let retry = fast_retry_policy();

        let retry_result = execute_with_retry(&handler, item, &ctx, &retry).await;
        assert!(retry_result.result.is_ok());
        assert_eq!(retry_result.attempts, 1);
        assert_eq!(retry_result.result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_execute_with_retry_succeeds_after_failures() {
        let handler: Arc<dyn Stage> = Arc::new(FailingStage::new("retry", 2));
        let item = make_pipeline_item("a");
        let ctx = make_stage_context();
        let retry = fast_retry_policy(); // max_attempts=3

        let retry_result = execute_with_retry(&handler, item, &ctx, &retry).await;
        assert!(retry_result.result.is_ok(), "should succeed on 3rd attempt");
        assert_eq!(retry_result.attempts, 3);
    }

    #[tokio::test]
    async fn test_execute_with_retry_exhausts_retries() {
        let handler: Arc<dyn Stage> = Arc::new(FailingStage::new("fail", 5));
        let item = make_pipeline_item("a");
        let ctx = make_stage_context();
        let retry = fast_retry_policy(); // max_attempts=3, so fails

        let retry_result = execute_with_retry(&handler, item, &ctx, &retry).await;
        assert!(retry_result.result.is_err(), "should fail after 3 attempts");
        assert_eq!(retry_result.attempts, 3);
    }

    // ── execute_stage_items tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_execute_stage_items_all_succeed() {
        let handler: Arc<dyn Stage> = Arc::new(MockStage::new("mock", "-ok"));
        let stage = make_resolved_stage("mock", handler, false);
        let items = vec![
            make_pipeline_item("a"),
            make_pipeline_item("b"),
            make_pipeline_item("c"),
        ];
        let ctx = make_stage_context();

        let result = execute_stage_items(stage, items, ctx, 4).await.unwrap();
        assert_eq!(result.successes.len(), 3);
        assert!(result.skipped.is_empty());
        assert!(result.failures.is_empty());
    }

    #[tokio::test]
    async fn test_execute_stage_items_with_skip_on_error() {
        let handler: Arc<dyn Stage> = Arc::new(AlwaysFailingStage::new("fail"));
        let stage = make_resolved_stage("fail", handler, true); // skip_on_error=true
        let items = vec![make_pipeline_item("a"), make_pipeline_item("b")];
        let ctx = make_stage_context();

        let result = execute_stage_items(stage, items, ctx, 4).await.unwrap();
        assert!(result.successes.is_empty());
        assert_eq!(result.skipped.len(), 2);
        assert!(result.failures.is_empty());
    }

    #[tokio::test]
    async fn test_execute_stage_items_without_skip_on_error() {
        let handler: Arc<dyn Stage> = Arc::new(AlwaysFailingStage::new("fail"));
        let stage = make_resolved_stage("fail", handler, false); // skip_on_error=false
        let items = vec![make_pipeline_item("a")];
        let ctx = make_stage_context();

        let result = execute_stage_items(stage, items, ctx, 4).await.unwrap();
        assert!(result.successes.is_empty());
        assert!(result.skipped.is_empty());
        assert_eq!(result.failures.len(), 1);
    }

    #[tokio::test]
    async fn test_execute_stage_items_empty_items() {
        let handler: Arc<dyn Stage> = Arc::new(MockStage::new("mock", ""));
        let stage = make_resolved_stage("mock", handler, false);
        let ctx = make_stage_context();

        let result = execute_stage_items(stage, vec![], ctx, 4).await.unwrap();
        assert!(result.successes.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.failures.is_empty());
    }

    #[tokio::test]
    async fn test_execute_stage_items_respects_concurrency() {
        use std::sync::atomic::AtomicUsize;
        use tokio::time::sleep;

        /// A stage that tracks max concurrent executions.
        #[derive(Debug)]
        struct ConcurrencyTracker {
            name: String,
            active: AtomicUsize,
            max_active: AtomicUsize,
        }

        #[async_trait::async_trait]
        impl Stage for ConcurrencyTracker {
            fn name(&self) -> &str {
                &self.name
            }

            async fn process(
                &self,
                item: PipelineItem,
                _ctx: &StageContext,
            ) -> std::result::Result<Vec<PipelineItem>, StageError> {
                let current = self.active.fetch_add(1, Ordering::SeqCst) + 1;
                // Update max if current is higher.
                self.max_active.fetch_max(current, Ordering::SeqCst);
                sleep(Duration::from_millis(50)).await;
                self.active.fetch_sub(1, Ordering::SeqCst);
                Ok(vec![item])
            }
        }

        let tracker = Arc::new(ConcurrencyTracker {
            name: "tracker".to_string(),
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
        });
        let handler: Arc<dyn Stage> = tracker.clone();
        let stage = make_resolved_stage("tracker", handler, false);
        let items: Vec<_> = (0..6)
            .map(|i| make_pipeline_item(&format!("{i}")))
            .collect();
        let ctx = make_stage_context();

        let result = execute_stage_items(stage, items, ctx, 2).await.unwrap();
        assert_eq!(result.successes.len(), 6);
        let max = tracker.max_active.load(Ordering::SeqCst);
        assert!(max <= 2, "max concurrent should be <= 2, was {max}");
    }
}
