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

/// Thread-safe handle for observing and updating service state.
///
/// Cheap to clone (Arc internals). State changes are broadcast
/// to all subscribers via a watch channel.
#[derive(Clone)]
pub struct ServiceHandle {
    inner: Arc<ServiceHandleInner>,
}

struct ServiceHandleInner {
    name: String,
    tx: watch::Sender<ServiceState>,
    started_at: Instant,
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
    /// All subscribers are notified of the change.
    pub fn set_state(&self, state: ServiceState) {
        log::info!("Service '{}' → {state}", self.inner.name);
        self.inner.tx.send_replace(state);
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
}
