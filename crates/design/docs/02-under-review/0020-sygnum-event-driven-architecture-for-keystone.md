---
number: 20
title: "Sygnum: Event-Driven Architecture for Keystone"
author: "TOML above"
component: All
tags: [change-me]
created: 2026-03-25
updated: 2026-03-25
state: Under Review
supersedes: null
superseded-by: null
version: 1.0
---

# Sygnum: Event-Driven Architecture for Keystone

> **Status:** Design conversation — pre-implementation
> **Date:** March 25, 2026
> **Context:** Keystone event bus architecture design, evolving ECL from procedural pipeline to event-native substrate
> **Participants:** Duncan + Claude (project chat)

---

## 0. Problem

A textrynum-based (ecl, fabryk, etc.) project called "keystone" needs to be able to consume textual data "live" or "freshly ingested" as well as via recuring, scheduled jobs that reassess textual sources stored in a database. This is a significant shift from the static, unchanging, published document model we started with.

While this code will be part of the textrynum project, the process of publishing crates via crates.io is slow, cumbersume, and often requires a series of version-bumping re-publishings. To shortcut that path, we're going to keep the new textrynum crates in the keystone project until we've reached feature stability for this work. At which point, the crates will simply get moved over as-is.

---

## 1. The Core Insight

As soon as you enumerate the parts of a system that need to *react* to changes, procedural pipelines become unmanageable — until you switch to an event bus, at which point the dependency graph becomes explicit and manageable.

ECL's current Extract → Classify → Load pipeline is the **degenerate case** of an event system: a linear chain where the fan-out is always 1 and ordering is strictly sequential. The general form is an event system. ECL is the special case. Good abstractions always subsume their special cases.

This means we don't bolt an event bus onto the side of ECL. We build the event system (sygnum), and ECL's pipeline patterns become *recipes* expressed in that system.

---

## 2. Naming

**sygnum** — from Latin *signum* (sign, signal, standard), with the textrynum y-swap.

Roman legions oriented on their *signa*: the standards were how you knew what was happening, where to rally, what to react to. An event bus is the signal system that the rest of the architecture orients around.

The name sits naturally alongside its siblings:

| Project | Name | Root | Role |
|---------|------|------|------|
| textrynum | *textrinum* (weaving workshop) | Workspace / org |
| fabryk | *fabric* | Knowledge fabric framework |
| ecl | Extract-Classify-Load | Workflow orchestration engine |
| probā | *probāre* (to prove) | Formal verification (future) |
| **sygnum** | *signum* (signal, standard) | Event signal system |

---

## 3. Architectural Roles

### Sygnum: The Nervous System

Sygnum is the event bus. Signals flow, handlers react. It knows nothing about workflows, retries, coordination, or what any event *means*. It is pure infrastructure.

**Responsibilities:**

- Event trait and envelope (metadata, correlation, causation)
- Publish/subscribe mechanics
- Typed event routing
- Backend abstraction (in-memory → GCP Pub/Sub)
- Backpressure policy
- Subscription lifecycle

**Not responsible for:**

- Workflow definition or orchestration
- Retry logic, backoff, bounded loops
- Domain-specific event semantics
- Configuration-driven wiring

### ECL: The Muscle

ECL is where work gets defined, configured, and executed. It knows about job shapes — "extract, then classify, then load" is one shape; "watch a directory and file things" is another; "critique loop with bounded revisions" is another.

**Responsibilities:**

- Recipe/workflow definition language
- Configuration-driven pipeline wiring
- Operational concerns: retries, backoff, journaling, graceful failure
- Coordination primitives (bounded loops, fan-out/fan-in)
- User-facing API for defining jobs

**What changes:** ECL's execution substrate becomes sygnum events rather than its own bespoke sequential runner. ECL's internals get simpler because it delegates signal routing to sygnum, but its configuration, recipe definition, and operational machinery remain ECL's domain.

**What doesn't change:** ECL is still a workflow engine. It still owns the Extract-Classify-Load pattern. Users who want a configured pipeline still interact with ECL — they don't need to know about sygnum.

### The Relationship

