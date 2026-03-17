//! Service lifecycle state management.
//!
//! Provides [`ServiceState`] and [`ServiceHandle`] for tracking the lifecycle
//! of background services (index builders, search backends, etc.).
//!
//! # Usage
//!
//! ```rust
//! use fabryk_core::service::{ServiceHandle, ServiceState};
//!
//! let handle = ServiceHandle::new("my-service");
//! assert_eq!(handle.state(), ServiceState::Stopped);
//!
//! handle.set_state(ServiceState::Starting);
//! handle.set_state(ServiceState::Ready);
//! assert!(handle.state().is_ready());
//! ```

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

// ============================================================================
// Free functions
// ============================================================================

/// Wait for **all** services to reach `Ready` (or fail), **in parallel**.
///
/// Each service gets its own concurrent `wait_ready` future sharing the
/// same `timeout`. Returns `Ok(())` if every service becomes `Ready`
/// within the deadline, or `Err(errors)` collecting every failure/timeout
/// message.
///
/// This is more efficient than calling `wait_ready` sequentially when
/// multiple services initialise concurrently — the wall-clock wait time
/// equals the **slowest** service rather than the **sum** of all services.
pub async fn wait_all_ready(
    services: &[ServiceHandle],
    timeout: Duration,
) -> Result<(), Vec<String>> {
    let futures: Vec<_> = services.iter().map(|svc| svc.wait_ready(timeout)).collect();

    let results = futures::future::join_all(futures).await;

    let errors: Vec<String> = results.into_iter().filter_map(|r| r.err()).collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Configuration for [`spawn_with_retry`].
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of attempts (including the initial one).
    pub max_attempts: u32,
    /// Delay between retries. Doubles each attempt (exponential backoff).
    pub initial_delay: Duration,
    /// Maximum delay between retries (caps the exponential growth).
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Spawn a background task that runs `init_fn` with retry on failure.
///
/// On each attempt the handle transitions to `Starting`. If `init_fn`
/// succeeds the handle moves to `Ready`. If all attempts are exhausted
/// the handle moves to `Failed` with the last error message.
///
/// Returns a `JoinHandle` for the background task.
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_core::service::{ServiceHandle, spawn_with_retry, RetryConfig};
///
/// let svc = ServiceHandle::new("redis");
/// let cell = Arc::new(tokio::sync::OnceCell::new());
/// let cell_bg = cell.clone();
///
/// spawn_with_retry(svc.clone(), RetryConfig::default(), move || {
///     let cell = cell_bg.clone();
///     async move {
///         let client = RedisClient::new(&url).await?;
///         cell.set(Arc::new(client)).ok();
///         Ok(())
///     }
/// });
/// ```
pub fn spawn_with_retry<F, Fut>(
    handle: ServiceHandle,
    config: RetryConfig,
    init_fn: F,
) -> tokio::task::JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    tokio::spawn(async move {
        let mut delay = config.initial_delay;

        for attempt in 1..=config.max_attempts {
            handle.set_state(ServiceState::Starting);

            match init_fn().await {
                Ok(()) => {
                    handle.set_state(ServiceState::Ready);
                    return;
                }
                Err(e) => {
                    if attempt == config.max_attempts {
                        log::error!(
                            "Service '{}' failed after {} attempts: {e}",
                            handle.name(),
                            config.max_attempts
                        );
                        handle.set_state(ServiceState::Failed(e));
                        return;
                    }
                    log::warn!(
                        "Service '{}' attempt {}/{} failed: {e} — retrying in {delay:?}",
                        handle.name(),
                        attempt,
                        config.max_attempts
                    );
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(config.max_delay);
                }
            }
        }
    })
}

// ============================================================================
// ServiceState
// ============================================================================

/// State of a service in its lifecycle.
#[derive(Clone, Debug, PartialEq)]
pub enum ServiceState {
    /// Service has not been started.
    Stopped,
    /// Service is initializing (e.g., building index).
    Starting,
    /// Service is operational and accepting requests.
    Ready,
    /// Service is partially operational.
    Degraded(String),
    /// Service is shutting down.
    Stopping,
    /// Service failed to start or encountered a fatal error.
    Failed(String),
}

impl ServiceState {
    /// Returns `true` if the service is fully ready.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Returns `true` if the service can handle requests (Ready or Degraded).
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Ready | Self::Degraded(_))
    }

    /// Returns `true` if the service is in a terminal state (Stopped or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed(_))
    }
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stopped => write!(f, "stopped"),
            Self::Starting => write!(f, "starting"),
            Self::Ready => write!(f, "ready"),
            Self::Degraded(reason) => write!(f, "degraded: {reason}"),
            Self::Stopping => write!(f, "stopping"),
            Self::Failed(reason) => write!(f, "failed: {reason}"),
        }
    }
}

