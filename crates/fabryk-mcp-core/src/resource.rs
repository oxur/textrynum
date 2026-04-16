//! Resource registry trait for MCP servers.
//!
//! The [`ResourceRegistry`] trait abstracts over resource listing and reading,
//! allowing domains to expose subscribable MCP resources.

use rmcp::model::{ErrorData, Resource, ResourceContents};
use std::future::Future;
use std::pin::Pin;

/// Type alias for async resource read results.
pub type ResourceFuture =
    Pin<Box<dyn Future<Output = Result<Vec<ResourceContents>, ErrorData>> + Send>>;

/// Trait for registering and reading MCP resources.
///
/// Implement this to expose domain-specific resources that clients can
/// list, read, and subscribe to for live updates.
///
/// # Example
///
/// ```rust,ignore
/// struct MyResources { /* ... */ }
///
/// impl ResourceRegistry for MyResources {
///     fn resources(&self) -> Vec<Resource> {
///         vec![/* resource definitions */]
///     }
///
///     fn read(&self, uri: &str) -> Option<ResourceFuture> {
///         match uri {
///             "my://resource" => Some(Box::pin(async { /* ... */ })),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait ResourceRegistry: Send + Sync {
    /// Returns all available resources.
    fn resources(&self) -> Vec<Resource>;

    /// Read a resource by URI.
    ///
    /// Returns `None` if the URI is not recognized.
    fn read(&self, uri: &str) -> Option<ResourceFuture>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify `ResourceRegistry` is object-safe.
    #[test]
    fn test_trait_object_safety() {
        fn _assert_object_safe(_: &dyn ResourceRegistry) {}
    }

    struct TestResources;

    impl ResourceRegistry for TestResources {
        fn resources(&self) -> Vec<Resource> {
            use rmcp::model::{Annotated, RawResource};
            vec![Annotated::new(
                RawResource::new("test://doc", "Test Document"),
                None,
            )]
        }

        fn read(&self, uri: &str) -> Option<ResourceFuture> {
            if uri == "test://doc" {
                Some(Box::pin(async {
                    Ok(vec![ResourceContents::text("test://doc", "hello")])
                }))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_resources_returns_items() {
        let registry = TestResources;
        let resources = registry.resources();
        assert_eq!(resources.len(), 1);
    }

    #[tokio::test]
    async fn test_read_known_uri() {
        let registry = TestResources;
        let future = registry.read("test://doc");
        assert!(future.is_some());
        let contents = future.unwrap().await.unwrap();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_read_unknown_uri_returns_none() {
        let registry = TestResources;
        assert!(registry.read("test://unknown").is_none());
    }
}