```
┌─────────────────────────────────────────────────┐
│                   Keystone                      │
│  (application: defines events, configures ECL,  │
│   wires to sygnum, owns domain logic)           │
├─────────────────────────────────────────────────┤
│                                                 │
│   ┌──────────┐          ┌──────────────────┐    │
│   │   ECL    │ registers│     Sygnum       │    │
│   │          │ handlers │                  │    │
│   │ recipes, │─────────>│ event bus,       │    │
│   │ config,  │          │ pub/sub,         │    │
│   │ retries  │<──events─│ routing          │    │
│   └──────────┘          └──────────────────┘    │
│                                                 │
└─────────────────────────────────────────────────┘
```

ECL depends on `sygnum-core` (to speak the Event trait and register handlers). ECL does **not** depend on any bus implementation — the application picks the bus and hands it to ECL.

---

## 4. Crate Structure

All crates live in `keystone/crates/` for fast iteration and easy Cloud Run deploys. They use their future textrynum names so extraction is a clean lift-and-publish with no renames.

### Sygnum Crates

| Crate | Role | Dependencies |
|-------|------|-------------|
| `sygnum-core` | `SubscriptionId`, error types | uuid |
| `sygnum-event` | `Event` trait, `EventEnvelope`, `EventHandler` trait, | sygnum-core, serde, chrono |
| `sygnum-bus` | `EventBus` trait, `InMemoryBus` (tokio broadcast), backpressure, ordering | sygnum-core, tokio |
| `sygnum` | Umbrella re-export | sygnum-core, sygnum-bus |

**Future bus backends** (not in initial scope):

- `sygnum-gcp` — GCP Pub/Sub implementation of `EventBus`
- `sygnum-pg` — Postgres LISTEN/NOTIFY implementation

### Keystone Crates

| Crate | Role | Dependencies |
|-------|------|-------------|
| `keystone-events` | Keystone domain-specific event types (`ContentIngested`, `PartnerUpdated`, etc.) | sygnum-core |
| `keystone` | Application binary: wires sygnum + ECL + fabryk, owns subscriber map | sygnum, ecl, fabryk, keystone-events |

### Open Question: sygnum-events

There may be a set of **generic knowledge-fabric events** that any fabryk-based system would use — `ContentIngested`, `ContentUpdated`, `GraphNodeAdded`, `GraphEdgeAdded`, `ExtractionCompleted`. These are not Keystone-specific; they describe changes to the knowledge fabric itself.

Whether these belong in a `sygnum-events` crate or stay in `sygnum-core` is **deferred** until the boundary proves itself in practice. Keystone-specific business events (`PartnerUpdated`, `DealUpdated`, `CommitmentUpdated`) will **not** live in any sygnum crate — they belong to Keystone.

Duncan: I suspect that each part of the larger system will end up having its own `*-events` crate, for all the "public" (as in API) events that part of the system is responsible for. Think `ecl-events`, `fabryk-events`, and, as we've seen above, `keystone-events` or `taproot-events` (taproot being another project based upon textrynum/ecl/fabryk).

---

## 5. Dependency Flow

```
keystone (application)
  ├── sygnum          (picks InMemoryBus, owns the bus instance)
  ├── ecl             (configures recipes, registers handlers with sygnum)
  ├── fabryk           (knowledge fabric: graph, fts, vector, content)
  └── keystone-events (domain event types)

ecl (workflow engine)
  └── sygnum-core     (Event trait, handler registration — NOT bus impl)

sygnum-bus
  └── sygnum-core

fabryk
  └── (no sygnum dependency — fabryk remains independent)
```

Key principle: **ECL depends on sygnum-core for the trait, not on any bus implementation.** The application picks the bus and hands it to ECL. This mirrors fabryk's pattern where `SearchBackend` is a trait in core and the Tantivy implementation is feature-gated.

---

## 6. How It Works: A Concrete Example

A Slack message arrives in Keystone. Here's the event flow:

```
Slack webhook hits Keystone HTTP endpoint
  │
  ▼
Keystone handler publishes to sygnum:
  ContentIngested { source: Slack, channel: #partner-acme, ... }
  │
  ├─→ ECL "extract-classify-load" recipe handler
  │     Extract: parse Slack message structure
  │     Classify: identify as partner communication, tag entities
  │     Load: write to Postgres activities table
  │     Publish: ActivityRecorded { partner: "acme", ... }
  │
  ├─→ Concept Extractor handler (LLM-driven)
  │     Extract structured knowledge from message content
  │     Publish: ConceptExtracted { slug: "acme-budget-2026", ... }
  │     │
  │     ├─→ Relationship Resolver
  │     │     Link to existing concepts
  │     │     Publish: RelationshipDiscovered { ... }
  │     │
  │     ├─→ Contradiction Detector
  │     │     Compare against existing claims
  │     │     Publish: ContradictionDetected { ... } (if applicable)
  │     │
  │     └─→ Graph Updater
  │           Add node + edges to petgraph
  │           Publish: GraphNodeAdded { ... }
  │
  └─→ Entity Updater handler
        Update partner/contact/deal records
        Publish: PartnerUpdated { ... }
```

