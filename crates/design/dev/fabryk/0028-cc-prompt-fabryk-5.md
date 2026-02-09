---
title: "CC Prompt: Fabryk 5.3 — Music Theory Content Providers"
milestone: "5.3"
phase: 5
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["5.2 Content traits complete"]
governing-docs: [0012-amendment §2a, 0013-project-plan]
---

# CC Prompt: Fabryk 5.3 — Music Theory Content Providers

## Context

This milestone creates the **domain-specific** implementations of
`ContentItemProvider` and `SourceProvider` for music theory. These
implementations **stay in ai-music-theory** — they are not part of Fabryk.

The implementations know about:
- Music theory concept cards and their structure
- Music theory sources (Tymoczko, Lewin, etc.)
- Chapter/section organization of music theory materials

## Objective

Create music-theory implementations in ai-music-theory:

1. `MusicTheoryContentProvider` implementing `ContentItemProvider`
2. `MusicTheorySourceProvider` implementing `SourceProvider`
3. Wire into the tool registration system

## Implementation Steps

### Step 1: Create MusicTheoryContentProvider

Create `crates/music-theory/mcp-server/src/music_theory_content.rs`:

```rust
//! Music theory implementation of ContentItemProvider.

use async_trait::async_trait;
use fabryk_core::Result;
use fabryk_mcp_content::{CategoryInfo, ContentItemProvider};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Config;

/// Summary information for a music theory concept.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConceptInfo {
    /// Concept ID.
    pub id: String,
    /// Concept title.
    pub title: String,
    /// Category (harmony, rhythm, form, etc.).
    pub category: Option<String>,
    /// Source reference (if source-specific variant).
    pub source: Option<String>,
    /// Brief description.
    pub description: Option<String>,
}

/// Full concept card detail.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConceptDetail {
    /// Concept ID.
    pub id: String,
    /// Concept title.
    pub title: String,
    /// Category.
    pub category: Option<String>,
    /// Source reference.
    pub source: Option<String>,
    /// Full description.
    pub description: Option<String>,
    /// Markdown content.
    pub content: String,
    /// Prerequisites.
    pub prerequisites: Vec<String>,
    /// Related concepts.
    pub related: Vec<String>,
    /// Tags.
    pub tags: Vec<String>,
    /// File path.
    pub path: PathBuf,
}

/// Content provider for music theory concepts.
pub struct MusicTheoryContentProvider {
    config: Arc<Config>,
    // Could cache concept list here
}

impl MusicTheoryContentProvider {
    /// Create a new provider.
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    /// Load concept info from a file.
    async fn load_concept_info(&self, path: &PathBuf) -> Result<ConceptInfo> {
        let content = tokio::fs::read_to_string(path).await?;
        let fm = fabryk_content::markdown::extract_frontmatter(&content)?;

        let id = fabryk_core::util::ids::id_from_path(
            &self.config.content_path("concepts")?,
            path,
        )?;

        Ok(ConceptInfo {
            id,
            title: fm.frontmatter
                .get("concept")
                .or_else(|| fm.frontmatter.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string(),
            category: fm.frontmatter
                .get("category")
                .and_then(|v| v.as_str())
                .map(String::from),
            source: fm.frontmatter
                .get("source")
                .and_then(|v| v.as_str())
                .map(String::from),
            description: fm.frontmatter
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
        })
    }

    /// Load full concept detail from a file.
    async fn load_concept_detail(&self, path: &PathBuf) -> Result<ConceptDetail> {
        let content = tokio::fs::read_to_string(path).await?;
        let fm = fabryk_content::markdown::extract_frontmatter(&content)?;

        let id = fabryk_core::util::ids::id_from_path(
            &self.config.content_path("concepts")?,
            path,
        )?;

        let parse_list = |key: &str| -> Vec<String> {
            fm.frontmatter
                .get(key)
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default()
        };

        Ok(ConceptDetail {
            id,
            title: fm.frontmatter
                .get("concept")
                .or_else(|| fm.frontmatter.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string(),
            category: fm.frontmatter
                .get("category")
                .and_then(|v| v.as_str())
                .map(String::from),
            source: fm.frontmatter
                .get("source")
                .and_then(|v| v.as_str())
                .map(String::from),
            description: fm.frontmatter
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            content: fm.content.to_string(),
            prerequisites: parse_list("prerequisites"),
            related: parse_list("see_also"),
            tags: parse_list("tags"),
            path: path.clone(),
        })
    }
}

#[async_trait]
impl ContentItemProvider for MusicTheoryContentProvider {
    type ItemSummary = ConceptInfo;
    type ItemDetail = ConceptDetail;

    async fn list_items(
        &self,
        category: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Self::ItemSummary>> {
        let concepts_path = self.config.content_path("concepts")?;
        let options = fabryk_core::util::files::FindOptions {
            pattern: Some("**/*.md".to_string()),
            ..Default::default()
        };

        let files = fabryk_core::util::files::find_all_files(&concepts_path, options).await?;

        let mut results = Vec::new();
        for file in files {
            if let Ok(info) = self.load_concept_info(&file.path).await {
                // Apply category filter
                if let Some(cat) = category {
                    if info.category.as_deref() != Some(cat) {
                        continue;
                    }
                }
                results.push(info);

                // Apply limit
                if let Some(max) = limit {
                    if results.len() >= max {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    async fn get_item(&self, id: &str) -> Result<Self::ItemDetail> {
        let concepts_path = self.config.content_path("concepts")?;

        // Try to find the file by ID
        let options = fabryk_core::util::files::FindOptions {
            pattern: Some("**/*.md".to_string()),
            ..Default::default()
        };

        let files = fabryk_core::util::files::find_all_files(&concepts_path, options).await?;

        for file in files {
            let file_id = fabryk_core::util::ids::id_from_path(&concepts_path, &file.path)?;
            if file_id == id {
                return self.load_concept_detail(&file.path).await;
            }
        }

        Err(fabryk_core::Error::not_found("concept", id))
    }

    async fn list_categories(&self) -> Result<Vec<CategoryInfo>> {
        let items = self.list_items(None, None).await?;

        let mut category_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for item in items {
            let cat = item.category.unwrap_or_else(|| "uncategorized".to_string());
            *category_counts.entry(cat).or_insert(0) += 1;
        }

        let mut categories: Vec<CategoryInfo> = category_counts
            .into_iter()
            .map(|(id, count)| CategoryInfo {
                id: id.clone(),
                name: title_case(&id),
                count,
                description: None,
            })
            .collect();

        categories.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(categories)
    }

    fn content_type_name(&self) -> &str {
        "concept"
    }

    fn content_type_name_plural(&self) -> &str {
        "concepts"
    }
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}
```

