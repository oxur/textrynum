---
number: 7
title: "Fabryk File Management: Addendum"
author: "relevance
LIMIT"
component: All
tags: [change-me]
created: 2026-01-23
updated: 2026-01-23
state: Under Review
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk File Management: Addendum

## Storage Architecture for Knowledge Items

**Version**: 1.0
**Date**: January 2026
**Status**: Proposal Addendum
**Parent Document**: 0005-fabryk-proposal.md

---

## Purpose

This addendum explores storage architecture options for Fabryk, specifically addressing:

1. Full-text search implementation in Rust
2. File-based vs. database-based content storage
3. Managing markdown files with database metadata
4. Recommended architecture for v1

---

## Full-Text Search Options

### Option 1: SQLite FTS5

**What it is**: SQLite's built-in full-text search extension, available since SQLite 3.9.0.

**Implementation**:

```sql
-- Create FTS5 virtual table
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    title,
    summary,
    content,
    tokenize='porter unicode61'  -- Stemming + Unicode support
);

-- Sync trigger (insert)
CREATE TRIGGER knowledge_fts_insert AFTER INSERT ON knowledge_items BEGIN
    INSERT INTO knowledge_fts(rowid, title, summary, content)
    VALUES (NEW.rowid, NEW.title, NEW.summary, NEW.content);
END;

-- Search with BM25 ranking
SELECT k.*, bm25(knowledge_fts) as relevance
FROM knowledge_items k
JOIN knowledge_fts fts ON k.rowid = fts.rowid
WHERE knowledge_fts MATCH 'architecture patterns'
ORDER BY relevance
LIMIT 20;
```

**Capabilities**:

- Boolean queries: `AND`, `OR`, `NOT`
- Phrase search: `"exact phrase"`
- Prefix search: `arch*`
- Column filtering: `title:architecture`
- Proximity search: `NEAR(word1 word2, 5)`
- BM25 relevance ranking

**Rust Integration**:

```rust
// Via SQLx - just write SQL
let results = sqlx::query_as::<_, KnowledgeItem>(
    r#"
    SELECT k.*, bm25(knowledge_fts) as relevance
    FROM knowledge_items k
    JOIN knowledge_fts fts ON k.rowid = fts.rowid
    WHERE knowledge_fts MATCH $1
    ORDER BY relevance
    LIMIT $2
    "#
)
.bind(&query)
.bind(limit)
.fetch_all(&pool)
.await?;
```

| Aspect | Assessment |
|--------|------------|
| **Maturity** | Excellent — part of SQLite core |
| **Performance** | Good for <1M documents |
| **Rust Support** | Via SQLx/rusqlite (SQL strings) |
| **Languages** | Porter stemmer for English; unicode61 for general |
| **Complexity** | Low — no additional dependencies |

**Best For**: Development environment, small-to-medium deployments, simplicity.

---

### Option 2: Tantivy

**What it is**: Pure Rust full-text search engine library, inspired by Apache Lucene. Powers Quickwit (distributed search) and ParadeDB (Postgres extension).

**Crate**: `tantivy` (10k+ GitHub stars, actively maintained)

**Implementation**:

```rust
use tantivy::{schema::*, Index, IndexWriter, collector::TopDocs, query::QueryParser};

// Define schema
let mut schema_builder = Schema::builder();
let title = schema_builder.add_text_field("title", TEXT | STORED);
let content = schema_builder.add_text_field("content", TEXT);
let id = schema_builder.add_text_field("id", STRING | STORED);
let schema = schema_builder.build();

// Create index
let index = Index::create_in_dir(&index_path, schema.clone())?;

// Index a document
let mut writer: IndexWriter = index.writer(50_000_000)?; // 50MB buffer
writer.add_document(doc!(
    id => item.id.to_string(),
    title => item.title.clone(),
    content => item.content.clone(),
))?;
writer.commit()?;

// Search
let reader = index.reader()?;
let searcher = reader.searcher();
let query_parser = QueryParser::for_index(&index, vec![title, content]);
let query = query_parser.parse_query("architecture patterns")?;
let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
```

**Capabilities**:

- BM25 ranking
- Boolean queries, phrase queries, fuzzy search
- Faceted search
- Range queries on numeric fields
- Highlighting
- 17+ language stemmers
- Custom tokenizers
- Columnar storage for fast aggregations

| Aspect | Assessment |
|--------|------------|
| **Maturity** | Excellent — production-ready |
| **Performance** | Excellent — often faster than Lucene |
| **Rust Support** | Native, idiomatic API |
| **Languages** | 17+ with stemming; CJK via plugins |
| **Complexity** | Medium — separate index to manage |

