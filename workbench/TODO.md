# TODO

## In Progress

  What's Left in 3.6

- Music-theory integration (milestone 2.4 / 3.5 from the overview) — updating ai-music-theory to depend on fabryk crates, swapping imports, verifying MCP tools still work. The repo exists at ~/lab/music-comp/ai-music-theory/mcp-server/.

Questions:

Questions about "Out of Scope" section:

- **fabryk-mcp-* as direct deps** — taproot keeps its own MCP tool handlers (rmcp 0.8)
  - Question: if the projects were using the same version of rmcp, would taproot be able to reuse fabryk-mcp-*?
- **Config migration to fabryk-cli** — taproot keeps its own config struct
  - Question: what would it take to properly generalise the config (in either or both projects) to allow for the reuse of the fabryk config system and tools (including CLI)?

## Pending

- [ ] I don't see a sources_get_chapter tool even though the search description references it in its NEXT guidance — is that planned or removed?
- [ ] The concepts_get parameter is named id but takes a "slug" — minor terminology mismatch that could confuse tool-calling LLMs slightly. Aligning on one term would help.

- Deps
  - [ ]
⏺ Here are the music-theory-specific milestones — the ones that wire Fabryk crates back into ai-music-theory and implement
  domain-specific logic. These are the milestones we skipped in every phase:
  Phase: 1
  Milestone: 1.7
  Description: Music theory integration — add fabryk-core dep to ai-music-theory, implement ConfigProvider for music theory Config,
    update imports, remove extracted files
  ────────────────────────────────────────
  Phase: 2
  Milestone: 2.4
  Description: Music theory integration — add fabryk-content dep, move domain-specific metadata logic (ConceptMetadata type, field
    names) to music theory's own modules
  ────────────────────────────────────────
  Phase: 3
  Milestone: 3.5
  Description: Music theory integration — add fabryk-fts dep, update imports, remove extracted search files, verify search results are
    identical
  ────────────────────────────────────────
  Phase: 4
  Milestone: 4.7
  Description: MusicTheoryExtractor — create music_theory_extractor.rs in ai-music-theory implementing GraphExtractor, move
  ConceptCard,
     RelatedConcepts, and all parsing logic from graph/parser.rs
  ────────────────────────────────────────
  Phase: 4
  Milestone: 4.8
  Description: Graph integration — wire MusicTheoryExtractor into ai-music-theory's graph build pipeline, verify identical node/edge
    counts
  ────────────────────────────────────────
  Phase: 5
  Milestone: 5.3
  Description: Music theory content providers — implement ContentItemProvider for concepts, SourceProvider for sources, wire into tool
    registration
  ────────────────────────────────────────
  Phase: 5
  Milestone: 5.6
  Description: Full MCP integration — wire all fabryk-mcp-* crates into ai-music-theory, implement ToolRegistry, verify all 25 tools
  via
     MCP inspector
  ────────────────────────────────────────
  Phase: 6
  Milestone: 6.3
  Description: Music theory CLI integration — update ai-music-theory main.rs to use fabryk-cli, register domain-specific subcommands
  ────────────────────────────────────────
  Phase: 7
  Milestone: 7.1–7.4
  Description: All of Phase 7 (testing, benchmarking, documentation, cleanup) involves verifying ai-music-theory works end-to-end with
    Fabryk
  In short: Every phase had a final "integration" milestone (.7, .4, .5, .8, .3, .6, .3) that plugs Fabryk back into ai-music-theory.
  Plus Phase 4 has an extra domain milestone (4.7 — the MusicTheoryExtractor implementation) and all of Phase 7 is integration/release
  work.

- [ ] 6.5 - config support, review the CLI capabilities from taproot, use of confyg, etc.

## Completed
