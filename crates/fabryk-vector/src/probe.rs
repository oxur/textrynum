//! Diagnostic probe adapter for vector backends.
//!
//! Wraps a [`VectorBackend`] as a [`BackendProbe`] so that vector engines
//! can participate in health-check reporting without `fabryk-mcp-core`
//! depending on `fabryk-vector`.

use std::sync::Arc;

use fabryk_core::BackendProbe;

use crate::backend::VectorBackend;

/// Wraps a [`VectorBackend`] as a [`BackendProbe`] for health diagnostics.
///
/// Unlike the FTS probe, [`VectorProbe`] also reports
/// [`probe_document_count()`](BackendProbe::probe_document_count) by
/// delegating to [`VectorBackend::document_count()`].
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use fabryk_vector::{VectorProbe, vector_probe, SimpleVectorBackend, MockEmbeddingProvider};
///
/// let provider = Arc::new(MockEmbeddingProvider::new(8));
/// let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(provider));
/// let probe = vector_probe(backend);
/// assert_eq!(probe.probe_kind(), "vector");
/// ```
pub struct VectorProbe<B: VectorBackend + ?Sized> {
    backend: Arc<B>,
}

impl<B: VectorBackend + ?Sized> VectorProbe<B> {
    /// Create a new vector probe wrapping the given backend.
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }
}

impl<B: VectorBackend + ?Sized + 'static> BackendProbe for VectorProbe<B> {
    fn probe_name(&self) -> &str {
        self.backend.name()
    }

    fn probe_ready(&self) -> bool {
        self.backend.is_ready()
    }

    fn probe_document_count(&self) -> Option<usize> {
        self.backend.document_count().ok()
    }

    fn probe_kind(&self) -> &str {
        "vector"
    }
}

/// Create a [`BackendProbe`] from a vector backend trait object.
///
/// Convenience function for constructing a probe from `Arc<dyn VectorBackend>`.
pub fn vector_probe(backend: Arc<dyn VectorBackend>) -> Arc<dyn BackendProbe> {
    Arc::new(VectorProbe::new(backend))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{SimpleVectorBackend, VectorBackend};
    use crate::embedding::MockEmbeddingProvider;
    use fabryk_core::BackendProbe;

    fn mock_provider() -> Arc<dyn crate::embedding::EmbeddingProvider> {
        Arc::new(MockEmbeddingProvider::new(8))
    }

    #[test]
    fn test_vector_probe_name() {
        let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(mock_provider()));
        let probe = VectorProbe::new(backend);
        assert_eq!(probe.probe_name(), "simple");
    }

    #[test]
    fn test_vector_probe_ready() {
        let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(mock_provider()));
        let probe = VectorProbe::new(backend);
        assert!(probe.probe_ready());
    }

    #[test]
    fn test_vector_probe_kind() {
        let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(mock_provider()));
        let probe = VectorProbe::new(backend);
        assert_eq!(probe.probe_kind(), "vector");
    }

    #[test]
    fn test_vector_probe_document_count() {
        let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(mock_provider()));
        let probe = VectorProbe::new(backend);
        // SimpleVectorBackend starts with 0 documents
        assert_eq!(probe.probe_document_count(), Some(0));
    }

    #[test]
    fn test_vector_probe_last_indexed_default() {
        let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(mock_provider()));
        let probe = VectorProbe::new(backend);
        assert!(probe.probe_last_indexed().is_none());
    }

    #[test]
    fn test_vector_probe_convenience_function() {
        let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(mock_provider()));
        let probe = vector_probe(backend);
        assert_eq!(probe.probe_name(), "simple");
        assert_eq!(probe.probe_kind(), "vector");
        assert!(probe.probe_ready());
        assert_eq!(probe.probe_document_count(), Some(0));
    }

    #[test]
    fn test_vector_probe_as_arc_dyn_backend_probe() {
        let backend: Arc<dyn VectorBackend> = Arc::new(SimpleVectorBackend::new(mock_provider()));
        let probe: Arc<dyn BackendProbe> = Arc::new(VectorProbe::new(backend));
        assert_eq!(probe.probe_name(), "simple");
        assert_eq!(probe.probe_kind(), "vector");
    }
}