**Best For**: Production deployments, large document collections, advanced search features.

---

### Option 3: PostgreSQL Full-Text Search

**What it is**: Built-in `tsvector`/`tsquery` full-text search in PostgreSQL.

**Implementation**:

```sql
-- Add search vector column
ALTER TABLE knowledge_items ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(title, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(summary, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(content, '')), 'C')
    ) STORED;

-- Create GIN index
CREATE INDEX idx_knowledge_search ON knowledge_items USING GIN(search_vector);

-- Search with ranking
SELECT *, ts_rank(search_vector, query) as rank
FROM knowledge_items, plainto_tsquery('english', 'architecture patterns') query
WHERE search_vector @@ query
ORDER BY rank DESC
LIMIT 20;
```

| Aspect | Assessment |
|--------|------------|
| **Maturity** | Excellent — core PostgreSQL |
| **Performance** | Good for most use cases |
| **Rust Support** | Via SQLx (SQL strings) |
| **Languages** | Multiple dictionaries available |
| **Complexity** | Low — no additional infrastructure |

**Best For**: Production when PostgreSQL is already the database, simpler deployments.

---

### Recommendation: Dual Strategy

| Environment | Search Backend | Rationale |
|-------------|----------------|-----------|
| **Development** | SQLite FTS5 | Simple, portable, no setup |
| **Production (simple)** | PostgreSQL FTS | Already have Postgres; good enough |
| **Production (advanced)** | Tantivy | Large scale, advanced features |

**Abstraction**:

```rust
#[async_trait]
pub trait SearchIndex: Send + Sync {
    async fn index(&self, item: &KnowledgeItem) -> Result<(), SearchError>;
    async fn remove(&self, id: &KnowledgeId) -> Result<(), SearchError>;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SearchError>;
    async fn reindex_all(&self) -> Result<(), SearchError>;
}

pub struct Fts5SearchIndex { /* SQLite FTS5 */ }
pub struct PostgresSearchIndex { /* Postgres tsvector */ }
pub struct TantivySearchIndex { /* Tantivy */ }
```

---

## Content Storage: Files vs. Database

### The Core Question

Should knowledge item content live:

- **In the database** as TEXT/JSONB columns?
- **On the filesystem** as markdown files?
- **Hybrid** based on size or type?

### Analysis

| Factor | Database Storage | Filesystem Storage |
|--------|------------------|-------------------|
| **Consistency** | Transactional with metadata | Requires sync logic |
| **Editability** | Only via Fabryk API | Any editor (VS Code, Obsidian, vim) |
| **Portability** | Export required | Files are portable |
| **Version Control** | Custom implementation | Git works natively |
| **Large Files** | DB bloat | Handles well |
| **Backup** | Single DB backup | Files + DB |
| **Search Indexing** | Direct from DB | Must read files |
| **ACL Enforcement** | At storage layer | At query layer |

### Use Case Consideration

Given Fabryk's target use case:

- ~90% of knowledge items are **markdown documents**
- Documents are often **outputs of AI workflows** (ECL)
- Users may want to **edit files directly**
- **Version control** (git) is highly desirable
- **Portability** matters (not locked into Fabryk)

---

## Recommended Architecture: Filesystem-Primary Hybrid

### Directory Structure

```
fabryk-data/
├── partitions/
│   ├── personal-alice/
│   │   ├── .partition.toml              # Partition metadata
│   │   ├── analysis-report.md           # Knowledge item (frontmatter + content)
│   │   ├── architecture-notes.md
│   │   └── assets/                      # Binary attachments
│   │       └── diagram.png
│   │
│   ├── team-engineering/
│   │   ├── .partition.toml
│   │   ├── coding-standards.md
│   │   └── onboarding-guide.md
│   │
│   └── project-alpha/
│       ├── .partition.toml
│       ├── requirements.md
│       └── design-doc.md
│
├── .fabryk/
│   ├── config.toml                      # Fabryk configuration
│   ├── index.db                         # SQLite: metadata cache, ACL
│   └── search/                          # Tantivy index directory
│       ├── meta.json
│       └── ...
│
└── .gitignore                           # Ignore .fabryk/ if desired
```

### Knowledge Item Format

Markdown files with YAML frontmatter:

