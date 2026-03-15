//! Validation logic for pipeline specifications.

use crate::PipelineSpec;
use crate::error::{Result, SpecError};

/// Validate a pipeline specification.
///
/// Checks:
/// - Pipeline has at least one source
/// - Pipeline has at least one stage
/// - Every stage with a `source` field references an existing source
pub fn validate(spec: &PipelineSpec) -> Result<()> {
    if spec.sources.is_empty() {
        return Err(SpecError::EmptySources);
    }

    if spec.stages.is_empty() {
        return Err(SpecError::EmptyPipeline);
    }

    // Validate stage source references.
    for (stage_name, stage_spec) in &spec.stages {
        if let Some(ref source_name) = stage_spec.source
            && !spec.sources.contains_key(source_name)
        {
            return Err(SpecError::UnknownSource {
                stage: stage_name.clone(),
                source_name: source_name.clone(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::source::{FilesystemSourceSpec, SourceSpec};
    use crate::stage::{ResourceSpec, StageSpec};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// Helper to build a minimal valid spec for testing.
    fn minimal_spec() -> PipelineSpec {
        let mut sources = BTreeMap::new();
        sources.insert(
            "local".to_string(),
            SourceSpec::Filesystem(FilesystemSourceSpec {
                root: PathBuf::from("/tmp/data"),
                filters: vec![],
                extensions: vec![],
            }),
        );

        let mut stages = BTreeMap::new();
        stages.insert(
            "extract".to_string(),
            StageSpec {
                adapter: "extract".to_string(),
                source: Some("local".to_string()),
                resources: ResourceSpec::default(),
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );

        PipelineSpec {
            name: "test".to_string(),
            version: 1,
            output_dir: PathBuf::from("./output"),
            sources,
            stages,
            defaults: Default::default(),
        }
    }

    #[test]
    fn test_validate_empty_sources_fails() {
        let mut spec = minimal_spec();
        spec.sources.clear();
        let err = validate(&spec).unwrap_err();
        assert!(matches!(err, SpecError::EmptySources));
    }

    #[test]
    fn test_validate_empty_stages_fails() {
        let mut spec = minimal_spec();
        spec.stages.clear();
        let err = validate(&spec).unwrap_err();
        assert!(matches!(err, SpecError::EmptyPipeline));
    }

    #[test]
    fn test_validate_unknown_source_reference_fails() {
        let mut spec = minimal_spec();
        spec.stages.insert(
            "bad-stage".to_string(),
            StageSpec {
                adapter: "extract".to_string(),
                source: Some("nonexistent-source".to_string()),
                resources: ResourceSpec::default(),
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );
        let err = validate(&spec).unwrap_err();
        assert!(matches!(err, SpecError::UnknownSource { .. }));
    }

    #[test]
    fn test_validate_valid_spec_passes() {
        let spec = minimal_spec();
        assert!(validate(&spec).is_ok());
    }
}
