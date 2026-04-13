//! Backend service orchestration.
//!
//! Provides helpers for initializing and managing multiple backend services
//! (FTS, graph, vector) in a Fabryk application. Builds on [`ServiceHandle`]
//! and [`ServiceState`] from the [`service`](crate::service) module.

use std::collections::HashMap;
use std::time::Duration;

use crate::service::{ServiceHandle, ServiceState, wait_all_ready};

// ============================================================================
// ManagedService
// ============================================================================

/// A managed backend service with its handle and metadata.
///
/// Wraps a [`ServiceHandle`] with a human-readable description for reporting
/// and diagnostics. Use [`ServiceOrchestrator`] to register and manage
/// multiple instances.
///
/// # Example
///
/// ```rust
/// use fabryk_core::orchestration::ManagedService;
/// use fabryk_core::service::ServiceState;
///
/// let svc = ManagedService::new("fts")
///     .with_description("Full-text search backend");
///
/// assert_eq!(svc.description(), "Full-text search backend");
/// assert_eq!(svc.state(), ServiceState::Stopped);
/// assert!(!svc.is_ready());
/// ```
pub struct ManagedService {
    /// Service lifecycle handle for state observation.
    pub handle: ServiceHandle,
    /// Human-readable description.
    description: String,
}

impl ManagedService {
    /// Create a new managed service with the given name.
    ///
    /// The description defaults to the name. Use [`with_description`](Self::with_description)
    /// to provide a more detailed description.
    pub fn new(name: &str) -> Self {
        Self {
            handle: ServiceHandle::new(name),
            description: name.to_string(),
        }
    }

    /// Get the human-readable description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Set a human-readable description (builder pattern).
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Get the current service state.
    pub fn state(&self) -> ServiceState {
        self.handle.state()
    }

    /// Returns `true` if the service is fully ready.
    pub fn is_ready(&self) -> bool {
        self.handle.state().is_ready()
    }
}

// ============================================================================
// ServiceStatus
// ============================================================================

/// Status snapshot of a single service for reporting.
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    /// Service name (registration key).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Current lifecycle state.
    pub state: ServiceState,
}

// ============================================================================
// ServiceOrchestrator
// ============================================================================

/// Orchestrates multiple backend services for a Fabryk application.
///
/// Tracks named services and provides aggregate lifecycle management.
/// Services are registered by name and can be queried individually or
/// as a group.
///
/// # Example
///
/// ```rust,ignore
/// let mut orch = ServiceOrchestrator::new();
/// orch.register("fts", ManagedService::new("fts").with_description("Full-text search"));
/// orch.register("graph", ManagedService::new("graph").with_description("Knowledge graph"));
///
/// // Initialize services (spawns async tasks)
/// let fts_handle = orch.handle("fts").unwrap().clone();
/// tokio::spawn(async move {
///     fts_handle.set_state(ServiceState::Starting);
///     // ... build index ...
///     fts_handle.set_state(ServiceState::Ready);
/// });
///
/// // Wait for all to be ready
/// orch.wait_all_ready(Duration::from_secs(30)).await?;
/// ```
pub struct ServiceOrchestrator {
    services: HashMap<String, ManagedService>,
}

impl ServiceOrchestrator {
    /// Create a new, empty orchestrator.
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service under the given name.
    ///
    /// If a service with the same name already exists it is replaced.
    pub fn register(&mut self, name: &str, service: ManagedService) {
        self.services.insert(name.to_string(), service);
    }

    /// Get a service handle by name.
    pub fn handle(&self, name: &str) -> Option<&ServiceHandle> {
        self.services.get(name).map(|s| &s.handle)
    }

    /// Get a managed service by name.
    pub fn service(&self, name: &str) -> Option<&ManagedService> {
        self.services.get(name)
    }

    /// Get all registered service handles.
    pub fn handles(&self) -> Vec<&ServiceHandle> {
        self.services.values().map(|s| &s.handle).collect()
    }

    /// Wait for all registered services to reach `Ready` (or fail).
    ///
    /// Delegates to [`wait_all_ready`](crate::service::wait_all_ready) which
    /// runs all waits concurrently. Returns `Ok(())` if every service reaches
    /// `Ready` within the timeout, or `Err` with a list of failure messages.
    pub async fn wait_all_ready(&self, timeout: Duration) -> Result<(), Vec<String>> {
        let handles: Vec<_> = self.services.values().map(|s| s.handle.clone()).collect();
        wait_all_ready(&handles, timeout).await
    }

    /// Get a summary of all service states.
    ///
    /// Returns a [`ServiceStatus`] for each registered service, sorted by
    /// name for deterministic output.
    pub fn status_summary(&self) -> Vec<ServiceStatus> {
        let mut statuses: Vec<ServiceStatus> = self
            .services
            .iter()
            .map(|(name, svc)| ServiceStatus {
                name: name.clone(),
                description: svc.description.clone(),
                state: svc.handle.state(),
            })
            .collect();
        statuses.sort_by(|a, b| a.name.cmp(&b.name));
        statuses
    }