### Step 2: Create MusicTheorySourceProvider

Create `crates/music-theory/mcp-server/src/music_theory_sources.rs`:

```rust
//! Music theory implementation of SourceProvider.

use async_trait::async_trait;
use fabryk_core::Result;
use fabryk_mcp_content::{ChapterInfo, SourceProvider};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Config;

/// Summary information for a music theory source.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Source ID.
    pub id: String,
    /// Source title.
    pub title: String,
    /// Author(s).
    pub author: String,
    /// Publication year.
    pub year: Option<u16>,
    /// Whether full text is available.
    pub available: bool,
    /// Chapter count (if known).
    pub chapter_count: Option<usize>,
}

/// Music theory source provider.
pub struct MusicTheorySourceProvider {
    config: Arc<Config>,
}

impl MusicTheorySourceProvider {
    /// Create a new provider.
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    /// Get the path to source materials.
    fn sources_path(&self) -> Result<PathBuf> {
        self.config.content_path("sources")
    }
}

#[async_trait]
impl SourceProvider for MusicTheorySourceProvider {
    type SourceSummary = SourceInfo;

    async fn list_sources(&self) -> Result<Vec<Self::SourceSummary>> {
        // In a real implementation, this would read from a sources index
        // or scan the sources directory

        // For now, return known music theory sources
        Ok(vec![
            SourceInfo {
                id: "tymoczko".to_string(),
                title: "A Geometry of Music".to_string(),
                author: "Dmitri Tymoczko".to_string(),
                year: Some(2011),
                available: true,
                chapter_count: Some(10),
            },
            SourceInfo {
                id: "lewin".to_string(),
                title: "Generalized Musical Intervals and Transformations".to_string(),
                author: "David Lewin".to_string(),
                year: Some(1987),
                available: true,
                chapter_count: Some(8),
            },
            SourceInfo {
                id: "cohn".to_string(),
                title: "Audacious Euphony".to_string(),
                author: "Richard Cohn".to_string(),
                year: Some(2012),
                available: true,
                chapter_count: Some(6),
            },
        ])
    }

    async fn get_chapter(
        &self,
        source_id: &str,
        chapter: &str,
        section: Option<&str>,
    ) -> Result<String> {
        let sources_path = self.sources_path()?;
        let mut chapter_path = sources_path.join(source_id).join(format!("{}.md", chapter));

        if !chapter_path.exists() {
            // Try with chapter- prefix
            chapter_path = sources_path
                .join(source_id)
                .join(format!("chapter-{}.md", chapter));
        }

        if !chapter_path.exists() {
            return Err(fabryk_core::Error::not_found(
                "chapter",
                &format!("{}:{}", source_id, chapter),
            ));
        }

        let content = tokio::fs::read_to_string(&chapter_path).await?;

        // If section specified, extract just that section
        if let Some(section_name) = section {
            if let Some(section_content) =
                fabryk_content::markdown::extract_section(&content, section_name)
            {
                return Ok(section_content);
            }
        }

        Ok(content)
    }

    async fn list_chapters(&self, source_id: &str) -> Result<Vec<ChapterInfo>> {
        let sources_path = self.sources_path()?;
        let source_path = sources_path.join(source_id);

        if !source_path.exists() {
            return Err(fabryk_core::Error::not_found("source", source_id));
        }

        let options = fabryk_core::util::files::FindOptions {
            pattern: Some("*.md".to_string()),
            ..Default::default()
        };

        let files = fabryk_core::util::files::find_all_files(&source_path, options).await?;

        let mut chapters: Vec<ChapterInfo> = files
            .iter()
            .filter_map(|f| {
                let stem = f.path.file_stem()?.to_str()?;
                let number = stem.strip_prefix("chapter-").map(String::from);

                Some(ChapterInfo {
                    id: stem.to_string(),
                    title: title_case(stem.replace('-', " ").trim()),
                    number,
                    available: true,
                })
            })
            .collect();

        chapters.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(chapters)
    }

    async fn get_source_path(&self, source_id: &str) -> Result<Option<PathBuf>> {
        let sources_path = self.sources_path()?;
        let pdf_path = sources_path.join(format!("{}.pdf", source_id));

        if pdf_path.exists() {
            Ok(Some(pdf_path))
        } else {
            Ok(None)
        }
    }
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
```

