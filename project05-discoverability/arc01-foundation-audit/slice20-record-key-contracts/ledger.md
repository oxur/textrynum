# Ledger — record key contracts

Initial0/5 refinements; CC proposes all five criteria met, pending independent CDC review.

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| KEY-01 | Complete bounded intake and cited current-source readback | Three full files1334lines plus Record/Stage units and relevant current dispatch boundary | correctness-grade | Separate static existing record-stage API facet | CC-proposed met | Companion intake section; pre-edit readback | No original ECL/runtime/SEARCH acceptance |
| KEY-02 | Exact key encoding and supported input/config contract mapped | as_str versus JSON serialization, validation, grouping and join/alias identities | correctness-grade | Separate static existing record-stage API facet | CC-proposed met | Companion Aggregate and Assemble sections | Separates aggregate collision from uncertain repeated-stream support |
| KEY-03 | Manual valid string cases and material controls | Tuple separation, same/different joins, intentional aliases/fallbacks; no observed output | correctness-grade | Separate static existing record-stage API facet | CC-proposed met | Companion derivation tables and controls | Hand-derived only; tests read, not run |
| KEY-04 | Calibrated actionable diagnoses or questions | Stable044+, pinned source/severity/scale/owner/fix/compatibility/oracle; held-feature limits | correctness-grade | Separate static existing record-stage API facet | CC-proposed met | FA-RS-044 in report; Q20-01 in companion | No finding on unsupported repeated-stream policy |
| KEY-05 | Prior evidence and own scoped commit preserved | Five paths/prefixes/nine protected/source719/privacy/both actual parsed authors | correctness-grade | Separate static existing record-stage API facet | CC-proposed after actual scoped commit checks | Closing report and handback | No original coverage/SEARCH/full-arc gate change |
