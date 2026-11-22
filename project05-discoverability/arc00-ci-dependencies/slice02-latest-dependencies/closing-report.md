# Slice02 latest dependencies — CC closing report

Status: implementation candidate returned for independent CDC review. This is not slice acceptance: fresh hosted candidate CI, CDC reproduction, ledger closure and retrospective remain pending. No branch push, PR, publication, merge, audit or project06/07 feature work was performed.

## Source and authority

- Source base: `edc8db69a2ef45d49e1c369ad5cf489dc58d161b`; branch `codex/ci-dependencies`; source HEAD at return is `719c58f245fe4a9115c7c0391d3e268c99157e2f`, with clean index/worktree.
- Binary source diff SHA-256 from the opening source pin to final HEAD: `333e85e7ebb519f177406af46c67c807f894998f8261d7c51f991b7e92f49312`.
- The same selected `gpt-6-luna/high` session was resumed using CDC's independently verified configuration receipt. Normal on-request/workspace-write authority and physical source cwd were used; no model substitution or backend identity claim was made.
- CDC owns design, independent review, planning/ledger changes, task-branch push/CI dispatch and retrospective. CC source/index ownership ends with this return. The seven-row ledger remains CDC-controlled and all rows are still open.
- Initial and iteration01/02 interruption histories, errors, stale claims and logs remain preserved. Iteration01 was intake clarification only; iteration02 and iteration03 were executed correction rounds. No failed log was rewritten to appear successful.

## Delivered behavior and dependency changes

- Reconciled all 42 tracked manifests, all 64 original external package names and 458 external declaration occurrences (73 explicit version sites), with no target-specific or Git dependency tables. The coherent current inventory records 63 retained current direct external packages after removing unused direct `stop-words`; its 0.10.1 selection remains transitive through Lance. Current declaration counts are 456 occurrences / 72 explicit version sites. Internal packages remain 0.5.0. Arrow 58.4.2 is the latest stable version in LanceDB 0.41's Arrow 58 family; Fastembed 7.7.1 upstream pins ORT 2.0.0-rc.15. Inventory: `artifacts/dependency-inventory.json`.
- Updated the RMCP 3.11 consumer facade and documented the approved unreleased 0.5 API migration in the existing Fabryk MCP README, linked from root README. The real legacy HTTP regression covers initialize, tools/list/call success and error, resources/list/read, exact resource metadata/text, connected logging notification delivery, default Host 403 and bounded task cleanup. The SDK legacy logging symbols remain narrowly expected as deprecated due to SEP-2577; no global suppression or policy bypass was added.
- Preserved old English stop-word behavior with the licensed, privately sourced 1298-word ISO corpus and independent old-package product oracle. The current dependency's NLTK-only words are not silently adopted. The license/provenance notice is packaged beside the data.
- Updated Russh, SFTP, ZIP, Fastembed and Avro API consumers only as selected upstream interfaces required; no Kafka live send, JWT policy change, new storage format or public Lance restore feature was introduced. Avro tests decode the produced payload and compare the complete value/consumption boundary.
- Metadata-filtered vector search now scans only metadata plus numeric row IDs in bounded 128-row batches, uses native row-ID restrictions and Lance L2, and merges only returned top-k rows. Empty output grows from actual matches, so a nonempty table plus absent-key predicate and `usize::MAX` returns empty without an up-front huge allocation or an arbitrary new cap. Tests also cover zero limit, all 260 matches across batches at limit 300, late winner, farther-only limit 1, conjunction, absent key, native distances and scores.

## Compatibility artifacts

The three thin baseline writer drivers and upgraded readers are under `artifacts/probes/`; baseline fixture originals, expected state and hashes are retained under `artifacts/baseline-fixtures/`. `persistence-proof.md` gives the precise oracle, versions, logs and limits. Redb whole-state equality, Lance full schema/rows/metadata and copied rebuild, and all three Tantivy product records executed successfully on owned copies. The 27 original fixture files were rehashed and matched after those operations. No example probe copies remain in product crates.

Other durable evidence includes the complete `artifacts/cc-execution.md`, whole-set `artifacts/dependency-inventory.json`, `persistence-proof.md`, per-gate logs, provider cache log, exact linker wrapper source/invocation record, package archive listing and this report. `cc-execution.md` preserves initial return state and superseded/failed attempts and appends the final reconciliation. No redundant config export or copied repository archive was added.

## Gate results and limits