### Step 3: Wire into tool registration

Update `crates/music-theory/mcp-server/src/lib.rs`:

```rust
pub mod music_theory_content;
pub mod music_theory_sources;

pub use music_theory_content::{MusicTheoryContentProvider, ConceptInfo, ConceptDetail};
pub use music_theory_sources::{MusicTheorySourceProvider, SourceInfo};
```

### Step 4: Update tool registry

In the music-theory server setup, register the content tools:

```rust
use fabryk_mcp::{CompositeRegistry};
use fabryk_mcp_content::{ContentTools, SourceTools};

// Create content providers
let content_provider = MusicTheoryContentProvider::new(Arc::clone(&config));
let source_provider = MusicTheorySourceProvider::new(Arc::clone(&config));

// Create tool registries
let content_tools = ContentTools::new(content_provider).with_prefix("concepts");
let source_tools = SourceTools::new(source_provider);

// Combine into composite registry
let registry = CompositeRegistry::new()
    .add(content_tools)
    .add(source_tools)
    // ... add other tools
    ;
```

### Step 5: Verify compilation and test

```bash
cd ~/lab/oxur/ecl
cargo check -p music-theory-mcp-server
cargo test -p music-theory-mcp-server
```

## Exit Criteria

- [ ] `MusicTheoryContentProvider` implements `ContentItemProvider`
- [ ] `ConceptInfo` and `ConceptDetail` types defined
- [ ] `MusicTheorySourceProvider` implements `SourceProvider`
- [ ] `SourceInfo` type defined
- [ ] Category filtering works correctly
- [ ] Chapter retrieval works for music theory sources
- [ ] Tools registered via CompositeRegistry
- [ ] All tests pass

## Commit Message

```
feat(music-theory): add ContentItemProvider and SourceProvider implementations

Implement Amendment §2a content provider traits for music theory:
- MusicTheoryContentProvider: concepts list/get/categories
- MusicTheorySourceProvider: sources list/chapters/content
- ConceptInfo/ConceptDetail for concept summaries and full cards
- SourceInfo for source material metadata

Wire into MCP tool registration via CompositeRegistry.

Phase 5 milestone 5.3 of Fabryk extraction.

Ref: Doc 0012 §2a (content provider traits)
Ref: Doc 0013 Phase 5

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
