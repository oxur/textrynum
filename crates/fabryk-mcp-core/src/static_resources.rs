//! Static resource registry for MCP servers.
//!
//! Provides [`StaticResources`], a configurable [`ResourceRegistry`] that
//! serves files from a directory, mapping URIs to filenames with optional
//! fallback content.

use std::path::PathBuf;

use rmcp::model::{AnnotateAble, ErrorCode, ErrorData, RawResource, Resource, ResourceContents};

use crate::resource::{ResourceFuture, ResourceRegistry};

/// Definition of a single static resource.
#[derive(Clone, Debug)]
pub struct StaticResourceDef {
    /// MCP resource URI (e.g., `"skill://conventions"`).
    pub uri: String,
    /// Human-readable resource name.
    pub name: String,
    /// Resource description.
    pub description: String,
    /// MIME type (typically `"text/markdown"`).
    pub mime_type: String,
    /// Filename relative to the base path.
    pub filename: String,
    /// Optional fallback content if the file doesn't exist on disk.
    pub fallback: Option<String>,
}

/// A [`ResourceRegistry`] that serves static files from a directory.
///
/// Maps URIs to files in `base_path`. If a file doesn't exist and a
/// fallback is provided in the resource definition, the fallback content
/// is returned. If there's no fallback either, returns a "not found" error.
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_mcp_core::{StaticResources, StaticResourceDef};
///
/// let resources = StaticResources::new("/path/to/docs")
///     .with_resource(StaticResourceDef {
///         uri: "skill://conventions".into(),
///         name: "Conventions".into(),
///         description: "Domain conventions".into(),
///         mime_type: "text/markdown".into(),
///         filename: "CONVENTIONS.md".into(),
///         fallback: Some("# Default Conventions\n...".into()),
///     });
/// ```
pub struct StaticResources {
    base_path: PathBuf,
    resources: Vec<StaticResourceDef>,
}

impl StaticResources {
    /// Create a new static resources registry with a base directory path.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            resources: Vec::new(),
        }
    }

    /// Add a resource definition. Returns self for chaining.
    pub fn with_resource(mut self, def: StaticResourceDef) -> Self {
        self.resources.push(def);
        self
    }
}

impl ResourceRegistry for StaticResources {
    fn resources(&self) -> Vec<Resource> {
        self.resources
            .iter()
            .map(|def| {
                RawResource {
                    uri: def.uri.clone(),
                    name: def.name.clone(),
                    title: None,
                    description: Some(def.description.clone()),
                    mime_type: Some(def.mime_type.clone()),
                    size: None,
                    icons: None,
                    meta: None,
                }
                .no_annotation()
            })
            .collect()
    }

    fn read(&self, uri: &str) -> Option<ResourceFuture> {
        let def = self.resources.iter().find(|d| d.uri == uri)?;

        let path = self.base_path.join(&def.filename);
        let fallback = def.fallback.clone();
        let uri_owned = uri.to_string();

        Some(Box::pin(async move {
            // Try reading from disk first
            match std::fs::read_to_string(&path) {
                Ok(content) => Ok(vec![ResourceContents::text(content, uri_owned)]),
                Err(_) => {
                    // Fall back to default content if available
                    if let Some(content) = fallback {
                        Ok(vec![ResourceContents::text(content, uri_owned)])
                    } else {
                        Err(ErrorData::new(
                            ErrorCode::RESOURCE_NOT_FOUND,
                            format!("Resource not found: {uri_owned}"),
                            None,
                        ))
                    }
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_def(uri: &str, filename: &str, fallback: Option<&str>) -> StaticResourceDef {
        StaticResourceDef {
            uri: uri.to_string(),
            name: format!("Test {uri}"),
            description: format!("Description for {uri}"),
            mime_type: "text/markdown".to_string(),
            filename: filename.to_string(),
            fallback: fallback.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_static_resources_empty() {
        let sr = StaticResources::new("/tmp/nonexistent");
        assert!(sr.resources().is_empty());
        assert!(sr.read("skill://anything").is_none());
    }

    #[test]
    fn test_static_resources_list() {
        let sr = StaticResources::new("/tmp")
            .with_resource(sample_def("skill://a", "a.md", None))
            .with_resource(sample_def("skill://b", "b.md", Some("fallback")));

        let listed = sr.resources();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].raw.uri, "skill://a");
        assert_eq!(listed[0].raw.name, "Test skill://a");
        assert_eq!(
            listed[0].raw.description.as_deref(),
            Some("Description for skill://a")
        );
        assert_eq!(listed[0].raw.mime_type.as_deref(), Some("text/markdown"));
        assert_eq!(listed[1].raw.uri, "skill://b");
    }

    #[tokio::test]
    async fn test_static_resources_read_file() {
        let dir = tempfile::tempdir().unwrap();
        let content = "# Hello\n\nThis is test content.";
        std::fs::write(dir.path().join("hello.md"), content).unwrap();

        let sr = StaticResources::new(dir.path()).with_resource(sample_def(
            "skill://hello",
            "hello.md",
            Some("fallback"),
        ));

        let result = sr.read("skill://hello").unwrap().await.unwrap();
        assert_eq!(result.len(), 1);
        // File content should take precedence over fallback
        match &result[0] {
            ResourceContents::TextResourceContents { text, uri, .. } => {
                assert_eq!(text, content);
                assert_eq!(uri, "skill://hello");
            }
            _ => panic!("Expected TextResourceContents"),
        }
    }

    #[tokio::test]
    async fn test_static_resources_read_fallback() {
        let dir = tempfile::tempdir().unwrap();
        // No file written to disk
        let fallback = "# Fallback Content";
        let sr = StaticResources::new(dir.path()).with_resource(sample_def(
            "skill://missing",
            "missing.md",
            Some(fallback),
        ));

        let result = sr.read("skill://missing").unwrap().await.unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            ResourceContents::TextResourceContents { text, .. } => {
                assert_eq!(text, fallback);
            }
            _ => panic!("Expected TextResourceContents"),
        }
    }

    #[tokio::test]
    async fn test_static_resources_read_no_fallback_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let sr = StaticResources::new(dir.path()).with_resource(sample_def(
            "skill://gone",
            "gone.md",
            None,
        ));

        let result = sr.read("skill://gone").unwrap().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::RESOURCE_NOT_FOUND);
        assert!(err.message.contains("skill://gone"));
    }

    #[test]
    fn test_static_resources_read_unknown_uri() {
        let sr = StaticResources::new("/tmp").with_resource(sample_def(
            "skill://known",
            "known.md",
            None,
        ));
        assert!(sr.read("skill://unknown").is_none());
    }

    #[test]
    fn test_static_resource_def_clone() {
        let def = sample_def("skill://clone", "clone.md", Some("fallback"));
        let cloned = def.clone();
        assert_eq!(cloned.uri, def.uri);
        assert_eq!(cloned.name, def.name);
        assert_eq!(cloned.fallback, def.fallback);
    }

    #[test]
    fn test_static_resources_chained_add() {
        let sr = StaticResources::new("/tmp")
            .with_resource(sample_def("skill://a", "a.md", None))
            .with_resource(sample_def("skill://b", "b.md", None))
            .with_resource(sample_def("skill://c", "c.md", None));

        assert_eq!(sr.resources().len(), 3);
    }
}
