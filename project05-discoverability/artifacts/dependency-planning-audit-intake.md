# Incidental audit intake from dependency planning

Status: **source evidence only; not a commissioned or independently accepted audit finding**. The full arc01 audit remains unopened. This note preserves one concrete observation discovered while grounding arc00's dependency migration; it supplies no audit coverage, severity verdict, acceptance or authorization to repair.

Observed by CDC `/root/ci_dependency_cdc` on 2026-11-21. Exact source revision: `edc8db69a2ef45d49e1c369ad5cf489dc58d161b` on `codex/ci-dependencies`; physical checkout `/Users/oubiwann/lab/oxur/textrynum`.

## SFTP server-key check

`crates/ecl-adapter-sftp/src/lib.rs:55–73` defines `SshHandler`. Its `client::Handler::check_server_key` at lines65–71 ignores `_server_public_key` and unconditionally returns `std::future::ready(Ok(true))`. The adjacent comments explicitly say all server keys are accepted and leave known-hosts verification as a TODO. `SftpAdapter::connect` at lines123 onward supplies this handler to `client::connect`.

Verification method: direct read of that complete handler and its connection caller from the pinned source, then `git show <revision>:crates/ecl-adapter-sftp/src/lib.rs | nl -ba | sed -n '50,78p'` for exact locators. This establishes the handler's acceptance policy in source. No real SSH/SFTP handshake, adversarial server, exploit, production configuration or downstream deployment was tested. It does not establish the security behavior of every adapter or the whole system.

Audit consumer: arc01's Existing ECL and workspace/system trust-boundary passes should examine the host-key policy, configuration/known-hosts requirements, error behavior, real connection path and appropriate validation oracle. The later audit owns severity, completeness and any repair handoff. Arc00 updates dependencies while preserving existing connection behavior; it does not add a security redesign, authorization/multi-tenancy or other held feature work.

This is current planning intake, not an edit to or acceptance of the imported historical audit reports. Re-pin the observation against the accepted post-upgrade source when the full audit actually opens; line numbers and implementation may drift.