```markdown
---
id: "550e8400-e29b-41d4-a716-446655440000"
title: "Architecture Analysis Report"
created_at: "2026-01-15T10:30:00Z"
updated_at: "2026-01-15T14:22:00Z"
created_by: "alice"
tags:
  - type:analysis
  - domain:architecture
  - project:alpha
summary: "Comprehensive analysis of the current system architecture with recommendations for improvement."
---

# Architecture Analysis Report

## Executive Summary

This document presents a comprehensive analysis of the current system architecture...

[Content continues...]
```

### Partition Metadata

`.partition.toml` in each partition directory:

```toml
[partition]
id = "project-alpha"
name = "Project Alpha"
description = "Knowledge base for Project Alpha"
owner = "team-engineering"  # Identity or group
created_at = "2026-01-01T00:00:00Z"

[defaults]
tags = ["project:alpha"]

[policy]
# Default policy for items in this partition
default_access = "deny"  # or "owner-only", "group-read"
```

### Database Schema (Metadata Cache)

```sql
-- Partitions (cached from .partition.toml files)
CREATE TABLE partitions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    owner_id TEXT NOT NULL,
    owner_type TEXT NOT NULL,  -- 'identity' or 'group'
    path TEXT NOT NULL,        -- Filesystem path
    created_at TIMESTAMPTZ NOT NULL,
    synced_at TIMESTAMPTZ NOT NULL
);

-- Knowledge items (cached from markdown frontmatter)
CREATE TABLE knowledge_items (
    id TEXT PRIMARY KEY,
    partition_id TEXT NOT NULL REFERENCES partitions(id),
    title TEXT NOT NULL,
    summary TEXT,
    file_path TEXT NOT NULL,   -- Relative path within partition
    content_hash TEXT,         -- For change detection
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    created_by TEXT NOT NULL,
    synced_at TIMESTAMPTZ NOT NULL
);

-- Tags (normalized for efficient querying)
CREATE TABLE knowledge_tags (
    item_id TEXT NOT NULL REFERENCES knowledge_items(id) ON DELETE CASCADE,
    namespace TEXT,
    value TEXT NOT NULL,
    PRIMARY KEY (item_id, namespace, value)
);

CREATE INDEX idx_tags_lookup ON knowledge_tags(namespace, value);

-- ACL tables (as defined in main proposal)
-- identities, groups, group_members, grants...

-- FTS5 for SQLite dev environment
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    id UNINDEXED,
    title,
    summary,
    content,
    tokenize='porter unicode61'
);
```

---

## Sync Mechanism

### File Watcher

Use the `notify` crate for filesystem watching:

```rust
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;

pub struct FabrykWatcher {
    watcher: RecommendedWatcher,
    index: Arc<dyn SearchIndex>,
    db: Pool<Sqlite>,
}

impl FabrykWatcher {
    pub fn new(data_dir: &Path, index: Arc<dyn SearchIndex>, db: Pool<Sqlite>) -> Result<Self> {
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(2))?;
        watcher.watch(data_dir.join("partitions"), RecursiveMode::Recursive)?;

        // Spawn handler task
        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                match event {
                    DebouncedEvent::Create(path) |
                    DebouncedEvent::Write(path) => {
                        if path.extension() == Some("md") {
                            self.sync_file(&path).await;
                        }
                    }
                    DebouncedEvent::Remove(path) => {
                        self.remove_file(&path).await;
                    }
                    _ => {}
                }
            }
        });

        Ok(Self { watcher, index, db })
    }

    async fn sync_file(&self, path: &Path) -> Result<()> {
        // 1. Parse frontmatter
        let (frontmatter, content) = parse_markdown_file(path)?;

        // 2. Update database
        self.update_db_metadata(&frontmatter, path).await?;

        // 3. Update search index
        self.index.index(&KnowledgeItem::from_frontmatter(frontmatter, content)).await?;

        Ok(())
    }
}
```

### Initial Sync / Reindex

```rust
impl Fabryk {
    /// Full sync of filesystem to database and search index
    pub async fn sync_all(&self) -> Result<SyncReport> {
        let mut report = SyncReport::default();

        // Walk all partition directories
        for entry in WalkDir::new(&self.partitions_dir)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension() == Some("md".as_ref()) {
                match self.sync_file(path).await {
                    Ok(_) => report.synced += 1,
                    Err(e) => {
                        report.errors.push((path.to_owned(), e));
                    }
                }
            } else if path.file_name() == Some(".partition.toml".as_ref()) {
                self.sync_partition(path).await?;
                report.partitions += 1;
            }
        }

        // Remove stale entries
        report.removed = self.remove_stale_entries().await?;

        Ok(report)
    }
}
```

### CLI Commands

```bash
# Full reindex
fabryk sync --full

# Watch mode (continuous sync)
fabryk watch

# Check sync status
fabryk status

# Verify integrity
fabryk verify --fix
```

