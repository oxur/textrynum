//! Generic backend slot combining service lifecycle with backend storage.
//!
//! [`BackendSlot`] bundles a [`ServiceHandle`] (lifecycle state machine) with
//! an `Arc<RwLock<Option<B>>>` (backend value), providing state-safe access to
//! an optionally-present backend that is populated asynchronously.

use std::fmt;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use crate::Error;
use crate::service::{ServiceHandle, ServiceState};

/// A managed backend slot with integrated service lifecycle.
///
/// Combines a [`ServiceHandle`] (for state tracking) with `Arc<RwLock<Option<B>>>`
/// (for backend storage), providing state-safe access to an optionally-present
/// backend that is populated asynchronously.
///
/// # Example
///
/// ```rust
/// use fabryk_core::BackendSlot;
/// use fabryk_core::service::ServiceState;
///
/// let slot: BackendSlot<String> = BackendSlot::new("my-backend");
/// assert!(!slot.is_ready());
///
/// slot.service().set_state(ServiceState::Starting);
/// slot.set("hello".to_string()).unwrap();
/// slot.service().set_state(ServiceState::Ready);
///
/// assert!(slot.is_ready());
/// let guard = slot.require().unwrap();
/// assert_eq!(guard.as_ref().unwrap(), "hello");
/// ```
#[derive(Clone)]
pub struct BackendSlot<B> {
    service: ServiceHandle,
    backend: Arc<RwLock<Option<B>>>,
}

impl<B> BackendSlot<B> {
    /// Create a new empty slot with the given service name.
    ///
    /// The service starts in [`ServiceState::Stopped`] and the backend is
    /// `None`. Callers should populate the backend with [`set`](Self::set)
    /// and advance the service state to [`ServiceState::Ready`] once
    /// initialisation completes.
    pub fn new(name: &str) -> Self {
        Self {
            service: ServiceHandle::new(name),
            backend: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the service handle for state observation/mutation.
    pub fn service(&self) -> &ServiceHandle {
        &self.service
    }

    /// Check if the service is in the [`ServiceState::Ready`] state.
    pub fn is_ready(&self) -> bool {
        self.service.state().is_ready()
    }

    /// Store a backend value in the slot.
    ///
    /// Does **not** change service state — the caller should set
    /// [`ServiceState::Ready`] after this succeeds.
    pub fn set(&self, value: B) -> Result<(), Error> {
        let mut guard = self.backend.write().map_err(|_| {
            Error::operation(format!(
                "Failed to acquire write lock for {}",
                self.service.name()
            ))
        })?;
        *guard = Some(value);
        Ok(())
    }

    /// Acquire a read guard, returning an error if the service is not
    /// [`ServiceState::Ready`].
    ///
    /// When the service is `Ready`, the inner `Option` is guaranteed to be
    /// `Some` (assuming the caller populated it before marking ready).
    pub fn require(&self) -> Result<RwLockReadGuard<'_, Option<B>>, Error> {
        let state = self.service.state();
        if !state.is_ready() {
            return match state {
                ServiceState::Starting => Err(Error::not_found_msg(format!(
                    "{} is currently initializing",
                    self.service.name()
                ))),
                ServiceState::Failed(msg) => Err(Error::operation(format!(
                    "{} failed to initialize: {}",
                    self.service.name(),
                    msg
                ))),
                _ => Err(Error::not_found_msg(format!(
                    "{} not initialized yet",
                    self.service.name()
                ))),
            };
        }
        self.backend.read().map_err(|_| {
            Error::operation(format!(
                "Failed to acquire read lock for {}",
                self.service.name()
            ))
        })
    }

    /// Get direct access to the inner `Arc<RwLock>` for cases that need it
    /// (e.g., creating a clone for async tasks).
    pub fn inner(&self) -> &Arc<RwLock<Option<B>>> {
        &self.backend
    }
}

impl<B: fmt::Debug> fmt::Debug for BackendSlot<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend_desc = match self.backend.read() {
            Ok(guard) => match guard.as_ref() {
                Some(val) => format!("Some({val:?})"),
                None => "None".to_string(),
            },
            Err(_) => "<locked>".to_string(),
        };
        f.debug_struct("BackendSlot")
            .field("service", &self.service)
            .field("backend", &backend_desc)
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
    fn test_new_slot_not_ready() {
        let slot: BackendSlot<String> = BackendSlot::new("test");
        assert!(!slot.is_ready());
        assert_eq!(slot.service().name(), "test");
    }

