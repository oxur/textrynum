//! Adapter and stage registries for runtime resolution.
//!
//! These registries map string identifiers (from TOML configuration) to
//! factory functions that produce concrete trait implementations. This is
//! the bridge between the declarative specification layer and the concrete
//! topology layer.

use std::collections::BTreeMap;
use std::sync::Arc;

use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_spec::StageSpec;
use ecl_pipeline_topo::error::ResolveError;
use ecl_pipeline_topo::{SourceAdapter, Stage};

/// Type alias for source adapter factory functions.
///
/// A factory takes a `&SourceSpec` and returns a concrete `SourceAdapter`
/// implementation wrapped in `Arc` for shared ownership in the topology.
pub type AdapterFactory =
    Box<dyn Fn(&SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError> + Send + Sync>;

/// Type alias for stage handler factory functions.
///
/// A factory takes a `&StageSpec` and returns a concrete `Stage`
/// implementation wrapped in `Arc` for shared ownership in the topology.
pub type StageFactory =
    Box<dyn Fn(&StageSpec) -> Result<Arc<dyn Stage>, ResolveError> + Send + Sync>;

/// Registry of source adapter factories, keyed by source `kind` string.
///
/// The `kind` string matches the `SourceSpec` variant tag in TOML:
/// - `"google_drive"` -> Google Drive adapter
/// - `"slack"` -> Slack adapter
/// - `"filesystem"` -> Filesystem adapter
///
/// # Example
///
/// ```ignore
/// let mut registry = AdapterRegistry::new();
/// registry.register("filesystem", Box::new(|spec| {
///     Ok(Arc::new(FilesystemAdapter::from_spec(spec)?))
/// }));
/// ```
#[derive(Default)]
pub struct AdapterRegistry {
    factories: BTreeMap<String, AdapterFactory>,
}

impl AdapterRegistry {
    /// Create an empty adapter registry.
    pub fn new() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }

    /// Register a factory function for a source kind.
    ///
    /// If a factory was already registered for this kind, it is replaced.
    pub fn register(&mut self, kind: &str, factory: AdapterFactory) {
        self.factories.insert(kind.to_string(), factory);
    }

    /// Look up and invoke the factory for the given source kind.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAdapter` if no factory is registered
    /// for the given kind.
    pub fn resolve(
        &self,
        kind: &str,
        source_name: &str,
        spec: &SourceSpec,
    ) -> Result<Arc<dyn SourceAdapter>, ResolveError> {
        let factory = self
            .factories
            .get(kind)
            .ok_or_else(|| ResolveError::UnknownAdapter {
                stage: source_name.to_string(),
                adapter: kind.to_string(),
            })?;
        factory(spec)
    }

    /// Returns true if a factory is registered for the given kind.
    pub fn contains(&self, kind: &str) -> bool {
        self.factories.contains_key(kind)
    }

    /// Returns the number of registered factories.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Returns true if no factories are registered.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

impl std::fmt::Debug for AdapterRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterRegistry")
            .field(
                "registered_kinds",
                &self.factories.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Registry of stage handler factories, keyed by stage `adapter` string.
///
/// The `adapter` string matches `StageSpec.adapter` in TOML:
/// - `"extract"` -> Extract stage (delegates to SourceAdapter)
/// - `"normalize"` -> Normalization stage
/// - `"emit"` -> Emit/output stage
///
/// # Example
///
/// ```ignore
/// let mut registry = StageRegistry::new();
/// registry.register("extract", Box::new(|spec| {
///     Ok(Arc::new(ExtractStage::from_spec(spec)?))
/// }));
/// ```
#[derive(Default)]
pub struct StageRegistry {
    factories: BTreeMap<String, StageFactory>,
}

impl StageRegistry {
    /// Create an empty stage registry.
    pub fn new() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }

    /// Register a factory function for a stage adapter name.
    ///
    /// If a factory was already registered for this adapter, it is replaced.
    pub fn register(&mut self, adapter: &str, factory: StageFactory) {
        self.factories.insert(adapter.to_string(), factory);
    }

    /// Look up and invoke the factory for the given stage adapter name.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAdapter` if no factory is registered
    /// for the given adapter name.
    pub fn resolve(
        &self,
        adapter: &str,
        stage_name: &str,
        spec: &StageSpec,
    ) -> Result<Arc<dyn Stage>, ResolveError> {
        let factory = self
            .factories
            .get(adapter)
            .ok_or_else(|| ResolveError::UnknownAdapter {
                stage: stage_name.to_string(),
                adapter: adapter.to_string(),
            })?;
        factory(spec)
    }

    /// Returns true if a factory is registered for the given adapter.
    pub fn contains(&self, adapter: &str) -> bool {
        self.factories.contains_key(adapter)
    }

    /// Returns the number of registered factories.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Returns true if no factories are registered.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

impl std::fmt::Debug for StageRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StageRegistry")
            .field(
                "registered_adapters",
                &self.factories.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ecl_pipeline_spec::ResourceSpec;
    use ecl_pipeline_spec::source::FilesystemSourceSpec;
    use ecl_pipeline_topo::error::SourceError;
    use ecl_pipeline_topo::{
        ExtractedDocument, PipelineItem, SourceItem, StageContext, StageError,
    };

    #[derive(Debug)]
    struct MockSourceAdapter {
        kind: String,
    }

    #[async_trait]
    impl SourceAdapter for MockSourceAdapter {
        fn source_kind(&self) -> &str {
            &self.kind
        }
        async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
            Ok(vec![])
        }
        async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
            Err(SourceError::NotFound {
                source_name: self.kind.clone(),
                item_id: "none".to_string(),
            })
        }
    }

    #[derive(Debug)]
    struct MockStage {
        name: String,
    }

    #[async_trait]
    impl Stage for MockStage {
        fn name(&self) -> &str {
            &self.name
        }
        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> Result<Vec<PipelineItem>, StageError> {
            Ok(vec![item])
        }
    }

    // ── AdapterRegistry tests ────────────────────────────────────────

    #[test]
    fn test_adapter_registry_new_is_empty() {
        let registry = AdapterRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_adapter_registry_register_and_contains() {
        let mut registry = AdapterRegistry::new();
        registry.register(
            "filesystem",
            Box::new(|_spec| {
                Ok(Arc::new(MockSourceAdapter {
                    kind: "filesystem".to_string(),
                }))
            }),
        );
        assert!(registry.contains("filesystem"));
        assert!(!registry.contains("google_drive"));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_adapter_registry_resolve_unknown_kind_returns_error() {
        let registry = AdapterRegistry::new();
        let spec = SourceSpec::Filesystem(FilesystemSourceSpec {
            root: std::path::PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec![],
        });
        let result = registry.resolve("nonexistent", "my-source", &spec);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ResolveError::UnknownAdapter { .. }));
    }

    #[test]
    fn test_adapter_registry_resolve_calls_factory() {
        let mut registry = AdapterRegistry::new();
        registry.register(
            "filesystem",
            Box::new(|_spec| {
                Ok(Arc::new(MockSourceAdapter {
                    kind: "filesystem".to_string(),
                }))
            }),
        );
        let spec = SourceSpec::Filesystem(FilesystemSourceSpec {
            root: std::path::PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec![],
        });
        let adapter = registry.resolve("filesystem", "local", &spec).unwrap();
        assert_eq!(adapter.source_kind(), "filesystem");
    }

    #[test]
    fn test_adapter_registry_debug_shows_registered_kinds() {
        let mut registry = AdapterRegistry::new();
        registry.register(
            "filesystem",
            Box::new(|_spec| {
                Ok(Arc::new(MockSourceAdapter {
                    kind: "filesystem".to_string(),
                }))
            }),
        );
        let debug = format!("{registry:?}");
        assert!(
            debug.contains("filesystem"),
            "debug should show registered kinds"
        );
    }

    // ── StageRegistry tests ──────────────────────────────────────────

    #[test]
    fn test_stage_registry_new_is_empty() {
        let registry = StageRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_stage_registry_register_and_contains() {
        let mut registry = StageRegistry::new();
        registry.register(
            "extract",
            Box::new(|_spec| {
                Ok(Arc::new(MockStage {
                    name: "extract".to_string(),
                }))
            }),
        );
        assert!(registry.contains("extract"));
        assert!(!registry.contains("normalize"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_stage_registry_resolve_unknown_adapter_returns_error() {
        let registry = StageRegistry::new();
        let spec = StageSpec {
            adapter: "nonexistent".to_string(),
            source: None,
            resources: ResourceSpec::default(),
            params: serde_json::Value::Null,
            retry: None,
            timeout_secs: None,
            skip_on_error: false,
            condition: None,
        };
        let result = registry.resolve("nonexistent", "my-stage", &spec);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ResolveError::UnknownAdapter { .. }));
    }

    #[test]
    fn test_stage_registry_resolve_calls_factory() {
        let mut registry = StageRegistry::new();
        registry.register(
            "extract",
            Box::new(|spec| {
                Ok(Arc::new(MockStage {
                    name: spec.adapter.clone(),
                }))
            }),
        );
        let spec = StageSpec {
            adapter: "extract".to_string(),
            source: None,
            resources: ResourceSpec::default(),
            params: serde_json::Value::Null,
            retry: None,
            timeout_secs: None,
            skip_on_error: false,
            condition: None,
        };
        let stage = registry.resolve("extract", "my-stage", &spec).unwrap();
        assert_eq!(stage.name(), "extract");
    }

    #[test]
    fn test_stage_registry_register_overwrites() {
        let mut registry = StageRegistry::new();
        registry.register(
            "extract",
            Box::new(|_spec| {
                Ok(Arc::new(MockStage {
                    name: "v1".to_string(),
                }))
            }),
        );
        registry.register(
            "extract",
            Box::new(|_spec| {
                Ok(Arc::new(MockStage {
                    name: "v2".to_string(),
                }))
            }),
        );
        assert_eq!(registry.len(), 1);
        let spec = StageSpec {
            adapter: "extract".to_string(),
            source: None,
            resources: ResourceSpec::default(),
            params: serde_json::Value::Null,
            retry: None,
            timeout_secs: None,
            skip_on_error: false,
            condition: None,
        };
        let stage = registry.resolve("extract", "my-stage", &spec).unwrap();
        assert_eq!(
            stage.name(),
            "v2",
            "second registration should overwrite first"
        );
    }

    #[test]
    fn test_stage_registry_debug_shows_registered_adapters() {
        let mut registry = StageRegistry::new();
        registry.register(
            "extract",
            Box::new(|_spec| {
                Ok(Arc::new(MockStage {
                    name: "extract".to_string(),
                }))
            }),
        );
        let debug = format!("{registry:?}");
        assert!(
            debug.contains("extract"),
            "debug should show registered adapters"
        );
    }
}
