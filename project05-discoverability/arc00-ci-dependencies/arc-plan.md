# CI and latest-dependency foundation

Status: planned and held for full hosted purge plus the requested AGENTS policy
and source-cleanup integration. Read-only CI checks can run during cleanup.

## Outcome and order

Before auditing or adding project05 features, establish fresh passing CI, update
all external dependencies to current stable versions, and establish green checks
again. Keep the baseline repair and full upgrade sweep separately identifiable.
Do not weaken dependency freshness checks, hide tests or reduce feature coverage
to manufacture a green baseline. If the baseline failure itself requires a
minimal dependency fix, record that dependency and its validation before the
broader upgrade sweep.

The first useful increment is a real, green run on a pinned sanitized source SHA.
The second is a coherent latest-dependency candidate with independent review,
local validation and real CI evidence. This is implementation preparation, not
the full code/architecture audit and not authorization for feature work.

## Observed preflight

Duncan re-enabled `.github/workflows/cicd.yml` on 2026-11-21. The workflow API now
reports active. The previous completed run was 28922892089: debug failed at
Check dependencies and release passed. Those July results are historical evidence,
not a current green baseline. The assistant dispatched fresh run 44943360336 on
sanitized main `856af87561c7e53379ec9094c69a01211d3e5283` without source changes.
The [fresh run](https://github.com/oxur/textrynum/actions/runs/44943360336)
completed with failure in both jobs:

- Debug stopped at `Check dependencies`: the compatible-update check reported
  Redis manifest requirement `1.2` versus `1.8.2` available in that run. Later
  debug build/lint/test steps were skipped.
- Release passed formatting, then `cargo build --locked --all-features --workspace
  --release` failed in `ethnum 1.5.2`, `src/error.rs:16`, with E0512 for
  `mem::transmute(())`. Release lint/tests were skipped.

These are observed baseline failures, not diagnosed root causes or an upgrade
recommendation. No source repair, dependency upgrade or audit was performed.

## Implementation scope when opened

- Pin release/0.5.x after the held cleanup integration, current plan, toolchain,
  manifest/package/feature inventory and actual fresh CI failures.
- Use a dedicated non-default source branch, proposed `codex/ci-dependencies`,
  and a single mutable checkout owner. Reuse a suitable checkout; do not create
  extra worktrees by default. Do not implement on main or the planning branch.
- Restore the real repository-native CI gates. Diagnose from current logs and
  discriminating reproductions; preserve the observed release/debug distinction.
- Inventory all external direct, dev, build, optional, target-specific and Git
  dependencies across manifests, plus relevant CI/tooling version pins. Keep
  internal workspace crates consistently on the intended 0.5.0 version.
- Verify latest stable versions at execution time from authoritative registries
  and upstream release information. Include major-version upgrades, update the
  lockfile and migrate APIs as required. Resolve tightly coupled library families
  together. Explain upstream compatibility constraints; do not silently retain
  older versions while claiming an all-latest result.
- Preserve supported product behavior and expose consequential public API/MSRV
  choices to Duncan where they change the approved compatibility contract.
- Run meaningful format/build/lint/test/package and feature checks plus actual
  debug/release CI for the current candidate. Preserve errors and unavailable
  checks instead of calling partial coverage complete.

## Autonomous Mode and authority

Duncan explicitly selected future code implementation in Autonomous Mode:
CDC `gpt-6.1-sol` High; CC `gpt-6-luna` High, through the Codex/OpenAI runner.
Both requested IDs and High reasoning are present in the local catalog. Apply
these exact settings at dispatch and report actual contexts; no contributor
has been dispatched by this plan. No inherited/default model or substitution
satisfies the selection. The preparation context hands coordination/review to
the selected CDC before execution.

Use CDC + CC with independent review, preserved per-slice prompts/iterations,
file ownership, commits with both Cayrn co-author trailers, ledger discipline,
closure/bubble-up and CDC retrospectives. Concrete slices/prompts are opened only
when the cleanup gate passes and current CI evidence has been inspected. Do not
manufacture a complete implementation recipe from stale logs.

The endpoint is accepted CI/dependency preparation for project05. Do not silently
advance into the audit or whole-project implementation. Publication/tagging,
unrelated provider messages and deployment remain separately gated. Recheck CI
workflow effects before CI pushes/reruns; the inspected workflow only builds,
lints and tests. The user separately authorized release-branch pushes after the
held cleanup sequence; do not conflate that with release publication.

## Evidence and closure

Keep substantive implementation evidence in the owning slice's artifacts unless
an explicit override applies. The later audit's results specifically belong in
project05/artifacts. Show baseline and upgraded candidate identities, exact
version inventory, failures/fixes, local checks, CI URLs and independent review.
A passing historical run, a new lockfile or a declared model name alone does not
close a row. Reconcile all findings and report the bounded endpoint to Duncan.