    /// Check if all registered services are ready.
    ///
    /// Returns `true` if there are no registered services, or if every
    /// registered service is in the `Ready` state.
    pub fn all_ready(&self) -> bool {
        self.services.values().all(|s| s.is_ready())
    }

    /// Get names of services that are not in the `Ready` state.
    ///
    /// Returns an empty vector if all services are ready. The returned
    /// names are sorted for deterministic output.
    pub fn not_ready(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .services
            .iter()
            .filter(|(_, svc)| !svc.is_ready())
            .map(|(name, _)| name.as_str())
            .collect();
        names.sort();
        names
    }
}

impl Default for ServiceOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_managed_service_new() {
        let svc = ManagedService::new("fts");
        assert_eq!(svc.description(), "fts");
        assert_eq!(svc.state(), ServiceState::Stopped);
        assert!(!svc.is_ready());
    }

    #[test]
    fn test_managed_service_with_description() {
        let svc = ManagedService::new("fts").with_description("Full-text search");
        assert_eq!(svc.description(), "Full-text search");
    }

    #[test]
    fn test_orchestrator_register_and_handle() {
        let mut orch = ServiceOrchestrator::new();
        orch.register(
            "fts",
            ManagedService::new("fts").with_description("Full-text search"),
        );
        orch.register(
            "graph",
            ManagedService::new("graph").with_description("Knowledge graph"),
        );

        assert!(orch.handle("fts").is_some());
        assert!(orch.handle("graph").is_some());
        assert!(orch.handle("nonexistent").is_none());

        assert!(orch.service("fts").is_some());
        assert_eq!(
            orch.service("fts").unwrap().description(),
            "Full-text search"
        );

        assert_eq!(orch.handles().len(), 2);
    }

    #[test]
    fn test_orchestrator_all_ready_when_all_services_ready() {
        let mut orch = ServiceOrchestrator::new();

        let fts = ManagedService::new("fts");
        fts.handle.set_state(ServiceState::Ready);
        orch.register("fts", fts);

        let graph = ManagedService::new("graph");
        graph.handle.set_state(ServiceState::Ready);
        orch.register("graph", graph);

        assert!(orch.all_ready());
        assert!(orch.not_ready().is_empty());
    }

    #[test]
    fn test_orchestrator_not_ready_when_one_service_not_ready() {
        let mut orch = ServiceOrchestrator::new();

        let fts = ManagedService::new("fts");
        fts.handle.set_state(ServiceState::Ready);
        orch.register("fts", fts);

        let graph = ManagedService::new("graph");
        graph.handle.set_state(ServiceState::Starting);
        orch.register("graph", graph);

        assert!(!orch.all_ready());
        assert_eq!(orch.not_ready(), vec!["graph"]);
    }

    #[test]
    fn test_orchestrator_status_summary() {
        let mut orch = ServiceOrchestrator::new();

        let fts = ManagedService::new("fts").with_description("Full-text search");
        fts.handle.set_state(ServiceState::Ready);
        orch.register("fts", fts);

        let graph = ManagedService::new("graph").with_description("Knowledge graph");
        graph.handle.set_state(ServiceState::Starting);
        orch.register("graph", graph);

        let summary = orch.status_summary();
        assert_eq!(summary.len(), 2);

        // Sorted by name
        assert_eq!(summary[0].name, "fts");
        assert_eq!(summary[0].description, "Full-text search");
        assert_eq!(summary[0].state, ServiceState::Ready);

        assert_eq!(summary[1].name, "graph");
        assert_eq!(summary[1].description, "Knowledge graph");
        assert_eq!(summary[1].state, ServiceState::Starting);
    }

    #[tokio::test]
    async fn test_orchestrator_wait_all_ready_services_transition() {
        let mut orch = ServiceOrchestrator::new();

        let fts = ManagedService::new("fts");
        let fts_handle = fts.handle.clone();
        orch.register("fts", fts);

        let graph = ManagedService::new("graph");
        let graph_handle = graph.handle.clone();
        orch.register("graph", graph);

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            fts_handle.set_state(ServiceState::Starting);
            tokio::time::sleep(Duration::from_millis(10)).await;
            fts_handle.set_state(ServiceState::Ready);
        });

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            graph_handle.set_state(ServiceState::Starting);
            tokio::time::sleep(Duration::from_millis(5)).await;
            graph_handle.set_state(ServiceState::Ready);
        });

        let result = orch.wait_all_ready(Duration::from_secs(1)).await;
        assert!(result.is_ok());
        assert!(orch.all_ready());
    }

    #[test]
    fn test_orchestrator_empty_all_ready() {
        let orch = ServiceOrchestrator::new();
        assert!(orch.all_ready());
        assert!(orch.not_ready().is_empty());
        assert!(orch.status_summary().is_empty());
    }

    #[test]
    fn test_orchestrator_default() {
        let orch = ServiceOrchestrator::default();
        assert!(orch.all_ready());
        assert!(orch.handles().is_empty());
    }
}
