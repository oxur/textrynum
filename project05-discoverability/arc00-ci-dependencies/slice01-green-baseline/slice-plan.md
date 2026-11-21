# Slice01 — restore the real CI baseline

Status: open, CC initial assignment ready. Source baseline: `92c72d7058dea91c0d475959e8c5b51b65f577ab`, branch `codex/ci-dependencies`, physical checkout `/Users/oubiwann/lab/oxur/textrynum`. Governing arc: arc00 CI/dependency preparation. This slice delivers a reviewable baseline candidate and actual complete green debug/release CI before the broader dependency sweep; CDC owns subsequent integration/review, with no main/release merge authorized.

## Scope and implementation

Current release/0.5.x run 44952335686 at the source baseline reproduced both failures from run 44943360336: debug freshness rejects Redis requirement 1.2 versus 1.8.2; release build rejects ethnum 1.5.2's transmute of unit into TryFromIntError (0 versus 8 bits). Upstream ethnum v1.5.3 replaces these layout assumptions with safe standard-library error construction. Registry queries on 2026-11-21 report Redis 1.8.2 (MSRV 1.88) and ethnum 1.5.3 as latest non-yanked stable.

Make the minimal baseline dependency repairs: root Redis requirement `1.8.2`, existing features/default-features retained; Cargo-generated targeted lock updates to Redis 1.8.2 and ethnum 1.5.3. Do not broadly update the lockfile yet. Root workspace version 0.5.0, MSRV 1.91, edition, member dependencies, CI coverage and freshness predicates stay intact. No vendoring or compiler downgrade. Additional failures are reported with exact diagnostics to CDC for a preserved corrective assignment; they do not authorize broad opportunistic repair.

Rust CG-B-05/07/10/11 supports Cargo-generated lockfile, compatible caret requirement and centralized workspace inheritance. AP-31 supports taking the actual safe upstream fix instead of adding layout-dependent unsafe shims. Testing Validation Gates/Proportionate Proof selects existing real format/freshness/build/clippy/test gates; no new test duplicating a version literal is useful. Existing Redis client commands, async connection management and error behavior must compile/test unchanged.

## Authority, owners and artifacts

CDC: `/root/ci_dependency_cdc`, dispatched by parent using requested gpt-6.1-sol High; provider/deployment Codex/OpenAI runner, actual backend beyond dispatch metadata unreported. CC: exact gpt-6-luna High via Codex CLI, no substitutions. CDC owns planning; CC exclusively owns source/index while its runner is active. CDC reviews after explicit return. One CC + CDC independent review; arc composition requires fresh independent CDC context or Operator gate. No audit/features, main update, PR, tag, release, publishing, deploy, external support message or privacy operations.

Artifacts: `artifacts/cc-execution.md` records CC intake, focused evidence and failures; `artifacts/cc-runner.jsonl` is the actual runner transcript needed to verify dispatch and intake. Avoid other routine exports/archives. CDC may push the candidate task branch to GitHub after reviewing exact files and rechecking workflow effects. Source commit exact files: Cargo.toml and Cargo.lock, both required co-author trailers. CC has no planning commit authority.

## Acceptance and tests

See sibling ledger. Required gates: `make check-deps-cicd`; `cargo fmt --all -- --check`; locked all-feature workspace build/clippy(-D warnings)/test in debug and release; actual complete debug/release GitHub CI at candidate SHA. Local macOS is supplementary to Linux CI; record platform/toolchain. If local build cost/environment blocks, expose it, and retain the actual CI gate. No silent reduction of features or ignored tests. CDC independently checks diff, targeted lock scope, upstream source and reproduces freshness/format and affected tests/checks; real Linux CI reconciles workspace gates. Broader package/feature inventory belongs in slice02 after this baseline is green.

## Author readiness — 2026-11-21

Source-grounded: source AGENTS, root manifest/lock, CI and Makefile checker inspected; Redis sole consumer manifest and client calls inspected. Full project/arc governing plans and current_decisions read. No source dirty state. Design complete for these two mechanical compatible dependency repairs; any new compile/contract failure is held for CDC. Domain guidance applied above. Recipe and observable gate outcomes in cc-prompt.md; root requirement/locked packages and same unchanged workflow falsify cosmetic-green alternatives. Prompt required reads are bounded and accessible; intake/readback is mandatory. No investigation-only work is commissioned.

## Assignment history

- 2026-11-21 initial `cc-prompt.md`, predecessor none, reason: current CI failures; status ready for dispatch. Current assignment: `cc-prompt.md`.
