# Notification and subscription composition contracts

Status: ready for independent static assignment after15substantive reviewcb6fbeb.
VEC-05 remains unmet/exception-pending; no Operator reply is assumed. This scope
is independent of15closure and interrupted SEARCH. Same retained LunaHigh author,
Sol6.1High independent CDC; initial0/5 refinements. Source/index719 read-only.

## Population and readiness

Full CC intake: fabryk-mcp-core notifier.rs255, server.rs1243, lib.rs162:
three files1660lines. Trace legacy resource subscription state, shared notifier
ownership, update/pruning calls and server callback composition. Named pass4/
original runtime slice05 and whole-file owners remain open. No actual transport,
service, peer fixture, RPC/notification, Rust or provider operation is assigned.

CDC own current reads: notifier1–255, server1–465/694–726/1110–1145, lib1–162.
SDK references from locked rmcp3.11.0 were read narrowly for protocol/lifecycle
and notification-method meaning, not audited as dependency implementations.
Local cached archive matches product Cargo.lock checksum391f153431ab68664d946513ea0cb3edbca78eaf99c5a983036cacab6220d85b;
selected model.rs,handler/server.rs,service/server.rs,service.rs bytes matched it.
Exact source-reference extents/receipt are in prompt. Current SDKLATEST is2026-
07-28; initialized legacy<=2025-11-25 callbacks are a distinct reachable path.
Do not assume old resource callbacks implement optional subscriptions/listen.

## Concrete assessment

Manually trace mixed first-pass notification success/failure through subscriber
pruning, counting method attempts separately from successful clients and observed
delivery. Compare correct one-pass failure-index controls. Separately trace two
initialized peers subscribing to one URI, one unsubscribing then resubscribing:
bulk removal can be masked temporarily by documented all-peer fallback, so the
second explicit registration is a discriminating step. Current bulk API policy,
missing Peer PartialEq and fallback are contrary evidence, not standalone defects.
Relate any server-composition defect to its actual legacy contract, not held
project06 authorization/multi-tenancy or an uncommissioned protocol migration.

Primary MCP2025-11-25 resource/schema references were read. ResourceUpdated and
ResourceListChanged have different subscription guidance; do not conflate them.
2026 optional-listen nonimplementation alone is not a finding. Stable IDs040+ only
if supported; distinguish pruning redundancy from per-peer lifecycle mechanisms,
keep uncertain semantics/questions explicit. No finding-count target or actual
client/ChatGPT usability claim. All results directly project05/artifacts.

Five NTF criteria cover intake, version/ownership map, manual cases/controls,
calibrated findings and this assignment's own preservation/Git compliance. Passing
this slice cannot waive15's immutable attribution defect. Exact prompt propagates
terminology/stop and message-file/parsed-footer checks before and after commit.
