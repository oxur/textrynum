# CI and latest-dependency foundation

Status: authorized for bounded Autonomous execution after the requested AGENTS
policy and source-cleanup integration. Duncan lifted the earlier hosted-purge
hold on 2026-11-21, accepting cleanup so far and keeping that task open without
blocking the remaining work. No repository deletion/recreation is authorized.

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

- Pin release/0.5.x after the requested cleanup integration, current plan, toolchain,
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
after the requested integration and inspection of current CI evidence. Do not
manufacture a complete implementation recipe from stale logs.

The endpoint is accepted CI/dependency preparation for project05. Do not silently
advance into the audit or whole-project implementation. Publication/tagging,
unrelated provider messages and deployment remain separately gated. Recheck CI
workflow effects before CI pushes/reruns; the inspected workflow only builds,
lints and tests. The user separately authorized release-branch pushes after the
requested integration sequence; do not conflate that with release publication.

## Evidence and closure

Keep substantive implementation evidence in the owning slice's artifacts unless
an explicit override applies. The later audit's results specifically belong in
project05/artifacts. Show baseline and upgraded candidate identities, exact
version inventory, failures/fixes, local checks, CI URLs and independent review.
A passing historical run, a new lockfile or a declared model name alone does not
close a row. Reconcile all findings and report the bounded endpoint to Duncan.


## Current execution preflight — 2026-11-21

- Operator continuation lifts only the earlier hosted-purge entry hold. The
  remaining purge task stays open; do not pursue destructive forms or repository
  deletion/recreation. AGENTS policies, migration FF and all 15 release-ref
  publications are complete; see the project privacy record.
- Source: `/Users/oubiwann/lab/oxur/textrynum`, branch `codex/ci-dependencies`,
  baseline `92c72d7058dea91c0d475959e8c5b51b65f577ab`, clean at handoff.
  Planning: separate orphan worktree `.worktrees/planning`. No new source
  worktree is needed. Assign one source/index mutation owner at a time.
- Scope/endpoint: this arc's CI-01, DEP-01, CI-02 and REVIEW-01, using exact
  Operator-selected CDC `gpt-6.1-sol` High for planning/independent review and CC
  `gpt-6-luna` High for implementation. No contributor substitution is authorized.
  A selected context must inspect source and load all required skills before
  issuing concrete slices/prompts. Existing failures are entry evidence, not a
  complete diagnosis or an implementation recipe.
- CDC receives coordination ownership; CC receives source mutation ownership
  only under a durable issued slice prompt. CDC owns planning records and
  independently inspects/reproduces CC evidence. Preserve current prompts,
  iterations, ledgers, closing records and CDC retrospectives.
- Nonpublishing GitHub pushes of the CI/dependency task branch and CI runs are
  necessary within the requested green-CI work. Inspect effects before each
  relevant change. Current workflow runs build/lint/test only; no deployment or
  release job exists. No PR, tag, package publication, new external support
  message, main update or merge of the CI/dependency branch is authorized here.
- Stop after accepted CI/dependency preparation and report; do not automatically
  begin arc01 audit or later feature work. Audit results, when commissioned,
  must go directly in `project05-discoverability/artifacts/`.
- Runtime note: the parent chat has a managed workspace alias pointing through
  a symlink. Sandboxed native/browser tools can fail before startup because of
  that alias. Use real workdir paths and normal reviewed escalation where needed;
  do not weaken the global symlink sandbox setting or change filesystem layout.
  The CC CLI can run with explicit real `--cd` and selected model/settings.

## Active CDC assignment and slice breakdown — 2026-11-21

CDC `/root/ci_dependency_cdc` accepted ownership after parent dispatch with requested `gpt-6.1-sol` High. This is dispatch metadata; backend identity beyond it is unreported. CC exact `gpt-6-luna` High will use Codex CLI with physical --cd and normal reviewed workspace-write approvals. Source remains clean at `92c72d7`.

1. slice01-green-baseline: minimal compatible Redis/ethnum repair and genuine complete green CI; covers CI-01. Newer run [44952335686](https://github.com/oxur/textrynum/actions/runs/44952335686) at `92c72d7` reproduces the same blockers.
2. slice02-latest-dependencies: complete external dependency/tooling inventory and all-latest migration with stated upstream constraints, meaningful local/package/feature checks and actual green CI; opens only after slice01 accepted; covers DEP-01/CI-02. Exact migration recipe follows source/version inspection then.

CDC independently reviews CC source at each slice. The final assembled arc composition has a separate fresh-context or Operator gate; parent can supply the approved CDC model context for that bounded review. No automatic audit. Prompt/evidence homes follow canonical slices; no duplicate archives/config exports.

### Version history

- 2026-11-21 v1.1 — actual CDC ownership recorded; current CI evidence and minimal-first slice decomposition opened, preserving all arc rows and the Operator order.
