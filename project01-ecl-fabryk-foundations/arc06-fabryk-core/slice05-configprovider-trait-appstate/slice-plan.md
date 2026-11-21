# CC Prompt: Fabryk 1.5 — ConfigProvider Trait & AppState

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Preserved historical assignment: [cc-prompt.md](cc-prompt.md). No new assignment is being issued.

<!-- Navigation excerpt from the preserved cc-prompt.md. -->

## Objective

1. Define `ConfigProvider` trait in `fabryk-core::traits`
2. Extract `state.rs` as `AppState<C: ConfigProvider>` in `fabryk-core::state`
3. Ensure `AppState` is generic but still provides the access patterns the
   rest of the codebase needs
4. Verify: trait defined, `AppState` generic, `fabryk-core` compiles