// ============================================================================
// ServiceHandle
// ============================================================================

/// A recorded state transition with timestamp.
#[derive(Clone, Debug)]
pub struct Transition {
    /// The state that was entered.
    pub state: ServiceState,
    /// When the transition occurred (monotonic, relative to handle creation).
    pub elapsed: Duration,
}

/// Thread-safe handle for observing and updating service state.
///
/// Cheap to clone (Arc internals). State changes are broadcast
/// to all subscribers via a watch channel. Every transition is
/// recorded in an audit trail accessible via [`transitions()`](Self::transitions).
#[derive(Clone)]
pub struct ServiceHandle {
    inner: Arc<ServiceHandleInner>,
}

struct ServiceHandleInner {
    name: String,
    tx: watch::Sender<ServiceState>,
    started_at: Instant,
    transitions: std::sync::Mutex<Vec<Transition>>,
}

impl ServiceHandle {
    /// Create a new service handle with the given name.
    ///
    /// Initial state is [`ServiceState::Stopped`].
    pub fn new(name: impl Into<String>) -> Self {
        let (tx, _rx) = watch::channel(ServiceState::Stopped);
        Self {
            inner: Arc::new(ServiceHandleInner {
                name: name.into(),
                tx,
                started_at: Instant::now(),
                transitions: std::sync::Mutex::new(vec![Transition {
                    state: ServiceState::Stopped,
                    elapsed: Duration::ZERO,
                }]),
            }),
        }
    }

    /// Get the service name.
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get the current service state.
    pub fn state(&self) -> ServiceState {
        self.inner.tx.borrow().clone()
    }

    /// Update the service state.
    ///
    /// All subscribers are notified of the change. The transition is
    /// recorded in the audit trail with a monotonic timestamp.
    pub fn set_state(&self, state: ServiceState) {
        match &state {
            ServiceState::Failed(_) => {
                log::error!("Service '{}' → {state}", self.inner.name);
            }
            ServiceState::Degraded(_) => {
                log::warn!("Service '{}' → {state}", self.inner.name);
            }
            _ => {
                log::info!("Service '{}' → {state}", self.inner.name);
            }
        }
        if let Ok(mut log) = self.inner.transitions.lock() {
            log.push(Transition {
                state: state.clone(),
                elapsed: self.inner.started_at.elapsed(),
            });
        }
        self.inner.tx.send_replace(state);
    }

    /// Get the full transition audit trail.
    ///
    /// Returns a snapshot of all recorded state transitions, each with a
    /// monotonic timestamp relative to handle creation. Useful for
    /// diagnostics and debugging startup timing.
    pub fn transitions(&self) -> Vec<Transition> {
        self.inner
            .transitions
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }

    /// Subscribe to state changes.
    pub fn subscribe(&self) -> watch::Receiver<ServiceState> {
        self.inner.tx.subscribe()
    }

    /// Wait until the service reaches Ready, Failed, or timeout.
    pub async fn wait_ready(&self, timeout: Duration) -> Result<(), String> {
        let mut rx = self.subscribe();
        let deadline = tokio::time::sleep(timeout);
        tokio::pin!(deadline);

        // Check current state first
        {
            let state = rx.borrow_and_update().clone();
            match state {
                ServiceState::Ready => return Ok(()),
                ServiceState::Failed(reason) => {
                    return Err(format!("Service '{}' failed: {reason}", self.inner.name));
                }
                _ => {}
            }
        }

        loop {
            tokio::select! {
                _ = &mut deadline => {
                    return Err(format!(
                        "Service '{}' not ready after {timeout:?} (state: {})",
                        self.inner.name, self.state()
                    ));
                }
                result = rx.changed() => {
                    if result.is_err() {
                        return Err(format!("Service '{}' channel closed", self.inner.name));
                    }
                    let state = rx.borrow().clone();
                    match state {
                        ServiceState::Ready => return Ok(()),
                        ServiceState::Failed(reason) => {
                            return Err(format!(
                                "Service '{}' failed: {reason}",
                                self.inner.name
                            ));
                        }
                        _ => continue,
                    }
                }
            }
        }
    }

    /// Elapsed time since the handle was created.
    pub fn elapsed(&self) -> Duration {
        self.inner.started_at.elapsed()
    }
}

