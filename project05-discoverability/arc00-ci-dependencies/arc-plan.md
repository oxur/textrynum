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

### CC runtime and compatibility preflight update — 2026-11-21

The initial exact-Luna runner session `01a2000f-9f2d-7660-97b4-f54373f19a94` reached execution. Existing session `turn_context` reports model gpt-6-luna, effort high, physical source cwd, on-request approval and workspace-write sandbox. No backend identity beyond reported configuration is asserted. CC initially stopped before edits because it treated generic self-description as unavailable runtime metadata; preserved iteration01 supplies the actual metadata witness and resumes remaining intake/implementation. No model substitution and no executed code-refinement pass occurred.

Immutable baseline inventory selected all42 tracked Cargo manifests, root plus41 members, all normal/dev/build/workspace declarations and target tables (zero target tables);64 distinct external packages, no registry query errors. Latest stable selection uses registry max_stable_version, preserving SemVer build metadata (toml1.2.2+spec-1.1.0 and toml_edit0.25.20+spec-1.1.0 are stable). This is prospective version/source inspection, not completion of DEP-01.

Latest LanceDB0.41.0 declares Rust1.94.0 against current MSRV1.91 and binds Arrow^58.0.0 while Arrow latest is60.2.0. CDC recommends the MSRV raise for all-latest and documenting the coupled Arrow constraint. Parent requested an explicit Operator decision; it is pending. Hold dependent slice02 implementation until recorded approval or another disposition; baseline slice01 is unaffected. Registry witnesses: https://crates.io/api/v1/crates/lancedb/0.41.0 and https://crates.io/api/v1/crates/lancedb/0.41.0/dependencies .

## Prospective slice02 migration proposal — current evidence, implementation held

Consumer: the all-latest candidate after accepted slice01. This is the concrete source-grounded route for the two pending Operator contract choices; it does not start the sweep or audit. The complete64-package authoritative registry projection has no query errors; a canonical slice02 version inventory will retain the selected declaration/version/constraint rows when opened, without parallel archives.

| Surface and baseline witness | Proposed migration and preservation check | Decision |
| --- | --- | --- |
| Root rust-version1.91; lancedb0.27.2 locked; fabryk-vector/src/lancedb.rs creates Arrow batches and passes readers/results across SDK boundary | Latest lancedb0.41.0 requires Rust1.94.0 and Arrow^58.0.0. Raise declared MSRV1.94 only after approval; select matching Arrow58 family rather than incompatible latest60.2.0. Keep vector schema, names, score normalization, category filters, embedding-provider trait and persistent table behavior. Existing real temporary LanceDB build/search/filter tests exercise that boundary. | Operator MSRV choice pending; coupled Arrow58 is a real upstream constraint. |
| fabryk-mcp-core/src/lib.rs:114–118 exposes AnnotateAble/Annotated/RawResource; static_resources.rs builds those resources; server.rs implements ServerHandler | Latest rmcp3.11 removes those wrapper types and uses flat Resource/ContentBlock. Recommended: migrate the public SDK reexports with downstream notes for unreleased0.5; preserve ToolRegistry/ToolResult and ResourceRegistry behavior. Adapt only ServerHandler call_tool/read_resource returns via existing From conversions to CallToolResponse/ReadResourceResponse; keep the inner methods returning their existing result types. Resource factory code uses Resource::new plus metadata/annotation setters. No new MRTR/task/UX feature is implied. | Operator facade compatibility choice pending; do not silently remove public reexports. |
| fabryk-vector/src/fastembed.rs new uses fastembed::InitOptions; published fastembed7.7.1 src/lib.rs:147–153 keeps it only as a deprecated alias | Use TextInitOptions::new with the same model/cache options, preserve four model aliases, dimension discovery, blocking-task/mutex ownership and errors. Existing model-resolution tests plus real ignored model-download tests can prove actual provider operation; disclose model/network limits. | Ordinary dependency adaptation, no new model/provider feature. |
| ecl-pipeline-state/src/redb_store.rs tables CHECKPOINTS<&str,&[u8]>, HASHES/METADATA<&str,&str>; no custom Value encoding | Latest redb5.0 removes inherent ReadOnlyTable accessors; retain/use ReadableTable trait and adapt range/borrow syntax only where needed. Upstream Option encoding break does not affect these table definitions. Existing checkpoint/hash round trips/reopen tests remain required. Document upstream prohibition on opening one file concurrently with redb4.2-or-earlier and5.x; do not touch real databases. | No additional data-format decision established for the actual used table types. |
| cicd.yml actions/checkout@v6, rust-cache@v2, rust-toolchain@stable; Makefile uses installed Cargo helpers or latest installers | Latest stable releases queried: checkoutv7.0.1, rust-cachev2.9.3, cargo-binstallv1.28.0, cargo-editv0.13.14. Update action refs to current stable releases in the sweep; keep stable Rust/Ubuntu rolling selection and all freshness/format/build/clippy/test gates. Verify actual helper versions and installer selection rather than inventing package pins/config exports. Recheck workflow effects before CI. | Ordinary tool dependency update; no publication/deploy step. |