---

## Frontmatter Parsing

### Crate Options

| Crate | Description | Recommendation |
|-------|-------------|----------------|
| `gray_matter` | YAML/TOML/JSON frontmatter extraction | Good, well-maintained |
| `markdown-meta-parser` | Focused on YAML frontmatter | Simpler, focused |
| `serde_yaml` + manual | Parse YAML after extracting | Most flexible |

### Implementation

```rust
use serde::{Deserialize, Serialize};
use gray_matter::{Matter, engine::YAML};

#[derive(Debug, Deserialize, Serialize)]
pub struct KnowledgeFrontmatter {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
}

pub fn parse_markdown_file(path: &Path) -> Result<(KnowledgeFrontmatter, String)> {
    let content = fs::read_to_string(path)?;
    let matter = Matter::<YAML>::new();
    let result = matter.parse(&content);

    let frontmatter: KnowledgeFrontmatter = result.data
        .ok_or_else(|| anyhow!("No frontmatter found"))?
        .deserialize()?;

    Ok((frontmatter, result.content))
}

pub fn write_markdown_file(
    path: &Path,
    frontmatter: &KnowledgeFrontmatter,
    content: &str
) -> Result<()> {
    let yaml = serde_yaml::to_string(frontmatter)?;
    let output = format!("---\n{}---\n\n{}", yaml, content);
    fs::write(path, output)?;
    Ok(())
}
```

---

## Binary / Large File Handling

### Strategy

| Content Type | Size | Storage |
|--------------|------|---------|
| Markdown | Any | File in partition directory |
| Images, PDFs | <10MB | `assets/` subdirectory |
| Large binaries | >10MB | Separate blob storage |
| Embeddings | N/A | Database (binary column or separate table) |

### Asset References in Markdown

```markdown
---
id: "..."
title: "Design Document"
assets:
  - path: "assets/architecture-diagram.png"
    type: "image/png"
    size: 245632
---

# Design Document

## Architecture

![Architecture Diagram](assets/architecture-diagram.png)

The system consists of three main components...
```

### Large Blob Storage

For files >10MB, use content-addressed storage:

```
fabryk-data/
├── partitions/
│   └── ...
├── blobs/
│   ├── ab/
│   │   └── cdef1234567890...  # SHA-256 hash as filename
│   └── ...
└── .fabryk/
    └── ...
```

```rust
pub struct BlobStore {
    base_path: PathBuf,
}

impl BlobStore {
    pub async fn store(&self, content: &[u8]) -> Result<BlobRef> {
        let hash = sha256(content);
        let hex = hex::encode(&hash);
        let dir = self.base_path.join(&hex[0..2]);
        let path = dir.join(&hex);

        if !path.exists() {
            fs::create_dir_all(&dir)?;
            fs::write(&path, content)?;
        }

        Ok(BlobRef { hash, size: content.len() })
    }

    pub async fn retrieve(&self, blob_ref: &BlobRef) -> Result<Vec<u8>> {
        let hex = hex::encode(&blob_ref.hash);
        let path = self.base_path.join(&hex[0..2]).join(&hex);
        Ok(fs::read(&path)?)
    }
}
```

---

## Git Integration

### .gitignore Template

```gitignore
# Fabryk internal data (can be regenerated)
.fabryk/

# Large blobs (optional - may want to track small ones)
# blobs/

# OS files
.DS_Store
Thumbs.db
```

### Git-Friendly Features

1. **Human-readable files**: Markdown with YAML frontmatter diffs well
2. **Deterministic IDs**: Use UUIDs in frontmatter, not auto-increment
3. **Timestamps in frontmatter**: Git tracks file changes, frontmatter tracks logical changes
4. **No binary in main tree**: Large binaries in separate `blobs/` directory

### Workflow Example

```bash
# Initialize Fabryk in a git repo
cd my-knowledge-base
git init
fabryk init

# Create content
fabryk new --partition project-alpha --title "Design Doc"
# Edit the created markdown file with any editor

# Commit
git add partitions/
git commit -m "Add design doc for project alpha"

# On another machine
git pull
fabryk sync  # Rebuilds index from files
```

---

## API Considerations

### Content Access

When retrieving via API, content can come from:

1. **File read** (source of truth)
2. **Cache** (for performance, if implemented)

