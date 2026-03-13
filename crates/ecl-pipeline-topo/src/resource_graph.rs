//! Resource graph: stages connected by shared resource declarations.
//! Used to compute the parallel execution schedule.

use std::collections::BTreeMap;

use ecl_pipeline_spec::StageSpec;
use ecl_pipeline_state::StageId;

use crate::error::ResolveError;
use crate::schedule;

/// The resource graph: stages connected by shared resource declarations.
///
/// This is an internal data structure — not part of the public API.
/// The public interface is `ResourceGraph::build()` and `compute_schedule()`.
///
/// Currently only used in tests and by `schedule::compute_schedule()`.
/// Full usage begins in milestone 2.3 when `resolve()` is implemented.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ResourceGraph {
    /// Which stage creates each resource. A resource may only have one creator.
    pub(crate) creators: BTreeMap<String, StageId>,
    /// Which stages read each resource.
    pub(crate) readers: BTreeMap<String, Vec<StageId>>,
    /// Which stages write (exclusively) to each resource.
    pub(crate) writers: BTreeMap<String, Vec<StageId>>,
    /// All stage IDs in the graph.
    pub(crate) stages: Vec<StageId>,
}

#[allow(dead_code)]
impl ResourceGraph {
    /// Build a resource graph from stage specifications.
    ///
    /// Iterates over all stages, collecting their resource declarations
    /// (reads, creates, writes) into the graph structure.
    pub(crate) fn build(stages: &BTreeMap<String, StageSpec>) -> Result<Self, ResolveError> {
        let mut creators: BTreeMap<String, StageId> = BTreeMap::new();
        let mut readers: BTreeMap<String, Vec<StageId>> = BTreeMap::new();
        let mut writers: BTreeMap<String, Vec<StageId>> = BTreeMap::new();
        let mut stage_ids: Vec<StageId> = Vec::new();

        for (name, spec) in stages {
            let stage_id = StageId::new(name);
            stage_ids.push(stage_id.clone());

            // Register created resources (each resource can only have one creator).
            for resource in &spec.resources.creates {
                if let Some(existing) = creators.get(resource) {
                    return Err(ResolveError::DuplicateCreator {
                        resource: resource.clone(),
                        stages: vec![existing.clone(), stage_id.clone()],
                    });
                }
                creators.insert(resource.clone(), stage_id.clone());
            }

            // Register read resources.
            for resource in &spec.resources.reads {
                readers
                    .entry(resource.clone())
                    .or_default()
                    .push(stage_id.clone());
            }

            // Register write resources.
            for resource in &spec.resources.writes {
                writers
                    .entry(resource.clone())
                    .or_default()
                    .push(stage_id.clone());
            }
        }

        Ok(Self {
            creators,
            readers,
            writers,
            stages: stage_ids,
        })
    }

    /// Validate that every resource read by a stage is either created by
    /// another stage or is an external resource (never created by anyone).
    ///
    /// A resource that appears in `reads` but NOT in `creators` is treated
    /// as external (API client, filesystem path) — always available.
    /// A resource that appears in BOTH `reads` and `creators` is internal
    /// and the scheduler will enforce ordering.
    pub(crate) fn validate_no_missing_inputs(&self) -> Result<(), ResolveError> {
        // All resources that are read are either external (not in creators)
        // or internal (in creators). Both are valid. For now, we treat
        // any resource not in `creators` as external.
        //
        // TODO: Add an explicit `externals` set so we can distinguish
        // "intentionally external" from "typo in resource name."
        Ok(())
    }

    /// Validate that the resource dependency graph contains no cycles.
    ///
    /// A cycle would mean stages have circular dependencies which makes
    /// scheduling impossible.
    pub(crate) fn validate_no_cycles(&self) -> Result<(), ResolveError> {
        // Delegates to the schedule module which performs topological sort.
        // If topo sort cannot process all nodes, a cycle exists.
        let _ = self.compute_schedule()?;
        Ok(())
    }

    /// Compute parallel batches via topological sort.
    /// Stages in the same batch touch independent resources.
    ///
    /// Algorithm: Kahn's algorithm for topological sort, then group into
    /// layers where no resource conflicts exist within a layer.
    pub(crate) fn compute_schedule(&self) -> Result<Vec<Vec<StageId>>, ResolveError> {
        schedule::compute_schedule(&self.stages, &self.creators, &self.readers, &self.writers)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::ResourceSpec;

    fn make_stage(reads: Vec<&str>, creates: Vec<&str>, writes: Vec<&str>) -> StageSpec {
        StageSpec {
            adapter: "test".to_string(),
            source: None,
            resources: ResourceSpec {
                reads: reads.into_iter().map(String::from).collect(),
                creates: creates.into_iter().map(String::from).collect(),
                writes: writes.into_iter().map(String::from).collect(),
            },
            params: serde_json::Value::Null,
            retry: None,
            timeout_secs: None,
            skip_on_error: false,
            condition: None,
        }
    }

    #[test]
    fn test_resource_graph_build_empty_stages() {
        let stages = BTreeMap::new();
        let graph = ResourceGraph::build(&stages).unwrap();
        assert!(graph.stages.is_empty());
        assert!(graph.creators.is_empty());
        assert!(graph.readers.is_empty());
        assert!(graph.writers.is_empty());
    }

    #[test]
    fn test_resource_graph_build_single_stage() {
        let mut stages = BTreeMap::new();
        stages.insert(
            "extract".to_string(),
            make_stage(vec!["api"], vec!["raw-docs"], vec![]),
        );
        let graph = ResourceGraph::build(&stages).unwrap();
        assert_eq!(graph.stages.len(), 1);
        assert_eq!(graph.creators.len(), 1);
        assert_eq!(graph.creators["raw-docs"], StageId::new("extract"));
        assert_eq!(graph.readers["api"].len(), 1);
    }

    #[test]
    fn test_resource_graph_build_duplicate_creator_fails() {
        let mut stages = BTreeMap::new();
        stages.insert("a".to_string(), make_stage(vec![], vec!["shared"], vec![]));
        stages.insert("b".to_string(), make_stage(vec![], vec!["shared"], vec![]));
        let result = ResourceGraph::build(&stages);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ResolveError::DuplicateCreator { .. }));
    }

    #[test]
    fn test_resource_graph_build_tracks_readers() {
        let mut stages = BTreeMap::new();
        stages.insert("a".to_string(), make_stage(vec!["shared"], vec![], vec![]));
        stages.insert("b".to_string(), make_stage(vec!["shared"], vec![], vec![]));
        let graph = ResourceGraph::build(&stages).unwrap();
        assert_eq!(graph.readers["shared"].len(), 2);
    }

    #[test]
    fn test_resource_graph_build_tracks_writers() {
        let mut stages = BTreeMap::new();
        stages.insert("a".to_string(), make_stage(vec![], vec![], vec!["output"]));
        stages.insert("b".to_string(), make_stage(vec![], vec![], vec!["output"]));
        let graph = ResourceGraph::build(&stages).unwrap();
        assert_eq!(graph.writers["output"].len(), 2);
    }

    #[test]
    fn test_resource_graph_validate_no_missing_inputs_passes() {
        let mut stages = BTreeMap::new();
        stages.insert(
            "extract".to_string(),
            make_stage(vec!["external-api"], vec!["raw-docs"], vec![]),
        );
        stages.insert(
            "normalize".to_string(),
            make_stage(vec!["raw-docs"], vec!["normalized"], vec![]),
        );
        let graph = ResourceGraph::build(&stages).unwrap();
        assert!(graph.validate_no_missing_inputs().is_ok());
    }
}
