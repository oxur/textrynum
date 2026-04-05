//! Diagnostic probe adapter for search backends.
//!
//! Wraps a [`SearchBackend`] as a [`BackendProbe`] so that search engines
//! can participate in health-check reporting without `fabryk-mcp-core`
//! depending on `fabryk-fts`.

use std::sync::Arc;

use fabryk_core::BackendProbe;

use crate::backend::SearchBackend;

/// Wraps a [`SearchBackend`] as a [`BackendProbe`] for health diagnostics.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use fabryk_fts::{SearchProbe, search_probe, SimpleSearch, SearchConfig};
///
/// let config = SearchConfig::default();
/// let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
/// let probe = search_probe(backend);
/// assert_eq!(probe.probe_kind(), "fts");
/// ```
pub struct SearchProbe<B: SearchBackend + ?Sized> {
    backend: Arc<B>,
}

impl<B: SearchBackend + ?Sized> SearchProbe<B> {
    /// Create a new search probe wrapping the given backend.
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }
}

impl<B: SearchBackend + ?Sized + 'static> BackendProbe for SearchProbe<B> {
    fn probe_name(&self) -> &str {
        self.backend.name()
    }

    fn probe_ready(&self) -> bool {
        self.backend.is_ready()
    }

    fn probe_kind(&self) -> &str {
        "fts"
    }
}

/// Create a [`BackendProbe`] from a search backend trait object.
///
/// Convenience function for constructing a probe from `Arc<dyn SearchBackend>`.
pub fn search_probe(backend: Arc<dyn SearchBackend>) -> Arc<dyn BackendProbe> {
    Arc::new(SearchProbe::new(backend))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{SearchBackend, SimpleSearch};
    use crate::types::SearchConfig;
    use fabryk_core::BackendProbe;

    #[test]
    fn test_search_probe_name() {
        let config = SearchConfig::default();
        let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
        let probe = SearchProbe::new(backend);
        assert_eq!(probe.probe_name(), "simple");
    }

    #[test]
    fn test_search_probe_ready() {
        let config = SearchConfig::default();
        let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
        let probe = SearchProbe::new(backend);
        assert!(probe.probe_ready());
    }

    #[test]
    fn test_search_probe_kind() {
        let config = SearchConfig::default();
        let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
        let probe = SearchProbe::new(backend);
        assert_eq!(probe.probe_kind(), "fts");
    }

    #[test]
    fn test_search_probe_document_count_default() {
        let config = SearchConfig::default();
        let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
        let probe = SearchProbe::new(backend);
        assert!(probe.probe_document_count().is_none());
    }

    #[test]
    fn test_search_probe_last_indexed_default() {
        let config = SearchConfig::default();
        let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
        let probe = SearchProbe::new(backend);
        assert!(probe.probe_last_indexed().is_none());
    }

    #[test]
    fn test_search_probe_convenience_function() {
        let config = SearchConfig::default();
        let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
        let probe = search_probe(backend);
        assert_eq!(probe.probe_name(), "simple");
        assert_eq!(probe.probe_kind(), "fts");
        assert!(probe.probe_ready());
    }

    #[test]
    fn test_search_probe_as_arc_dyn_backend_probe() {
        let config = SearchConfig::default();
        let backend: Arc<dyn SearchBackend> = Arc::new(SimpleSearch::new(&config));
        let probe: Arc<dyn BackendProbe> = Arc::new(SearchProbe::new(backend));
        assert_eq!(probe.probe_name(), "simple");
    }
}
