//! Property-based tests for core types.

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::types::{StepId, WorkflowId};
    use proptest::prelude::*;
    use uuid::Uuid;

    proptest! {
        #[test]
        fn test_workflow_id_roundtrip(uuid in any::<u128>()) {
            let uuid = Uuid::from_u128(uuid);
            let id = WorkflowId::from_uuid(uuid);
            assert_eq!(id.into_uuid(), uuid);
        }

        #[test]
        fn test_step_id_roundtrip(s in "\\PC+") {
            let id = StepId::new(s.clone());
            assert_eq!(id.as_str(), &s);
        }

        #[test]
        fn test_workflow_id_display_parse_roundtrip(uuid in any::<u128>()) {
            let uuid = Uuid::from_u128(uuid);
            let id = WorkflowId::from_uuid(uuid);
            let string = id.to_string();
            let parsed: WorkflowId = string.parse().unwrap();
            assert_eq!(id, parsed);
        }
    }
}
