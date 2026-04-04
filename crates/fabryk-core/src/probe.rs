//! Minimal diagnostic probe trait for backend services.
//!
//! [`BackendProbe`] is implemented by search backends, vector backends,
//! and any other service that should report status in health checks.
//!
//! The trait lives in `fabryk-core` (dependency level 0) so that
//! `fabryk-mcp-core` can consume probes without depending on
//! `fabryk-fts` or `fabryk-vector`.
//!
//! Method names use a `probe_` prefix to avoid conflicts with existing
//! trait methods like [`SearchBackend::name()`] and [`VectorBackend::name()`].

/// Minimal diagnostic probe for a backend service.
///
/// Implemented by search backends, vector backends, and any other
/// service that should report status in health checks.
///
/// # Object Safety
///
/// This trait is object-safe and intended to be used as `dyn BackendProbe`.
///
/// # Example
///
/// ```
/// use fabryk_core::BackendProbe;
///
/// struct MyProbe;
///
/// impl BackendProbe for MyProbe {
///     fn probe_name(&self) -> &str { "my-backend" }
///     fn probe_ready(&self) -> bool { true }
///     fn probe_kind(&self) -> &str { "custom" }
/// }
///
/// let probe: Box<dyn BackendProbe> = Box::new(MyProbe);
/// assert_eq!(probe.probe_name(), "my-backend");
/// assert!(probe.probe_ready());
/// assert_eq!(probe.probe_kind(), "custom");
/// assert!(probe.probe_document_count().is_none());
/// assert!(probe.probe_last_indexed().is_none());
/// ```
pub trait BackendProbe: Send + Sync {
    /// Backend identifier (e.g., "tantivy", "simple", "lancedb").
    fn probe_name(&self) -> &str;

    /// Whether the backend is ready to handle requests.
    fn probe_ready(&self) -> bool;

    /// Number of documents/items indexed (if applicable).
    fn probe_document_count(&self) -> Option<usize> {
        None
    }

    /// When the backend was last updated/indexed (if applicable).
    fn probe_last_indexed(&self) -> Option<String> {
        None
    }

    /// Backend kind for grouping in health output (e.g., "fts", "vector").
    fn probe_kind(&self) -> &str {
        "unknown"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct TestProbe {
        name: &'static str,
        ready: bool,
        kind: &'static str,
        doc_count: Option<usize>,
        last_indexed: Option<String>,
    }

    impl TestProbe {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                ready: true,
                kind: "test",
                doc_count: None,
                last_indexed: None,
            }
        }

        fn with_ready(mut self, ready: bool) -> Self {
            self.ready = ready;
            self
        }

        fn with_kind(mut self, kind: &'static str) -> Self {
            self.kind = kind;
            self
        }

        fn with_document_count(mut self, count: usize) -> Self {
            self.doc_count = Some(count);
            self
        }

        fn with_last_indexed(mut self, ts: impl Into<String>) -> Self {
            self.last_indexed = Some(ts.into());
            self
        }
    }

    impl BackendProbe for TestProbe {
        fn probe_name(&self) -> &str {
            self.name
        }

        fn probe_ready(&self) -> bool {
            self.ready
        }

        fn probe_document_count(&self) -> Option<usize> {
            self.doc_count
        }

        fn probe_last_indexed(&self) -> Option<String> {
            self.last_indexed.clone()
        }

        fn probe_kind(&self) -> &str {
            self.kind
        }
    }

    #[test]
    fn test_backend_probe_name() {
        let probe = TestProbe::new("tantivy");
        assert_eq!(probe.probe_name(), "tantivy");
    }

    #[test]
    fn test_backend_probe_ready_true() {
        let probe = TestProbe::new("test");
        assert!(probe.probe_ready());
    }

    #[test]
    fn test_backend_probe_ready_false() {
        let probe = TestProbe::new("test").with_ready(false);
        assert!(!probe.probe_ready());
    }

    #[test]
    fn test_backend_probe_kind() {
        let probe = TestProbe::new("test").with_kind("fts");
        assert_eq!(probe.probe_kind(), "fts");
    }

    #[test]
    fn test_backend_probe_document_count_none() {
        let probe = TestProbe::new("test");
        assert!(probe.probe_document_count().is_none());
    }

    #[test]
    fn test_backend_probe_document_count_some() {
        let probe = TestProbe::new("test").with_document_count(42);
        assert_eq!(probe.probe_document_count(), Some(42));
    }

    #[test]
    fn test_backend_probe_last_indexed_none() {
        let probe = TestProbe::new("test");
        assert!(probe.probe_last_indexed().is_none());
    }

    #[test]
    fn test_backend_probe_last_indexed_some() {
        let probe = TestProbe::new("test").with_last_indexed("2026-04-02T00:00:00Z");
        assert_eq!(
            probe.probe_last_indexed(),
            Some("2026-04-02T00:00:00Z".to_string())
        );
    }

    #[test]
    fn test_backend_probe_default_document_count() {
        // Verify the default impl returns None
        struct MinimalProbe;
        impl BackendProbe for MinimalProbe {
            fn probe_name(&self) -> &str {
                "minimal"
            }
            fn probe_ready(&self) -> bool {
                true
            }
        }

        let probe = MinimalProbe;
        assert!(probe.probe_document_count().is_none());
    }

    #[test]
    fn test_backend_probe_default_last_indexed() {
        struct MinimalProbe;
        impl BackendProbe for MinimalProbe {
            fn probe_name(&self) -> &str {
                "minimal"
            }
            fn probe_ready(&self) -> bool {
                true
            }
        }

        let probe = MinimalProbe;
        assert!(probe.probe_last_indexed().is_none());
    }

    #[test]
    fn test_backend_probe_default_kind() {
        struct MinimalProbe;
        impl BackendProbe for MinimalProbe {
            fn probe_name(&self) -> &str {
                "minimal"
            }
            fn probe_ready(&self) -> bool {
                true
            }
        }

        let probe = MinimalProbe;
        assert_eq!(probe.probe_kind(), "unknown");
    }

    #[test]
    fn test_backend_probe_is_object_safe() {
        // Verify BackendProbe can be used as a trait object
        fn assert_object_safe(_: &dyn BackendProbe) {}

        let probe = TestProbe::new("test");
        assert_object_safe(&probe);
    }

    #[test]
    fn test_backend_probe_as_arc_dyn() {
        let probe: Arc<dyn BackendProbe> = Arc::new(TestProbe::new("arc-test").with_kind("fts"));
        assert_eq!(probe.probe_name(), "arc-test");
        assert_eq!(probe.probe_kind(), "fts");
        assert!(probe.probe_ready());
    }

    #[test]
    fn test_backend_probe_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TestProbe>();
    }

    #[test]
    fn test_backend_probe_vec_of_arc_dyn() {
        let probes: Vec<Arc<dyn BackendProbe>> = vec![
            Arc::new(TestProbe::new("fts-backend").with_kind("fts")),
            Arc::new(
                TestProbe::new("vector-backend")
                    .with_kind("vector")
                    .with_document_count(100),
            ),
        ];

        assert_eq!(probes.len(), 2);
        assert_eq!(probes[0].probe_name(), "fts-backend");
        assert_eq!(probes[1].probe_document_count(), Some(100));
    }
}
