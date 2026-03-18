//! Default adapter and stage registries for the CLI.
//!
//! Registers all built-in adapters (filesystem, Google Drive) and stages
//! (extract, normalize, filter, emit) so that TOML configs can reference
//! them by name.

use std::collections::BTreeMap;
use std::sync::Arc;

use ecl_adapter_fs::FilesystemAdapter;
use ecl_adapter_gcs::GcsAdapter;
use ecl_adapter_gdrive::GoogleDriveAdapter;
use ecl_adapter_slack::SlackAdapter;
use ecl_adapter_zapier::ZapierAdapter;
use ecl_pipeline_spec::{PipelineSpec, SourceSpec, StageSpec};
use ecl_pipeline_topo::error::ResolveError;
use ecl_pipeline_topo::{PushSourceAdapter, SourceAdapter, Stage};
use ecl_stages::{CsvParseStage, EmitStage, ExtractStage, FieldMapStage, FilterStage, NormalizeStage, ValidateStage};

/// Pre-resolve all source adapters from the spec.
///
/// Returns a map of source_name -> concrete adapter.
///
/// # Errors
///
/// Returns `ResolveError` if a source kind is unknown or adapter creation fails.
pub fn resolve_adapters(
    spec: &PipelineSpec,
) -> Result<BTreeMap<String, Arc<dyn SourceAdapter>>, ResolveError> {
    let mut adapters = BTreeMap::new();

    for (name, source_spec) in &spec.sources {
        let adapter: Arc<dyn SourceAdapter> = match source_spec {
            SourceSpec::Filesystem(_) => Arc::new(FilesystemAdapter::from_spec(name, source_spec)?),
            SourceSpec::GoogleDrive(_) => {
                Arc::new(GoogleDriveAdapter::from_spec(name, source_spec)?)
            }
            SourceSpec::Slack(_) => Arc::new(SlackAdapter::from_spec(name, source_spec)?),
            SourceSpec::Zapier(_) => continue, // Push sources resolved separately
            SourceSpec::Gcs(_) => Arc::new(GcsAdapter::from_spec(name, source_spec)?),
        };
        adapters.insert(name.clone(), adapter);
    }

    Ok(adapters)
}

/// Create an adapter lookup closure from pre-resolved adapters.
///
/// The closure returns clones of the pre-resolved adapters.
pub fn adapter_lookup_fn(
    adapters: &BTreeMap<String, Arc<dyn SourceAdapter>>,
) -> impl Fn(&str, &SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError> + '_ {
    move |name: &str, _spec: &SourceSpec| {
        adapters
            .get(name)
            .cloned()
            .ok_or_else(|| ResolveError::UnknownAdapter {
                stage: name.to_string(),
                adapter: "unknown".to_string(),
            })
    }
}

/// Create a stage lookup closure that uses pre-resolved adapters for extract stages.
pub fn stage_lookup_fn(
    adapters: &BTreeMap<String, Arc<dyn SourceAdapter>>,
) -> impl Fn(&str, &StageSpec) -> Result<Arc<dyn Stage>, ResolveError> + '_ {
    move |name: &str, spec: &StageSpec| -> Result<Arc<dyn Stage>, ResolveError> {
        match spec.adapter.as_str() {
            "extract" => {
                let source_name = spec.source.as_deref().ok_or_else(|| {
                    ResolveError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("extract stage '{name}' has no 'source' field"),
                    ))
                })?;
                let adapter = adapters.get(source_name).cloned().ok_or_else(|| {
                    ResolveError::UnknownAdapter {
                        stage: name.to_string(),
                        adapter: format!("source '{source_name}' not found"),
                    }
                })?;
                Ok(Arc::new(ExtractStage::new(adapter, source_name)))
            }
            "normalize" => Ok(Arc::new(NormalizeStage::new())),
            "filter" => {
                let stage = FilterStage::from_params(&spec.params).map_err(|e| {
                    ResolveError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("filter stage '{name}': {e}"),
                    ))
                })?;
                Ok(Arc::new(stage))
            }
            "csv_parse" => {
                let stage = CsvParseStage::from_params(&spec.params).map_err(|e| {
                    ResolveError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("csv_parse stage '{name}': {e}"),
                    ))
                })?;
                Ok(Arc::new(stage))
            }
            "field_map" => {
                let stage = FieldMapStage::from_params(&spec.params).map_err(|e| {
                    ResolveError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("field_map stage '{name}': {e}"),
                    ))
                })?;
                Ok(Arc::new(stage))
            }
            "validate" => {
                let stage = ValidateStage::from_params(&spec.params).map_err(|e| {
                    ResolveError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("validate stage '{name}': {e}"),
                    ))
                })?;
                Ok(Arc::new(stage))
            }
            "emit" => Ok(Arc::new(EmitStage::new())),
            other => Err(ResolveError::UnknownAdapter {
                stage: name.to_string(),
                adapter: other.to_string(),
            }),
        }
    }
}

/// Pre-resolve all push-based source adapters from the spec.
///
/// Returns a map of source_name -> concrete push adapter.
/// Currently only Zapier sources are push-based.
///
/// # Errors
///
/// Returns `ResolveError` if adapter creation fails.
pub fn resolve_push_adapters(
    spec: &PipelineSpec,
) -> Result<BTreeMap<String, Arc<dyn PushSourceAdapter>>, ResolveError> {
    let mut adapters = BTreeMap::new();

    for (name, source_spec) in &spec.sources {
        if let SourceSpec::Zapier(_) = source_spec {
            let adapter = ZapierAdapter::from_spec(name, source_spec)?;
            adapters.insert(
                name.clone(),
                Arc::new(adapter) as Arc<dyn PushSourceAdapter>,
            );
        }
    }

    Ok(adapters)
}
