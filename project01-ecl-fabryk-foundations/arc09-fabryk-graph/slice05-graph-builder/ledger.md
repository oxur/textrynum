# CC Prompt: Fabryk 4.5 — Graph Builder — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Historical checkbox states are assertions from the source; this migration does not establish acceptance.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Exit Criteria

- [ ] `GraphBuilder<E: GraphExtractor>` generic struct defined
- [ ] `with_content_path()` sets content directory
- [ ] `with_manual_edges()` loads supplementary JSON edges (Amendment §2f-iii)
- [ ] `with_error_handling()` configures fail-fast vs collect vs skip
- [ ] `with_progress()` enables progress callbacks
- [ ] `build()` async method orchestrates full graph construction
- [ ] Manual edges marked with `EdgeOrigin::Manual`
- [ ] `BuildResult` contains graph, stats, and error info
- [ ] All tests pass