    #[test]
    fn test_set_and_require() {
        let slot: BackendSlot<String> = BackendSlot::new("test");
        slot.service().set_state(ServiceState::Starting);
        slot.set("hello".to_string()).unwrap();
        slot.service().set_state(ServiceState::Ready);

        assert!(slot.is_ready());
        let guard = slot.require().unwrap();
        assert_eq!(guard.as_ref().unwrap(), "hello");
    }

    #[test]
    fn test_require_when_not_ready() {
        let slot: BackendSlot<String> = BackendSlot::new("test");
        let err = slot.require().unwrap_err();
        assert!(err.to_string().contains("not initialized yet"));
    }

    #[test]
    fn test_require_when_starting() {
        let slot: BackendSlot<String> = BackendSlot::new("test");
        slot.service().set_state(ServiceState::Starting);
        let err = slot.require().unwrap_err();
        assert!(
            err.to_string().contains("currently initializing"),
            "Expected 'currently initializing', got: {}",
            err
        );
    }

    #[test]
    fn test_require_when_failed() {
        let slot: BackendSlot<String> = BackendSlot::new("test");
        slot.service()
            .set_state(ServiceState::Failed("disk full".to_string()));
        let err = slot.require().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("failed to initialize") && msg.contains("disk full"),
            "Expected failure message, got: {msg}"
        );
    }

    #[test]
    fn test_clone_shares_backend() {
        let slot: BackendSlot<String> = BackendSlot::new("shared");
        let clone = slot.clone();

        slot.service().set_state(ServiceState::Starting);
        slot.set("shared-value".to_string()).unwrap();
        slot.service().set_state(ServiceState::Ready);

        // The clone should see the same service state and backend value.
        assert!(clone.is_ready());
        let guard = clone.require().unwrap();
        assert_eq!(guard.as_ref().unwrap(), "shared-value");
    }

    #[test]
    fn test_set_without_ready() {
        let slot: BackendSlot<String> = BackendSlot::new("test");
        slot.set("value".to_string()).unwrap();

        // Value is stored, but require should still fail because the
        // service is not in the Ready state.
        assert!(!slot.is_ready());
        let err = slot.require().unwrap_err();
        assert!(err.to_string().contains("not initialized yet"));
    }

    #[test]
    fn test_debug_with_value() {
        let slot: BackendSlot<u32> = BackendSlot::new("debug-test");
        slot.set(42).unwrap();
        let debug = format!("{slot:?}");
        assert!(debug.contains("BackendSlot"));
        assert!(debug.contains("debug-test"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn test_debug_empty() {
        let slot: BackendSlot<u32> = BackendSlot::new("empty");
        let debug = format!("{slot:?}");
        assert!(debug.contains("None"));
    }

    #[test]
    fn test_inner_returns_arc() {
        let slot: BackendSlot<String> = BackendSlot::new("inner-test");
        let inner = slot.inner().clone();
        slot.set("via-slot".to_string()).unwrap();

        let guard = inner.read().unwrap();
        assert_eq!(guard.as_ref().unwrap(), "via-slot");
    }

    // Compile-time check: BackendSlot<B> must be Send + Sync when B is.
    fn _assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_backend_slot_send_sync() {
        _assert_send_sync::<BackendSlot<String>>();
        _assert_send_sync::<BackendSlot<Vec<u8>>>();
    }
}
