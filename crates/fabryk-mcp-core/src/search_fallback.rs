//! Search backend fallback pattern.
//!
//! Provides the [`SearchFallback`] trait for MCP servers that want transparent
//! FTS-with-simple-backend failover. When the FTS backend is ready, it's
//! returned; otherwise, the always-available simple backend is used.

use std::sync::Arc;

use fabryk_core::BackendSlot;
use fabryk_fts::SearchBackend;

/// Trait for MCP application state that provides search backend fallback.
///
/// Implementations should return the best available search backend:
/// FTS if ready, simple (linear scan) otherwise. This enables transparent
/// failover during FTS initialization.
///
/// # Example
///
/// ```rust,ignore
/// impl SearchFallback for MyAppState {
///     fn search_backend(&self) -> Arc<dyn SearchBackend + Send + Sync> {
///         if self.fts.is_ready() {
///             if let Ok(guard) = self.fts.inner().read() {
///                 if let Some(ref backend) = *guard {
///                     return Arc::clone(backend) as _;
///                 }
///             }
///         }
///         Arc::clone(&self.simple_backend) as _
///     }
///
///     fn active_backend_name(&self) -> &'static str {
///         if self.fts.is_ready() { "tantivy" } else { "simple" }
///     }
/// }
/// ```
pub trait SearchFallback {
    /// Get the currently active search backend.
    ///
    /// Returns FTS backend if ready, otherwise the simple fallback.
    fn search_backend(&self) -> Arc<dyn SearchBackend + Send + Sync>;

    /// Get the name of the currently active backend.
    fn active_backend_name(&self) -> &'static str;
}

/// Resolve the best available search backend from an optional FTS slot
/// and an always-available simple backend.
///
/// This is the generic implementation of the FTS-with-simple-fallback
/// pattern. Application state types can call this from their
/// [`SearchFallback`] implementation.
///
/// # Arguments
///
/// * `fts_slot` - Optional FTS backend slot (feature-gated in caller)
/// * `simple` - Always-available simple search backend
pub fn resolve_search_backend<T>(
    fts_slot: Option<&BackendSlot<Arc<T>>>,
    simple: &Arc<dyn SearchBackend + Send + Sync>,
) -> Arc<dyn SearchBackend + Send + Sync>
where
    T: SearchBackend + Send + Sync + 'static,
{
    if let Some(slot) = fts_slot
        && slot.is_ready()
        && let Ok(guard) = slot.inner().read()
        && let Some(ref backend) = *guard
    {
        return Arc::clone(backend) as Arc<dyn SearchBackend + Send + Sync>;
    }
    Arc::clone(simple)
}

/// Resolve the name of the active backend.
///
/// Returns `"tantivy"` if the FTS slot is ready, `"simple"` otherwise.
pub fn resolve_backend_name<T>(fts_slot: Option<&BackendSlot<Arc<T>>>) -> &'static str {
    if let Some(slot) = fts_slot
        && slot.is_ready()
    {
        return "tantivy";
    }
    "simple"
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use fabryk_fts::SimpleSearch;

    #[test]
    fn test_resolve_backend_name_no_fts() {
        // Use SimpleSearch as a stand-in for any SearchBackend impl
        let name = resolve_backend_name::<SimpleSearch>(None);
        assert_eq!(name, "simple");
    }

    #[test]
    fn test_resolve_backend_name_fts_not_ready() {
        let slot: BackendSlot<Arc<SimpleSearch>> = BackendSlot::new("fts");
        let name = resolve_backend_name(Some(&slot));
        assert_eq!(name, "simple");
    }

    #[test]
    fn test_resolve_search_backend_no_fts() {
        let config = fabryk_fts::SearchConfig::default();
        let simple: Arc<dyn SearchBackend + Send + Sync> =
            Arc::new(SimpleSearch::with_default_extractor(&config));
        let backend = resolve_search_backend::<SimpleSearch>(None, &simple);
        assert_eq!(backend.name(), "simple");
    }
}
