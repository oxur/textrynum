# CDC verification — slice16 notification/subscription composition

All five NTF criteria independently accepted for this bounded static facet at
CC candidate67cf34e13740b3ef53749eb22b0084c226469e44. Low040 and Medium041
accepted statically; repeated-subscribe idempotence remains a question. Same
retained LunaHigh author/Sol6.1High CDC; no unavailable fresh reviewer credited.
No runtime/client-receipt/ChatGPT usability or full-pass acceptance.

## Source and interpretation

CDC own reads: notifier1–255, server1–465/694–726/1110–1145, lib1–162; current
notifier1–255/server430–465 rechecked during review. Narrow locked rmcp3.11
references model150–205/225–232/2185–2270, handler/server76–145/213–233/530–583,
service241–247 and service/server790–814/945–960 support version/method meaning.
Current service230–250, handler205–235, peer wrapper790–814 and model150–205
were independently rechecked. This is not third-party implementation coverage.

Original frozen author log records notifier/lib complete and server contiguous
1–230/231–500/501–800/801–1020/1021–1243, each command0/no source truncation.
The three full files1660lines and source-cited pre-edit readbacks are supported;
administrative stop did not erase intake. Current readback reconciled readiness
78847d6, corrected15lineage and actual protected populations before edits.

040: explicit map snapshot sends once, then any failure enters a second loop
sending a new real update parameter to each current subscriber (notifier112–141).
Assume A succeeds/B fails on both passes, no concurrent change: A2successfulmethod
attempts/B2failedattempts, firstpasssuccess return1. All-success skips sweep;
no-entry fallback/log/list_changed prune first-pass failures without resending.
Low fits redundant method work with no measured receipt/processing cost and no
exactly-once promise. A changing current map requires identity-safe future pruning;
snapshot indices cannot blindly identify registrations in a later vector.

041: subscribe records context.peer(server443–445); unsubscribe ignorescontext
and removes entire URI entry(455–458). Bulk helper docs86–93 explicitly specify
all-peer removal; that helper alone is not the defect. With global[A,B], map[A,B],
Aunsubscribe removesmap; fallbackmaystilltargetboth. Aresubscribe createsmap[A],
nexttargetedupdate omitsB thoughBnevercancelled. This server composition conflicts
with caller-specific legacy subscription cancellation. Medium fits correctness
of multiple live client registrations; no actual deployed impact is asserted.
Legacy dispatch is gated by uses_legacy_lifecycle, not every current2026session.
Optional2026listen absence is not graded; ResourceListChanged unsolicited guidance
is not ResourceUpdated guidance, and SHOULD is not an exactly-once SLA.

Both findings have separate controlling mechanisms, owners, compatible fix
directions and discriminating future oracles. Public bulk helper/global fallback
policies remain explicit controls. No protocol migration or repair commissioned.

## Criterion walk

| ID | Independent evidence | Verdict |
| --- | --- | --- |
| NTF-01 | Full author command extents, own logical-unit reads, retained guidance/SDK receipts and cited pre-edit readback | Accepted bounded intake |
| NTF-02 | Shared Arc/Clone/server factory; initialized global peers; lifecycle-gated callback map | Accepted static ownership/version map |
| NTF-03 | Independently walked both manual tables and all-success/bulk/fallback/version controls | Accepted manual source derivation |
| NTF-04 | Low040/Medium041 calibrated, stable IDs/pinned loci/owner/fix/compatibility/oracles; no delivery proof | Accepted static diagnoses |
| NTF-05 | Exact five-path author delta, cumulative prefixes, nine byte-identical protected files, clean719, valid own actual trailers | Accepted preservation/own commit |

## Actual verification, failures and chronology

Author67cf has exactly artifact/report/index/ledger/close. CDC independently
checked report/index start with exact78847d6 bytes and all nine protected files
are byte-identical to that baseline. All prior001–039/coverage statuses preserved.
Hashing reads bytes solely for fingerprints; it is not textual/semantic inspection
or execution. Unfinished CDC preparations stay UNVERIFIED; SEARCH stays pending.
Current source/index719 clean, planning clean at handback. Privacy path/content0.
Both required actual67cf coauthors parse. Check normalized nonempty values as a
set/subset; formatter trailing blank is not a missing author. Initial raw-list
assertion failed on that blank; corrected semantic check0, no message rewrite or
refinement. Staging initially met shared-index permission boundary; normal reviewed
exact staging completed. A guessed verification-report.md read failed ENOENT,
then actual cdc-verification.md was found/read. These are administrative/tool-
bookkeeping observations, not platform flags or source/test failures.

Three author-provided GitHub SDK navigation links could not be resolved by the
CDC web tool(cache misses/timeout); this does not prove them dead. Final report
uses the actual inspected cached locked SDK files with pinned hashes in the
artifact instead of implying those tag links were verified. Ledger's 'both'
criteria typo and 'not opened' hashing wording corrected proportionately byCDC.
No extra author refinement or product rerun was needed.

Initial16exec49038 intentionally SIGINT-stopped idle for root's authorizedfooter
correction: exit1/no16edits/no refinement. Frozen log221384bytes SHA256
60ef964745257f1d21d8ca7de082c5a416d5425b3dd989237704b0b6fe13b7fd unchanged.
Resumption exec14694 completed0, ownership returned; distinct frozen log:
/private/tmp/textrynum-notification-subscription-slice16-resumption01-run.jsonl
435978bytes SHA25624e0ef12387baacdca2e0c4c82203a45289f9fd791569669357705c3a5c442c3.
Both logs screened encoded-private-pattern0. No active owned mutable command.
16refinements0/5;15:1/5;03:2/5. VEC05 satisfied on corrected current lineage,
oldbf193 historicalmalformed; no exception. Original38/31/291 and49cells7/7/35,
SEARCH/fullpasses/final independent composition unchanged. Arc remains active.
