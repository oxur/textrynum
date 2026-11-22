# Slice15 — vector construction and provider-identity contracts

Read the public construction APIs and map batching, provider ownership and cache
reuse contracts. Assess concrete source-level inconsistencies with explicit
preconditions and contrary controls. Same immutable source719; separate static
work, not a retry of interrupted content/search runtime. Initial0/5 refinements;
03 remains2/5used. No finding-count target.

## Terminology and stop contract

Use accurate ordinary software engineering terms in new prose/prompts—“code quality assessment”, “behavioral check”, or “API consistency review”—when equally precise. Keep necessary technical facts, exact code identifiers and historical records unchanged. Do not construct keyword blacklists, obscure or relabel the work, or use wording/context/model changes to evade platform restrictions; we do not know which words triggered prior flags. If a task is platform-flagged, stop that affected work and report its evidence without variants or retries.

Propagate this paragraph to later authorized contributor assignments.

## Roles and exact ownership

Same retained CC gpt-6-luna High session01a2019f-73de-7de1-831f-53d55b3da247;
CDC gpt-6.1-sol High independently reviews. Requested receipt already checked;
acknowledge it, hidden backend unreported. Disclose unavailable session without
substitution. Source /Users/oubiwann/lab/oxur/textrynum at
719c58f245fe4a9115c7c0391d3e268c99157e2f on codex/foundation-audit is READ-ONLY,
including index. Planning /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning
is orphanplanning. Use ABSOLUTE reads/writes and physical planning cwd/Git -C.
The source root is exact above, not a guessed .worktrees/source path.

You own ONLY these five paths and planning index:
1. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-vector-construction-contracts.md
2. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-audit-results-rust.md
3. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.21-audit-index.md
4. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice15-vector-construction-contracts/ledger.md
5. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice15-vector-construction-contracts/closing-report.md


Static text/manual reasoning and scoped Markdown bookkeeping only. No Rust/Cargo/
build/test, diagnostic program, executable fixture, embedding/model/provider or
service operation. Do not inspect/reconcile/run/reassign interrupted SEARCH
preparations; preservation hashes only. Source/unknown outputs, coverageJSON and
all original gates are protected. No repair, native model download, CI edit,
push/publication, Daybreak or external message. Whole arc remains incomplete.

## Required source intake and pre-edit readback

Read current arc-plan/ledger, this plan/ledger/prompt, current applicable AGENTS/
project instructions. Retain unchanged full framework, PM wayfinder/operation
and Autonomous contract, work-verification, code-auditing full guide set,
engineering prompt/intake, methodology-stewardship and complete Rust skill/all36
existing guides with anti-patterns first; missingCLI09 known. Re-read relevant
uncertain API/error/collection/data-model/persistence rules as needed. No corpus
restart merely for a slice name and no receipt-only substitute for source reads.

Read COMPLETE under ABSOLUTE source root:
- crates/fabryk-vector/src/builder.rs753lines
- crates/fabryk-vector/src/embedding.rs194
- crates/fabryk-vector/src/backend.rs626
- crates/fabryk-vector/src/types.rs583
- crates/fabryk-vector/src/extractor.rs207
- crates/fabryk-vector/src/lib.rs108
Six files2471lines. Recover any truncation with contiguous ranges. These are
full intake files for a construction facet, not whole implementation acceptance.
CDC own controlling-source extents are recorded in plan, independent of the
unavailable historical reviewer. Do not inspect other modules for new findings.

Normative primary input already read by CDC2026-11-22: official Rust1.100.0
slice documentation, https://doc.rust-lang.org/std/primitive.slice.html#method.chunks,
chunks Panics section explicitly excludes zero chunk size. This is a library
precondition citation, NOT a new Rust execution witness. Cite the URL and
paraphrase rather than reproduce a large external page. Existing relevant Rust
skill collection/public-API guidance also applies.

Give source-cited readback BEFORE edits: complete population, actual public
preconditions/early exits, builder/backend ownership and cache fields, built-in
provider controls, static-versus-runtime limits and exact output/stop contract.

## Source-grounded assessment

