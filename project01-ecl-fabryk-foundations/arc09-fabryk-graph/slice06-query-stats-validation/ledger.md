# CC Prompt: Fabryk 4.6 — Query, Stats, Validation — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] Query response types defined (RelatedConceptsResponse, PathResponse, etc.)
- [ ] `compute_stats()` returns comprehensive GraphStats
- [ ] `validate_graph()` checks for orphans, self-loops, cycles, etc.
- [ ] `ValidationResult` categorizes errors, warnings, and info
- [ ] All types derive Serialize/Deserialize for MCP responses
- [ ] All tests pass

