# Incidental audit intake from dependency planning

Status: **source evidence only; not a commissioned or independently accepted audit finding**. The full arc01 audit remains unopened. This note preserves concrete observations discovered while grounding arc00's dependency migration; it supplies no audit coverage, severity verdict, acceptance or authorization to repair.

Observed by CDC `/root/ci_dependency_cdc` on 2026-11-21. Exact source revision: `edc8db69a2ef45d49e1c369ad5cf489dc58d161b` on `codex/ci-dependencies`; physical checkout `/Users/oubiwann/lab/oxur/textrynum`.

## SFTP server-key check

`crates/ecl-adapter-sftp/src/lib.rs:55–73` defines `SshHandler`. Its `client::Handler::check_server_key` at lines65–71 ignores `_server_public_key` and unconditionally returns `std::future::ready(Ok(true))`. The adjacent comments explicitly say all server keys are accepted and leave known-hosts verification as a TODO. `SftpAdapter::connect` at lines123 onward supplies this handler to `client::connect`.

Verification method: direct read of that complete handler and its connection caller from the pinned source, then `git show <revision>:crates/ecl-adapter-sftp/src/lib.rs | nl -ba | sed -n '50,78p'` for exact locators. This establishes the handler's acceptance policy in source. No real SSH/SFTP handshake, adversarial server, exploit, production configuration or downstream deployment was tested. It does not establish the security behavior of every adapter or the whole system.

Audit consumer: arc01's Existing ECL and workspace/system trust-boundary passes should examine the host-key policy, configuration/known-hosts requirements, error behavior, real connection path and appropriate validation oracle. The later audit owns severity, completeness and any repair handoff. Arc00 updates dependencies while preserving existing connection behavior; it does not add a security redesign, authorization/multi-tenancy or other held feature work.

This is current planning intake, not an edit to or acceptance of the imported historical audit reports. Re-pin the observation against the accepted post-upgrade source when the full audit actually opens; line numbers and implementation may drift.

## Resource subscription pruning sends another notification

A second incidental source observation arose while reviewing the dependency change's logging-only deprecation attributes in the complete Notifier implementation. At the same pinned edc8db69 revision, `crates/fabryk-mcp-core/src/notifier.rs:95–134` defines resource_updated's explicit-subscriber path. Lines105–110 send the actual resource update to every subscriber. If any send fails (line113), lines115–127 iterate the stored subscribers and call notify_resource_updated again as the liveness/pruning test, including peers whose first send succeeded. Thus the source contains a second actual send to surviving subscribers on that failure branch; it is not a separate no-op health check. This algorithm predates the current dependency migration, whose inspected diff adds logging deprecation attributes rather than changing this branch.

Verification: complete current-file read and `git show edc8db69a2ef45d49e1c369ad5cf489dc58d161b:crates/fabryk-mcp-core/src/notifier.rs` with exact95–134 line projection. No mixed healthy/disconnected-peer runtime scenario or actual duplicate wire delivery was executed. This is source-only intake, not an accepted defect/severity, audit coverage or permission to repair in arc00. Arc01's MCP notification/lifecycle consumer should test the real failed-send branch and desired pruning/delivery-count behavior against the accepted upgraded source. The current real logging-delivery regression remains a separate dependency acceptance obligation.