1. with_batch_size accepts usize unchanged (builder96–100); build and append
   call documents.chunks(size) at220/357. Trace complete early exits: missing
   path/provider returns Err; a valid cache hit returns before batching; file/
   extraction errors may stop first. Establish exactly when zero reaches chunks;
   don't claim every build panics or that dimension0 (documented auto-detection
   config) implies batch0 is auto. Positive batch/default64 is a control. Do not
   claim VectorConfig.batch_size is automatically wired into an application:
   current in-repo uses are tests/docs; public library inputs are the consumer.
2. Map embedding-space ownership. Backend retains provider from construction
   (backend113–125), while append uses a separately required builder provider
   (builder277–390) and adds documents without rebinding backend. The example
   deliberately reuses provider.clone/current provider; preserve it as a control.
   Check documented preconditions before grading provider mismatch: API type
   acceptance alone does not prove every provider combination is supported.
3. Map cache identity separately: VectorCache holds content_hash/documents;
   freshness compares content hash; load_cache attaches passed provider to old
   documents (backend85–203); builder147–173 takes that hit with current provider
   and reports its dimension. Content hash includes sorted paths/bytes
   (builder408–428), not provider/model identity. Same-provider unchanged-content,
   changed-content miss and skip_cache examples are controls. Separate a present
   cache reuse defect from caller-owned cache partition policy; a missing
   supported contract must remain a question. Existing provider name is for
   diagnostics, not necessarily a stable model fingerprint.
4. Distinguish embedding-space identity, dimensional validity and actual returned
   behavior. Valid built-in mock providers can have8 or16dimensions; manually
   trace old-vector/current-provider metadata through cache and append, without
   generating vectors. Same dimensions do not establish same embedding space;
   a different name alone does not prove different vectors. Backend cosine
   returns0 on length mismatch (205–219), not an error. Default threshold0 may
   retain zero-score items; a positive threshold filters them (230–243). These
   are source branch consequences, not executed retrieval/deployed data loss.
5. Default embed_batch loops once per input and Mock batch maps once per input
   (embedding28–38/94–101). Builder zip224/361 truncates to the shorter iterator,
   but no current built-in cardinality failure is established. Keep short/long
   custom-provider results a defensive-contract question unless current contract
   and product evidence establish a defect; do not inflate missing validation.
6. Consolidate only genuinely shared root causes. Cache fingerprint/reuse and
   append provider ownership may require different fixes/oracles even though
   both concern embedding compatibility. Keep batch preconditions independent.
   No new factory/native/Lance/FTS/MCP/general-search findings in this scope.

## Deliverable and preservation

Companion artifact: complete intake, source-pinned precondition/ownership/cache
matrix, manually derived cases with assumptions/early exits, at least five
specific contrary controls, questions and actual bookkeeping commands/exits.
No executable model or fixture. Detailed findings ONCE in appended cumulative
Rust report, start stable FA-RS-039 only if supported. Each needs severity/rationale,
pinned source719file:line, Scale, static mechanism/current consumer impact,
confidence, logical repair owner, issue-specific fix/compatibility decision and
future discriminating oracle. Separate questions/unavailable checks. Preserve
001–038 bytes/history/status, including037question and accepted038static.

Append one index note; ledger/close author-proposed pending independentCDC. No
whole-file owner,49cell, original vector/graph/MCP/SEARCH gate or arc acceptance.
Full original scope stays intact. All results directly project05/artifacts.

Retain original report/index bytes; append new sections with only needed
separator bytes; verify result starts with EXACT retained prefix. Do not assert
old newline shape or cosmetic status phrasing as semantic gates. Check IDs,
source/link relations and ownership; correct minor prose proportionately.

Before staging screen exact five owned path/content bytes in memory:

```python
import re
pattern = re.compile(bytes([87,97,108,103,114,101,101,110,115]), re.I)
```

COUNTS ONLY; Acme Inc. examples. Verify source719 clean, exact five-path diff,
old prefixes/coverageJSON/SEARCH preservation hashes and meaningful claim/source
reconciliation. Record actual completed statuses in artifact, not stale pending
check prose. Ordinary commit only five paths with BOTH trailers after blank body:

Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>

No amend/squash/fixup/drop/merge/push. Return commit/paths/actual trailers,
conclusions/questions/static limits/privacycounts/source state and explicit
five-path/planning-index ownership handback with no active mutable command.