RMCP resource contents construction remains text-first, URI-second in both pinned1.3.0 and3.11.0. StaticResources already uses that order; preserve actual URI/content fields and assert them in affected tests. Some old tests reverse those strings, so their mere compilation is not a meaningful wire oracle. No unrelated resource policy repair is commissioned.

Further consequential findings will be collected here for a consolidated decision, rather than piecemeal Operator questions. Remaining package migrations (Avro, base64, dirs, JWT, rdkafka, russh/SFTP, Tantivy and zip, plus compatible versions) must be grounded in their actual consumers/upstream APIs before issuing the full slice prompt. All root/member/internal package versions stay0.5.0. No held project06/07 features or full audit is opened.

Upstream witnesses: published crate archives from crates.io API for fastembed7.7.1, rmcp1.3.0/3.11.0 and lancedb0.41.0; latest registry max_stable_version metadata; https://github.com/cberner/redb/releases/tag/v5.0.0 ; GitHub authoritative releases APIs for CI/helper versions.

## Operator compatibility decisions — approved 2026-11-21

The Operator explicitly answered both choices through the parent relay: **raise minimum Rust to1.94** and **migrate the public facade API**, both recommended options. These supersede the pending-decision statements above. No further approval of these unchanged choices is needed.

- Slice02 may raise workspace `rust-version` from1.91 to1.94 for latest LanceDB0.41.0. Keep Arrow58 as the documented coupled-library constraint against latest Arrow60.2.0. Verify the advertised MSRV meaningfully, separately from latest-stable CI.
- Migrate the unreleased0.5 public RMCP facade to current Resource/ContentBlock APIs and remove the obsolete SDK wrapper reexports where required. Include downstream source migration notes. No layer preserving the removed AnnotateAble/Annotated/RawResource types is required. Preserve higher-level tool/resource registry behavior and adapt handler wrappers internally; no new task/MRTR/subscription feature or ChatGPT UX redesign is authorized by this dependency choice.
- Continue bounded arc00 autonomously with the already approved CDC/CC role/model/settings. Slice02 still waits for independently accepted, actual green baseline CI. Audit, project06/07 features, branch merge and publication remain unauthorized.

### Version history continuation

- 2026-11-21 v1.2 — Operator resolved the two concrete compatibility choices surfaced by prospective slice02 dependency inspection. Adopt Rust1.94 and current RMCP facade migration with notes; retain the green-baseline sequencing gate and all prior obligations.

## Current baseline review and next-slice proof contract — 2026-11-21

Baseline candidate edc8db69a2ef45d49e1c369ad5cf489dc58d161b has passed all local CC gates and CDC's independent scope/format/freshness/Redis checks. Only Cargo.toml/Cargo.lock changed, with Redis1.8.2, ethnum1.5.3 and necessary combine4.6.9. Source/index owner is CDC after CC return. Actual GitHub run44957414892 is still in progress; slice01 is not accepted yet. No main/release update, merge or publication occurred.

The slice02 MSRV row must run the **actual locked candidate on Rust1.94.0**, including `cargo +1.94.0 check --locked --workspace --all-features --all-targets`, plus complementary default/no-default/isolated feature checks as applicable. Current stable1.100 CI and direct-package metadata are separate evidence and cannot establish the declared minimum. If resolved transitive dependencies or source APIs make1.94 false, preserve the concrete failures/parent constraints and seek a disposition rather than silently raising Rust or declaring compatibility. Keep all internal crates0.5.0.

Future CC prompts use a temporary commit-message file, verify its actual bytes and both exact parsed trailers before committing, and prohibit housekeeping amend/squash/fixup/drop operations. Prefer ordinary follow-up commits. The baseline's already-completed unpublished message amendment is retained accurately in its verification/retrospective and is not undone.
