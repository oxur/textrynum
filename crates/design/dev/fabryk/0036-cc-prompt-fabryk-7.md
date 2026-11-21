---
title: "CC Prompt: Fabryk 7.2 — Performance Benchmarking"
milestone: "7.2"
phase: 7
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["7.1 Testing complete"]
governing-docs: [0013-project-plan]
---

# CC Prompt: Fabryk 7.2 — Performance Benchmarking

## Context

This milestone benchmarks performance to ensure the extraction didn't introduce
regressions. We compare against pre-extraction baselines.

## Objective

Benchmark and verify no regression (>5%) in:

1. Startup time
2. Graph build time
3. Search latency
4. Index build time
5. Memory usage

## Benchmarking Steps

### Step 1: Capture pre-extraction baselines

If not already captured, these are typical baseline values:

```
Pre-extraction baselines (example values):
- Startup time: ~150ms
- Graph build time: ~2.5s (for 342 nodes)
- Search latency (p50): ~5ms
- Search latency (p99): ~25ms
- Index build time: ~8s
- Memory at startup: ~45MB
- Memory after graph load: ~85MB
```

### Step 2: Benchmark startup time

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server

# Measure startup time (time to first ready)
time cargo run --release -- health

# Run multiple times for average
for i in {1..5}; do
    time cargo run --release -- health 2>&1 | grep real
done
```

### Step 3: Benchmark graph build time

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server

# Clean build (no cache)
rm -f data/graphs/graph.json
time cargo run --release -- graph build

# Record times
echo "Graph build time: X.XXs"
```

### Step 4: Benchmark search latency

Create a benchmark script:

```bash
#!/bin/bash
# benchmark_search.sh

QUERIES=(
    "picardy third"
    "dominant seventh"
    "neo-riemannian"
    "parallel fifths"
    "voice leading"
)

echo "Search latency benchmark"
echo "========================"

for query in "${QUERIES[@]}"; do
    # Run search and measure time
    start=$(date +%s%N)
    cargo run --release -q -- search "$query" > /dev/null 2>&1
    end=$(date +%s%N)

    elapsed=$(( (end - start) / 1000000 ))
    echo "$query: ${elapsed}ms"
done
```

### Step 5: Benchmark index build time

```bash
cd ~/lab/oxur/ecl/crates/music-theory/mcp-server

# Clean index
rm -rf data/index

# Build index
time cargo run --release -- index --force

# Record time
echo "Index build time: X.XXs"
```

### Step 6: Benchmark memory usage

```bash
# Using /usr/bin/time for memory measurement
/usr/bin/time -v cargo run --release -- health 2>&1 | grep "Maximum resident"

# Or use heaptrack if available
heaptrack cargo run --release -- serve &
# Let it run for 30s then kill
sleep 30
kill %1
heaptrack_gui heaptrack.*.gz
```

### Step 7: Create benchmark report

```markdown
## Performance Benchmark Report

**Date:** 2026-02-XX
**Baseline:** Pre-extraction (commit XXXX)
**Current:** Post-extraction (commit YYYY)

### Startup Time
| Metric | Baseline | Current | Delta |
|--------|----------|---------|-------|
| Cold start | 150ms | XXXms | +X% |
| Warm start | 80ms | XXXms | +X% |

### Graph Build
| Metric | Baseline | Current | Delta |
|--------|----------|---------|-------|
| Build time | 2.5s | X.Xs | +X% |
| Peak memory | 120MB | XXXMB | +X% |

### Search Latency
| Metric | Baseline | Current | Delta |
|--------|----------|---------|-------|
| p50 | 5ms | Xms | +X% |
| p95 | 15ms | Xms | +X% |
| p99 | 25ms | Xms | +X% |

### Index Build
| Metric | Baseline | Current | Delta |
|--------|----------|---------|-------|
| Build time | 8s | Xs | +X% |
| Index size | 15MB | XXMB | +X% |

### Memory Usage
| Metric | Baseline | Current | Delta |
|--------|----------|---------|-------|
| Startup | 45MB | XXMB | +X% |
| After graph | 85MB | XXMB | +X% |
| Peak | 150MB | XXMB | +X% |

### Verdict
- [ ] No regression > 5% in any metric
- [ ] OR: Regressions documented with justification
```

### Step 8: Address any regressions

If regressions > 5% are found:

1. Profile the affected code path
2. Identify the cause (extra allocations, unnecessary clones, etc.)
3. Fix if possible
4. Document if acceptable trade-off for better architecture

## Exit Criteria

- [ ] Startup time: no regression > 5%
- [ ] Graph build time: no regression > 5%
- [ ] Search latency (p99): no regression > 5%
- [ ] Index build time: no regression > 5%
- [ ] Memory usage: no regression > 10%
- [ ] Benchmark report created
- [ ] Any regressions documented with justification

## Commit Message

```
perf: benchmark Fabryk extraction performance

Benchmark results vs pre-extraction baseline:
- Startup time: XXXms (Δ +X%)
- Graph build: X.Xs (Δ +X%)
- Search p99: Xms (Δ +X%)
- Index build: Xs (Δ +X%)
- Memory peak: XXMB (Δ +X%)

No regressions > 5% threshold.

Phase 7 milestone 7.2 of Fabryk extraction.

Ref: Doc 0013 Phase 7

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
