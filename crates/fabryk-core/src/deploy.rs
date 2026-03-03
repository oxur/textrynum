//! Cloud deployment detection utilities.
//!
//! Google Cloud Run's Knative runtime sets the `K_SERVICE` environment variable
//! on every container. These helpers detect the runtime environment and extract
//! deployment metadata.

/// Returns `true` when running on Google Cloud Run (i.e. `K_SERVICE` is set).
pub fn is_cloud_run() -> bool {
    std::env::var("K_SERVICE").is_ok()
}

/// Returns the Cloud Run service name, if running on Cloud Run.
pub fn service_name() -> Option<String> {
    std::env::var("K_SERVICE").ok()
}

/// Returns the Cloud Run revision name, if running on Cloud Run.
pub fn revision_name() -> Option<String> {
    std::env::var("K_REVISION").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cloud_run_returns_false_when_k_service_absent() {
        if std::env::var("K_SERVICE").is_err() {
            assert!(!is_cloud_run());
        }
    }

    #[test]
    fn test_service_name_returns_none_when_k_service_absent() {
        if std::env::var("K_SERVICE").is_err() {
            assert!(service_name().is_none());
        }
    }

    #[test]
    fn test_revision_name_returns_none_when_k_revision_absent() {
        if std::env::var("K_REVISION").is_err() {
            assert!(revision_name().is_none());
        }
    }
}