The user who configured Keystone defined the ECL recipes and the subscriber wiring. Sygnum delivered the events. ECL executed the work with retries and error handling. Fabryk's graph and search engines got updated via their normal APIs — they don't know about sygnum.

---

## 7. The macOS Daemon Example

To illustrate that this architecture is general-purpose, not Keystone-specific:

A user configures a file-watching daemon:

```toml
# config.toml
watch: ~/Downloads
pipeline: auto-file
rules:
  - match: "*.pdf"
    action: classify-and-file
```

Under the hood:

```
macOS FSEvents → notify crate → sygnum: FileCreated { path: ~/Downloads/report.pdf }
                                    │
                                    ▼
                              ECL recipe handler
                              (configured by TOML above)
                                    │
                              Extract: read file metadata
                              Classify: LLM-based categorization
                              Load: move to ~/Documents/Finance/
                                    │
                                    ▼
                              sygnum: FileClassified, FileFiled
```

The user wrote a config file. They don't know about sygnum. ECL read the config, wired handlers to events, and executed the pipeline. Sygnum was the signal bus underneath.

---

## 8. Design Principles

1. **Sygnum is pure signal infrastructure.** No workflow logic, no retries, no domain semantics. If it's about *what happens when an event fires*, it's ECL's job, not sygnum's.

2. **ECL is the user-facing workflow layer.** Configuration, recipes, operational concerns. Its execution substrate is sygnum, but users interact with ECL.

3. **Patterns emerge from wiring, not from primitives.** A sequential pipeline is three handlers chained by events. A critique loop is a handler that conditionally re-publishes. Sygnum doesn't need special support for these — they fall out of the event model.

4. **No database triggers.** All reactivity lives in application code. Explicit, testable, traceable.

5. **Backend-swappable.** `InMemoryBus` for single-process Cloud Run. GCP Pub/Sub for distributed deployments. Same `EventBus` trait, same application code.

6. **Fabryk stays independent.** Fabryk's graph, FTS, and vector engines are updated via their normal APIs by sygnum handlers. Fabryk never imports sygnum.

7. **Keystone owns the wiring.** Which events exist, which handlers subscribe, which ECL recipes run — all application-layer decisions in Keystone, not baked into libraries.

---

## 9. What's Deferred

These are explicitly out of scope for this design document and will be addressed in follow-up conversations:

| Topic | Why Deferred |
|-------|-------------|
| Trait sketches for `EventBus`, `Event`, `EventEnvelope` | Need to sit with the architecture first; traits follow from usage patterns |
| ECL recipe/configuration API in the sygnum world | Needs its own focused design session |
| Confidence/contradiction tracking data model | Separate domain concern, not event infrastructure |
| LLM extraction pipeline details | How to prompt Claude for concept extraction is separate from event architecture |
| Graph store migration (filesystem → Postgres) | The `GraphLoader` trait is in scope eventually; the full migration is follow-up |
| GCP Pub/Sub implementation | Trait should accommodate it; implementation is future work |
| Event storage/replay | Important for debugging and reprocessing; needs its own discussion |
| Ordering, idempotency, backpressure specifics | Informed by implementation experience |
| `sygnum-core` vs `sygnum-events` boundary | Wait to see what shakes out in practice |

---

## 10. Next Steps

1. **Sit with this.** Let the architecture settle before writing code.
1. **Investigate the crates ecosystem.** Let's not reinvent any really *good* wheels of substance and innovative or strong designs.
1. **Design session: sygnum traits.** `EventBus`, `Event`, `EventEnvelope`, `EventHandler` — nail the foundation.
1. **Design session: ECL evolution.** How ECL's recipe/config API works when its substrate is sygnum.
1. **Design session: Keystone subscriber map.** Walk through the full event flow end-to-end.
1. **Code.** Start with `sygnum-core` and `sygnum-bus`, then wire into Keystone.