impl fmt::Debug for ServiceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceHandle")
            .field("name", &self.inner.name)
            .field("state", &self.state())
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_state_display() {
        assert_eq!(ServiceState::Stopped.to_string(), "stopped");
        assert_eq!(ServiceState::Starting.to_string(), "starting");
        assert_eq!(ServiceState::Ready.to_string(), "ready");
        assert_eq!(
            ServiceState::Degraded("low memory".to_string()).to_string(),
            "degraded: low memory"
        );
        assert_eq!(ServiceState::Stopping.to_string(), "stopping");
        assert_eq!(
            ServiceState::Failed("crash".to_string()).to_string(),
            "failed: crash"
        );
    }

    #[test]
    fn test_service_state_predicates() {
        assert!(ServiceState::Ready.is_ready());
        assert!(!ServiceState::Starting.is_ready());
        assert!(!ServiceState::Degraded("x".into()).is_ready());

        assert!(ServiceState::Ready.is_available());
        assert!(ServiceState::Degraded("x".into()).is_available());
        assert!(!ServiceState::Starting.is_available());
        assert!(!ServiceState::Stopped.is_available());

        assert!(ServiceState::Stopped.is_terminal());
        assert!(ServiceState::Failed("x".into()).is_terminal());
        assert!(!ServiceState::Ready.is_terminal());
        assert!(!ServiceState::Starting.is_terminal());
    }

    #[test]
    fn test_service_handle_initial_state() {
        let handle = ServiceHandle::new("test");
        assert_eq!(handle.name(), "test");
        assert_eq!(handle.state(), ServiceState::Stopped);
    }

    #[test]
    fn test_service_handle_state_transitions() {
        let handle = ServiceHandle::new("test");

        handle.set_state(ServiceState::Starting);
        assert_eq!(handle.state(), ServiceState::Starting);

        handle.set_state(ServiceState::Ready);
        assert_eq!(handle.state(), ServiceState::Ready);

        handle.set_state(ServiceState::Stopping);
        assert_eq!(handle.state(), ServiceState::Stopping);

        handle.set_state(ServiceState::Stopped);
        assert_eq!(handle.state(), ServiceState::Stopped);
    }

    #[test]
    fn test_service_handle_clone_shares_state() {
        let handle1 = ServiceHandle::new("shared");
        let handle2 = handle1.clone();

        handle1.set_state(ServiceState::Ready);
        assert_eq!(handle2.state(), ServiceState::Ready);

        handle2.set_state(ServiceState::Stopping);
        assert_eq!(handle1.state(), ServiceState::Stopping);
    }

    #[test]
    fn test_service_handle_subscribe() {
        let handle = ServiceHandle::new("test");
        let mut rx = handle.subscribe();

        // Initial value
        assert_eq!(*rx.borrow(), ServiceState::Stopped);

        handle.set_state(ServiceState::Starting);
        // Note: watch::Receiver sees latest value on next borrow
        assert_eq!(*rx.borrow_and_update(), ServiceState::Starting);
    }

    #[tokio::test]
    async fn test_service_handle_wait_ready_success() {
        let handle = ServiceHandle::new("test");
        let h = handle.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            h.set_state(ServiceState::Starting);
            tokio::time::sleep(Duration::from_millis(10)).await;
            h.set_state(ServiceState::Ready);
        });

        let result = handle.wait_ready(Duration::from_secs(1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_service_handle_wait_ready_timeout() {
        let handle = ServiceHandle::new("slow");
        handle.set_state(ServiceState::Starting);

        let result = handle.wait_ready(Duration::from_millis(50)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not ready after"));
    }

    #[tokio::test]
    async fn test_service_handle_wait_ready_failed() {
        let handle = ServiceHandle::new("broken");
        let h = handle.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            h.set_state(ServiceState::Failed("out of memory".to_string()));
        });

        let result = handle.wait_ready(Duration::from_secs(1)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("failed"));
        assert!(err.contains("out of memory"));
    }

    #[tokio::test]
    async fn test_service_handle_wait_ready_already_ready() {
        let handle = ServiceHandle::new("instant");
        handle.set_state(ServiceState::Ready);

        let result = handle.wait_ready(Duration::from_millis(50)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_service_handle_wait_ready_already_failed() {
        let handle = ServiceHandle::new("instant-fail");
        handle.set_state(ServiceState::Failed("boom".to_string()));

        let result = handle.wait_ready(Duration::from_millis(50)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("boom"));
    }

    #[test]
    fn test_service_handle_elapsed() {
        let handle = ServiceHandle::new("test");
        std::thread::sleep(Duration::from_millis(10));
        assert!(handle.elapsed() >= Duration::from_millis(10));
    }

    #[test]
    fn test_service_handle_debug() {
        let handle = ServiceHandle::new("debug-test");
        let debug = format!("{:?}", handle);
        assert!(debug.contains("debug-test"));
        assert!(debug.contains("ServiceHandle"));
    }

    // Compile-time check: ServiceHandle must be Send + Sync
    fn _assert_send_sync<T: Send + Sync>() {}
    #[test]
    fn test_service_handle_send_sync() {
        _assert_send_sync::<ServiceHandle>();
        _assert_send_sync::<ServiceState>();
    }

    // ── wait_all_ready tests ────────────────────────────────────────

    #[tokio::test]
    async fn test_wait_all_ready_all_already_ready() {
        let a = ServiceHandle::new("a");
        let b = ServiceHandle::new("b");
        a.set_state(ServiceState::Ready);
        b.set_state(ServiceState::Ready);

        let result = wait_all_ready(&[a, b], Duration::from_millis(50)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_all_ready_parallel_startup() {
        let a = ServiceHandle::new("a");
        let b = ServiceHandle::new("b");

        let ha = a.clone();
        let hb = b.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            ha.set_state(ServiceState::Ready);
        });
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            hb.set_state(ServiceState::Ready);
        });

        let result = wait_all_ready(&[a, b], Duration::from_secs(1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_all_ready_one_fails() {
        let a = ServiceHandle::new("ok");
        let b = ServiceHandle::new("broken");
        a.set_state(ServiceState::Ready);
        b.set_state(ServiceState::Failed("boom".into()));

        let result = wait_all_ready(&[a, b], Duration::from_millis(50)).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("boom"));
    }

    #[tokio::test]
    async fn test_wait_all_ready_one_timeout() {
        let a = ServiceHandle::new("ok");
        let b = ServiceHandle::new("slow");
        a.set_state(ServiceState::Ready);
        b.set_state(ServiceState::Starting);

        let result = wait_all_ready(&[a, b], Duration::from_millis(50)).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not ready after"));
    }

    #[tokio::test]
    async fn test_wait_all_ready_empty_services() {
        let result = wait_all_ready(&[], Duration::from_millis(50)).await;
        assert!(result.is_ok());
    }

    // ── spawn_with_retry tests ──────────────────────────────────────

    #[tokio::test]
    async fn test_spawn_with_retry_succeeds_first_attempt() {
        let handle = ServiceHandle::new("ok");
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
        };

        let jh = spawn_with_retry(handle.clone(), config, || async { Ok(()) });
        jh.await.unwrap();

        assert_eq!(handle.state(), ServiceState::Ready);
    }

    #[tokio::test]
    async fn test_spawn_with_retry_succeeds_after_failures() {
        let handle = ServiceHandle::new("flaky");
        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
        };

        let jh = spawn_with_retry(handle.clone(), config, move || {
            let c = counter_clone.clone();
            async move {
                let attempt = c.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if attempt < 3 {
                    Err(format!("attempt {attempt} failed"))
                } else {
                    Ok(())
                }
            }
        });
        jh.await.unwrap();

        assert_eq!(handle.state(), ServiceState::Ready);
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_spawn_with_retry_exhausts_attempts() {
        let handle = ServiceHandle::new("broken");
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
        };

        let jh = spawn_with_retry(handle.clone(), config, || async {
            Err("still broken".to_string())
        });
        jh.await.unwrap();

        assert!(
            matches!(handle.state(), ServiceState::Failed(msg) if msg.contains("still broken"))
        );
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(30));
    }

    // ── transition audit trail tests ────────────────────────────────

    #[test]
    fn test_transitions_initial_state_recorded() {
        let handle = ServiceHandle::new("test");
        let transitions = handle.transitions();
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].state, ServiceState::Stopped);
        assert_eq!(transitions[0].elapsed, Duration::ZERO);
    }

    #[test]
    fn test_transitions_records_all_changes() {
        let handle = ServiceHandle::new("test");
        handle.set_state(ServiceState::Starting);
        handle.set_state(ServiceState::Ready);

        let transitions = handle.transitions();
        assert_eq!(transitions.len(), 3);
        assert_eq!(transitions[0].state, ServiceState::Stopped);
        assert_eq!(transitions[1].state, ServiceState::Starting);
        assert_eq!(transitions[2].state, ServiceState::Ready);
    }

    #[test]
    fn test_transitions_timestamps_monotonic() {
        let handle = ServiceHandle::new("test");
        std::thread::sleep(Duration::from_millis(5));
        handle.set_state(ServiceState::Starting);
        std::thread::sleep(Duration::from_millis(5));
        handle.set_state(ServiceState::Ready);

        let transitions = handle.transitions();
        for window in transitions.windows(2) {
            assert!(
                window[1].elapsed >= window[0].elapsed,
                "timestamps should be monotonically increasing"
            );
        }
        // The last transition should have a non-zero elapsed
        assert!(transitions[2].elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_transitions_cloned_handle_shares_log() {
        let h1 = ServiceHandle::new("shared");
        let h2 = h1.clone();

        h1.set_state(ServiceState::Starting);
        h2.set_state(ServiceState::Ready);

        // Both should see all 3 transitions
        assert_eq!(h1.transitions().len(), 3);
        assert_eq!(h2.transitions().len(), 3);
    }
}
