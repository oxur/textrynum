# Ledger — relationship parser contracts

Initial0/5 refinements. CC proposes all five criteria met after scoped checks; independent CDC review pending.

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| REL-01 | Complete substantiated intake and pre-edit readback | Retained5276 plus fresh1362 lines/actual units | correctness-grade | Bounded public contract review | CC-proposed met; CDC pending | Companion intake and cited readback | Same-session receipts; no borrowed reviewer intake |
| REL-02 | Current name/parser/typed consumer contract map | Supported subsets, aliases/case, actual comparisons/loader | correctness-grade | Same | CC-proposed met; CDC pending | All-variant parser map | Missing arms alone insufficient |
| REL-03 | Valid manual cases and contrary controls | Stock producer, filter/typed identity, weights/legacy compatibility | correctness-grade | Same | CC-proposed met; CDC pending | Manual tables and boundaries | Source derivation only; no execution |
| REL-04 | Calibrated findings or questions | Stable045+, owner/compatibility/oracle, root deduplication | correctness-grade | Same | CC-proposed met; CDC pending | FA-RS-045; Q21-01/02 | No finding-count target; no SameAs/Cites finding |
| REL-05 | Exact outputs and existing proof preserved | Fixed baseline raw byte equality, prefixes/source/privacy/parsed authors | correctness-grade | Same | CC-proposed met after scoped commit checks; CDC pending | Closing report and handback | No SEARCH/full-pass acceptance |

## CC close — 2026-11-22

Proposed one Medium static finding, FA-RS-045, for the canonical `contrasts_with` MCP filter being parsed as `Custom` while the stock extractor stores `ContrastsWith`; typed equality excludes the edge. Q21-01 retains the unresolved ManualEdge support/default-weight policy for `SameAs` and `Cites`; Q21-02 retains unknown custom-name casing policy. No parser consolidation, alias change, cache migration, test, runtime call, or repair is selected. All prior findings/questions/coverage/SEARCH gates remain unchanged. Initial slice21 refinement budget 0/5; no refinement consumed.

Precommit checks: source stayed clean at719; planning outputs were based on fixed commit `56d9f3d79f07d68114ebfb24c576679b446408a1`; the original report/index bytes were retained as exact prefixes; the canonical protected manifest was parsed programmatically from immutable commit `78847d67ec54f1617bb3f734ceaf60f86a32f99a` and all nine files matched raw bytes at the fixed baseline. Exact owned-path privacy counts, diff whitespace, commit message/trailers, and actual commit paths are recorded in the closing report/final return. No source execution or SEARCH activity occurred.
