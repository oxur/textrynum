# Historical criteria — Original pipeline outcomes

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 45-62; full original recoverable from import commit 3a39aea. -->

## Phase Overview

| Phase | Milestone | Focus | Crate(s) | Outcome |
|-------|-----------|-------|----------|---------|
| 1 | **1.1** | Specification Layer | `ecl-pipeline-spec` | TOML config parses, validates, and round-trips through serde |
|| **1.2** | State Layer Types | `ecl-pipeline-state` | State/checkpoint types serialize; `InMemoryStateStore` works |
|| **1.3** | Topology Layer Types | `ecl-pipeline-topo` | Resource graph builds schedule; traits (`SourceAdapter`, `Stage`) defined |
| 2 | **2.1** | RedbStateStore | `ecl-pipeline-state` (extend) | Persistent checkpoint storage with atomic writes via redb |
|| **2.2** | Pipeline Runner Core | `ecl-pipeline` | Runner executes batches with retry, concurrency, and checkpointing |
|| **2.3** | Topology Resolution | `ecl-pipeline-topo` + `ecl-pipeline` (extend) | Spec resolves to wired topology via adapter/stage registries |
| 3 | **3.1** | Filesystem Adapter + Stages | `ecl-adapter-fs`, `ecl-stages` | End-to-end pipeline with local files (extract, normalize, filter, emit) |
|| **3.2** | Integration Test Suite | `ecl-pipeline` (tests) | Comprehensive scenarios: resume, incrementality, errors, fan-in |
| 4 | **4.1** | Google Drive — Auth & Enumerate | `ecl-adapter-gdrive` | OAuth2 auth, folder traversal, pagination, filtering |
|| **4.2** | Google Drive — Fetch & Pipeline | `ecl-adapter-gdrive` (extend) | Content download, format export, full Drive-to-markdown pipeline |
| 5 | **5.1** | CLI Commands | `ecl-cli` (extend) | `run`, `resume`, `status`, `inspect`, `items`, `diff` commands |
|| **5.2** | Observability & Reporting | `ecl-pipeline` + `ecl-pipeline-state` (extend) | Tracing spans, structured JSON output matching design doc section 6 |
| 6 | **6.1** | Slack Stub Validation | `ecl-adapter-slack` | Second adapter proves abstractions hold without trait changes |

