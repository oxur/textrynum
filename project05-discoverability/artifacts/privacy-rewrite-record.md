# Privacy rewrite record — 2026-11-21

## Status and authority

Duncan explicitly required anonymization of a private customer throughout source,
fixtures, documents, filenames and Git history, including overwriting affected
refs on every configured remote. Acme Inc. is the prose replacement; code and
path identifiers use anonymized forms. The private original is intentionally
not repeated in this record.

Current status: local rewrite and remote ref replacement completed; full hosted
object/cache purge remains pending. This is privacy-cleanup evidence, not the
planned full code/architecture audit, an independent acceptance, or a 0.5.0 release.

## Preparation and preservation boundaries

Prepared an isolated mirror, fetched all advertised remote refs, and checked
clean ownership of all seven local worktrees. No unsanitized archive or backup
branch is retained. The verified rewrite changes private-name payload/path
occurrences and associated Git identities. Exact tree comparisons against the
anonymization covered all 30 original local refs and 1,219 distinct source blobs,
including the Codex snapshot tree refs. Read-only PR refs were separately checked:
their original reachable objects contained no private-name matches.

All seven local worktrees were refreshed from the rewritten refs. The ignored
first-party reference list was also anonymized. The import/source commit IDs,
migration headers, hashes and sizes were reconciled against sanitized history.
All 138 selected original documents still have complete source-line coverage.
The original 214-source inventory and 76 preferred-version decisions remain
accounted for. Pre-rewrite byte-preservation claims are superseded by the explicit
privacy sanitization; historical mtimes remain capture metadata.

## Remote ref replacement and fresh verification

Updates used exact old-ref leases and atomic transactions per remote, with
successful dry-runs first. Existing advertised branches/tags were replaced;
no new planning/release branches were published and no source cleanup merge or
0.5.0 release was performed. Remote baselines were preserved independently:
Codeberg/macpro did not receive unrelated later source commits.

| Remote | Existing refs replaced | Advertised refs read back | Result |
| --- | ---: | ---: | --- |
| github | 10 | 20 | Exact expected identities matched |
| codeberg | 6 | 12 | Exact expected identities matched |
| macpro | 8 | 16 | Exact expected identities matched |

Freshly fetched object populations across all remotes: 270 commit objects, 1729 tree objects, 1649 blob objects, 6 tag objects. No private-name matches were found in those fetched objects. This verifies advertised history, not unadvertised host storage.

## Confirmed host-side residue — still open

After the successful rewrite, a bounded fetch-by-old-hash probe recovered one
known old main commit from each host. Each recovered tree still contained 11
matching files. All disposable probe repositories were destroyed immediately;
no old sensitive payload was retained as a new archive.

GitHub, Codeberg and macpro therefore still need host-side object/cache cleanup.
A normal fresh clone is clean, but the complete privacy-removal requirement is
not closed. The configured macpro endpoint is a restricted Git service and
rejected shell-style maintenance commands; its admin route is requested from
Duncan. Do not claim that force-pushing alone erased host-retained objects.
No support messages or requests to other people have been sent.

Provider guidance:

- [GitHub sensitive-data removal](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository): cached views and retained references may require provider-side cleanup.
- [Codeberg history replacement and collection](https://docs.codeberg.org/git/reduce-size/): collection is a separate host operation, normally run periodically.

The next owner is Duncan with the required Git-service admin/provider access.
Re-entry is a host-side purge followed by repeating the old-hash and fresh-fetch
checks. No repository deletion, account-wide maintenance, support message, or
changes to other clones are authorized by this record. Existing clones must be
recloned or carefully moved to the rewritten history before they push again.

## Focused implementation validation

`cargo test -p ecl-stages -p ecl-pipeline-spec` passed: 50 configuration/spec tests
and 142 stage tests; 192 total, zero failures. This includes anonymized regex and
fixture cases. The first offline attempts were blocked by missing cached crates;
the online rerun fetched dependencies and passed without changing Cargo.lock.
The full workspace audit has not started and no broader test-coverage claim is made.

The configured GitHub workflow was inspected before publication: pushes trigger
build, lint and test jobs; no release/publishing job was present. Remote CI outcomes
are not claimed by these local test results.

## Hashes and signatures

History rewriting changed commit/tag identities. The translation map is preserved
in `migration-map.json` and the local `.git/filter-repo/commit-map`; it contains
identities rather than sensitive payloads. Historical signatures were not re-signed.

Observed signature metadata: {"old_signed_commits": 3, "rewritten_signed_commits": 0}.

## Local object purge

Local reflogs and obsolete objects are to be expired/pruned after committing this
record, followed by a scan of every remaining object. The final result is appended
here after the purge; this statement does not itself claim completion.

Final local purge check: reflogs expired and obsolete objects pruned. Scanned every remaining local Git object: 270 commit, 6 tag, 2049 tree, 2225 blob; zero private-name matches. First-party live files/paths across all worktrees also had zero matches. Release-tip relationships, clean release worktrees and orphan planning ancestry passed. These are author self-checks.
