# fabryk-fts

Full-text search infrastructure for the Fabryk knowledge fabric.

## Status

Under construction — being extracted from the music-theory MCP server.

## Features

- `fts-tantivy`: Enable Tantivy-based full-text search (recommended)

## Default Schema

Fabryk ships with a sensible default schema suitable for any knowledge domain:

| Field | Type | Purpose |
|-------|------|---------|
| `id` | STRING | Unique identifier |
| `path` | STORED | File path |
| `title` | TEXT | Full-text, boosted 3.0x |
| `description` | TEXT | Full-text, boosted 2.0x |
| `content` | TEXT | Full-text, boosted 1.0x |
| `category` | STRING | Facet filtering |
| `source` | STRING | Facet filtering |
| `tags` | STRING | Facet filtering |
| `chapter` | STORED | Metadata |
| `part` | STORED | Metadata |
| `author` | STORED | Metadata |
| `date` | STORED | Metadata |
| `content_type` | STRING | Content type classification |
| `section` | STORED | Section reference |

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                      fabryk-fts                             │
├─────────────────────────────────────────────────────────────┤
│  SearchBackend trait                                        │
│  ├── SimpleSearch (linear scan fallback)                    │
│  └── TantivySearch (full-text with Tantivy)                │
├─────────────────────────────────────────────────────────────┤
│  SearchSchema (default 14-field schema)                     │
│  SearchDocument (indexed document representation)           │
│  QueryBuilder (weighted multi-field queries)                │
├─────────────────────────────────────────────────────────────┤
│  Indexer (Tantivy index writer)                            │
│  IndexBuilder (batch indexing orchestration)               │
│  IndexFreshness (content hash validation)                  │
└─────────────────────────────────────────────────────────────┘
```

## License

Apache-2.0
