//! Topology resolution: converting a PipelineSpec into a PipelineTopology.
//!
//! **STUB:** This module contains only the function signature in this
//! milestone. The full implementation (resolving adapters, building the
//! resource graph, computing the schedule) is implemented in milestone 2.3.

use ecl_pipeline_spec::PipelineSpec;

use crate::PipelineTopology;
use crate::error::ResolveError;

/// Resolve a `PipelineSpec` into a `PipelineTopology`.
///
/// This is the main entry point for topology construction:
/// 1. Hash the spec for config drift detection.
/// 2. Resolve each source into a concrete `SourceAdapter`.
/// 3. Resolve each stage into a concrete `Stage` handler.
/// 4. Build the resource graph and validate (no missing inputs, no cycles).
/// 5. Compute the parallel schedule.
/// 6. Create the output directory.
///
/// # Errors
///
/// Returns `ResolveError` if any step fails (unknown adapter, cycle, I/O, etc.).
pub async fn resolve(_spec: PipelineSpec) -> Result<PipelineTopology, ResolveError> {
    // Full implementation deferred to milestone 2.3.
    // At that point this function will:
    // - Serialize and hash the spec
    // - Call resolve_source_adapter() for each source
    // - Call resolve_stage() for each stage
    // - Build ResourceGraph and validate
    // - Compute schedule
    // - Create output directory
    todo!("Full topology resolution implemented in milestone 2.3")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    #[should_panic(expected = "milestone 2.3")]
    async fn test_resolve_stub_is_todo() {
        let spec = PipelineSpec::from_toml(
            r#"
name = "test"
version = 1
output_dir = "./out"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["docs"] }
"#,
        )
        .unwrap();
        let _ = resolve(spec).await;
    }
}