```rust
impl Fabryk {
    pub async fn get_item(&self, id: &KnowledgeId) -> Result<KnowledgeItem> {
        // Get metadata from DB
        let metadata = self.db.get_item_metadata(id).await?;

        // Read content from file
        let file_path = self.partitions_dir
            .join(&metadata.partition_id)
            .join(&metadata.file_path);

        let (frontmatter, content) = parse_markdown_file(&file_path)?;

        Ok(KnowledgeItem {
            metadata,
            content,
        })
    }
}
```

### Content Modification

```rust
impl Fabryk {
    pub async fn update_item(
        &self,
        id: &KnowledgeId,
        updates: ItemUpdate
    ) -> Result<KnowledgeItem> {
        // Get current item
        let mut item = self.get_item(id).await?;

        // Apply updates
        if let Some(title) = updates.title {
            item.metadata.title = title;
        }
        if let Some(content) = updates.content {
            item.content = content;
        }
        item.metadata.updated_at = Utc::now();

        // Write back to file
        let file_path = self.resolve_item_path(&item.metadata);
        write_markdown_file(&file_path, &item.to_frontmatter(), &item.content)?;

        // DB and search index will be updated by file watcher
        // Or update synchronously for consistency:
        self.sync_file(&file_path).await?;

        Ok(item)
    }
}
```

---

## Migration Path

### Phase 1: Filesystem + SQLite FTS5

- Files on disk with frontmatter
- SQLite for metadata cache and ACL
- FTS5 for search
- Simple, portable, good for development

### Phase 2: Add Tantivy for Production

- Keep filesystem storage
- Replace FTS5 with Tantivy for better search
- PostgreSQL for ACL (if needed for scale)

### Phase 3: Optional Enhancements

- Content-addressed blob storage for large files
- S3 backend option for blobs
- Distributed search via Quickwit (if needed)

---

## Comparison with Alternatives

### vs. Obsidian

| Aspect | Fabryk | Obsidian |
|--------|--------|----------|
| Storage | Markdown files | Markdown files |
| Metadata | YAML frontmatter | YAML frontmatter |
| Search | Tantivy/FTS5 | Custom |
| ACL | Built-in | None (single-user) |
| API | HTTP + MCP | Plugin-based |
| Multi-user | Yes | No |

**Fabryk differentiator**: Multi-tenant ACL, API access, MCP integration.

### vs. Notion

| Aspect | Fabryk | Notion |
|--------|--------|--------|
| Storage | Local files | Cloud proprietary |
| Portability | High (markdown) | Export required |
| ACL | Fine-grained | Workspace-level |
| API | Open | Proprietary |
| Self-hosted | Yes | No |

**Fabryk differentiator**: Self-hosted, portable, fine-grained ACL.

### vs. Plain Git Repo

| Aspect | Fabryk | Git + Markdown |
|--------|--------|----------------|
| Search | Full-text + semantic | grep |
| Metadata | Structured + indexed | Manual |
| ACL | Built-in | None |
| API | HTTP + MCP | None |

**Fabryk differentiator**: Structured search, ACL, AI integration.

---

## Summary

The recommended storage architecture for Fabryk v1:

1. **Filesystem as source of truth** for content
   - Markdown files with YAML frontmatter
   - Organized in partition directories
   - Git-friendly and portable

2. **Database as metadata cache and ACL store**
   - SQLite for development
   - PostgreSQL for production
   - Synced from filesystem

3. **Pluggable search backend**
   - FTS5 for simple deployments
   - Tantivy for production scale

4. **File watcher for live sync**
   - Changes detected automatically
   - Full reindex available via CLI

This approach provides:

- **Editability**: Use any markdown editor
- **Portability**: Files are self-contained
- **Version control**: Git works naturally
- **Scalability**: Can grow to Tantivy/Postgres
- **Simplicity**: Start with SQLite, evolve as needed

---

## References

### Crates

| Crate | Purpose | URL |
|-------|---------|-----|
| `tantivy` | Full-text search engine | crates.io/crates/tantivy |
| `sqlx` | Async SQL toolkit | crates.io/crates/sqlx |
| `notify` | Filesystem watcher | crates.io/crates/notify |
| `gray_matter` | Frontmatter parsing | crates.io/crates/gray_matter |
| `walkdir` | Directory traversal | crates.io/crates/walkdir |
| `serde_yaml` | YAML serialization | crates.io/crates/serde_yaml |

### SQLite FTS5

- <https://www.sqlite.org/fts5.html>

### Tantivy

- <https://github.com/quickwit-oss/tantivy>
- <https://tantivy-search.github.io/tantivy/tantivy/>

### Prior Art

- Obsidian: <https://obsidian.md>
- Dendron: <https://dendron.so>
- Foam: <https://foambubble.github.io/foam/>