- Actual Rust 1.94.0 minimum: locked workspace all-features/all-targets and default all-targets checks passed. Of 17 isolated declared feature selections, 15 unchanged direct-status witnesses from the complete item347 replay remain applicable; the two source-affected `fabryk` and `fabryk-vector/vector-lancedb` cases were rerun after the final source correction and passed. No feature case was rerun merely for aggregate counts. See `check-msrv-iteration03-all-features02.log`, `check-msrv-iteration03-default02.log`, `feature-msrv-final-summary.txt`, and the two `feature-msrv-iteration03-*-final.log` files.
- Rust 1.100.0: format, freshness (`make check-deps-cicd` with cargo-edit-upgrade 0.13.14), debug/release builds, original strict debug/release Clippy commands, debug/release workspace tests and workspace docs passed with captured Cargo/Make status. Full debug/release tests each reported 2440 passed, 0 failed and 61 ignored. The four explicitly ignored Fastembed tests ran serially with a task-owned cache and actual model inference: 4 passed.
- Native release-link limitation: the original unwrapped macOS release test gate remains a real failure before tests with undefined `___isPlatformVersionAtLeast` in ORT CoreML static objects. An initial broad runtime wrapper passed but was superseded because it added the runtime to too many links. The final task-local wrapper delegates to `/usr/bin/cc` and appends the authentic runtime discovered by `xcrun clang --print-resource-dir` only when `-nodefaultlibs`, `-Wl,-dead_strip`, and a `libort_sys` input are all present. Its SHA-256 is `4d21d6b67753ba8116581a6b5e2517412bc1978b9ef60a456fb48442af94628d`; the invocation log records fresh release links for both `fabryk-cli` and `fabryk-vector`. Under that explicit task-local configuration the full release suite passed. This proves that configuration only; it does not establish an unmodified consumer macOS release link. Root README documents a dynamically discovered runtime configuration and the limitation.
- The first no-escalation release rerun also failed because sandbox loopback denied six existing GCS wiremock tests; the exact complete suite passed on a reviewed loopback-enabled rerun. Logs preserve both statuses.
- Packaging passed after clean source commits: `cargo +1.100.0 package --workspace --locked --no-verify` returned status 0 and produced 43 workspace archives. The `fabryk-fts-0.5.0.crate` file list confirms both the ISO JSON and full notice are included (`artifacts/package-archive-inspection-iteration03.txt`). This is packaging only, not compiled-package or release acceptance. The initial DNS-restricted attempt remains preserved. No hosted CI was dispatched; CDC owns the fresh candidate run.

## DEP2 row proposals

| Row | CC proposal | Evidence/remaining authority |
| --- | --- | --- |
| DEP2-01 | Proposed done | Whole-set inventory and lock constraints above; CDC independently rechecks all manifests/selections. |
| DEP2-02 | Proposed done | Old Redb/Lance/Tantivy files and current roundtrips; see `persistence-proof.md`; CDC must replay/review. |
| DEP2-03 | Proposed done | Actual RMCP HTTP/logging, native vector scoring, stopword and Avro oracles plus compatibility notes; CDC source review remains. |
| DEP2-04 | Proposed done | Actual 1.94 unified/default and all17 isolated feature witnesses as listed above; hosted MSRV job remains pending. |
| DEP2-05 | Proposed done | Local 1.100 gates/provider pass; 43 archives were prepared and the stopword source/notice package contents were inspected. CDC must independently check. |
| DEP2-06 | Open / CDC-owned | Fresh upgraded candidate GitHub Actions debug/release/MSRV jobs, exact SHA and full logs not run by CC. |
| DEP2-07 | Open / CDC-owned | Independent reproduction, formal row closure, retrospective and arc bubble-up pending. |

## Bubble-up and handoff

Capability delivered is a latest-stable dependency candidate with real minimum-toolchain evidence, upgraded SDK consumers, persistent-format compatibility probes and bounded metadata-filtered vector search. The main unanticipated integration findings are Lance's transitive NLTK feature replacing the old ISO word corpus; RMCP 3.11's retained-but-deprecated legacy logging support; and ORT's macOS static-link requirement for the authentic Clang availability runtime when linking with `-nodefaultlibs`. The observable no-match `usize::MAX` capacity panic was fixed by retaining output only as rows arrive, without capping caller limits. No source findings were silently dropped; superseded status/count narratives are corrected in the execution log while original entries remain intact.

CDC must independently review the source diff, reproduce consequential compatibility and boundedness boundaries, update planning-owned records, push only the reviewed task branch, and run the complete fresh candidate CI. The mandatory CDC retrospective remains pending and is linked here as CDC's duty; CC does not self-accept or close the ledger.

## Commits

- `23572bf936cd99395cabad857f91fdd985a96bbd` — dependency declarations, coherent lockfile and CI workflow; exact 15-file list is in `artifacts/cc-execution.md`.
- `719c58f245fe4a9115c7c0391d3e268c99157e2f` — selected SDK consumer adaptations, persistence/MCP/vector/stopword oracles and compatibility documentation; exact 21-file list is in `artifacts/cc-execution.md`.

Both commits were ordinary commits with the required contiguous co-author trailers. The source status was clean after packaging. No detached mutable process remains. Source/index ownership is explicitly returned to CDC for independent review; CDC owns actual candidate CI, formal row closure and the retrospective.
