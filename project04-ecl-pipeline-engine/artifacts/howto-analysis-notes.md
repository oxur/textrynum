
<!-- Historical source: workbench/howto-analysis-notes.md, lines 1-57. Original bytes are recoverable from import commit 3a39aea. -->

# Howto Analysis Notes

## Guide Comparison Summary

### Taproot strengths to adopt:
- Step-by-step numbered flow (clear progression)
- rmcp macro details (#[tool_router], #[tool], Parameters<T>)
- Fire-and-forget async side effects
- OptionalParts auth (HTTP + stdio dual-mode)
- Dev-only tool filtering (additive security)
- Structured error JSON with hints for AI agents
- Architecture diagram (ASCII art)

### AI-Kasu strengths to adopt:
- Extractor adapter pattern (GraphExtractor, VectorExtractor, DocumentExtractor)
- ContentItemProvider trait with prefixed tool generation
- Registry composition (CompositeRegistry → ServiceAwareRegistry → DiscoverableRegistry)
- spawn_with_retry with ServiceHandle lifecycle
- ToolMeta + DiscoverableRegistry for auto-generated tool manifests
- ConfigManager + ConfigProvider trait separation
- Testing examples (schema validation, health, OAuth)
- Quick start checklist

### Neither guide covers:
- Full crate dependency graph
- Feature flag selection guide
- fabryk-redis usage
- fabryk-gcp ADC resolution
- fabryk-cli integration for building/validating indices
- Hybrid search (RRF) setup
- ResourceRegistry for deferred resource loading
- VectorSlot for deferred backend wiring
- Content freshness validation (is_index_fresh, is_cache_fresh)
- Graph algorithms beyond basic usage

### Violations/deviations from current API:
- Taproot uses OnceCell directly; current API provides spawn_with_retry (preferred)
- AI-Kasu shows ConfigProvider but actual trait has more methods (to_toml, to_env_vars)
- Neither shows the ResourceRegistry pattern (new feature)
- Neither shows the ServerGuidance pattern (new feature)
- fabryk-mcp-core now re-exports ServiceHandle, ServiceState, and axum (simplifies imports)

### Existing howto coverage:
- mcp-async-startup.md — ServiceHandle lifecycle, spawn_with_retry, parallel wait
- mcp-health.md — health_router, HTTP status codes, testing
- mcp-metadata.md — DiscoverableRegistry, ToolMeta, ExternalConnector
- mcp-with-claude-code.md — stdio/HTTP transport, .mcp.json, FabrykMcpServer builder

### Gaps in existing howtos:
- No howto for content processing (fabryk-content)
- No howto for FTS setup (fabryk-fts)
- No howto for graph building (fabryk-graph)
- No howto for vector search (fabryk-vector)
- No howto for auth setup (fabryk-auth, fabryk-auth-google)
- No howto for Redis integration (fabryk-redis)
- No howto for CLI framework (fabryk-cli)
- No howto for the full end-to-end stack
