# Benchmark Notes

## Build benchmark binaries

```bash
CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo bench --no-run
```

## Run benchmark suite

```bash
CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo bench --bench simulation
```

## Benchmark groups

### `simulation.guardrails`
- `compile_scenario`
- `single_run`
- `single_run_capture_full`, `single_run_capture_none`, `single_run_capture_final`, and
  `single_run_capture_selective`
- `single_run_expanded_semantics`
- `batch_run_sequential`
- `batch_aggregation_all_single_thread` and `batch_aggregation_none_single_thread`
- `batch_aggregation_all_rayon` and `batch_aggregation_none_rayon` (with `--features parallel`)
- `batch_run_expanded_semantics`
- `batch_run_expanded_semantics_rayon` (with `--features parallel`)
- `artifact_write_path`

### `simulation.hotspots`
- `compile_large_topology`
- `single_run_expression_fanout`
- `single_run_expression_fanout_with_events`
- `single_run_sorting_gate_routing`
- `single_run_state_modifiers`
- `batch_run_expression_fanout`
- `batch_run_expression_fanout_rayon` (with `--features parallel`)
- `artifact_write_expanded_capture`
- `artifact_write_expanded_capture_io_only`

## Baseline matrix and manual regression summary

Capture matching default and parallel baselines. Criterion runs can take time;
run each compare against the baseline saved from the same machine/toolchain and
identical workload. The saved metadata labels these as `forward_baseline`: a
baseline captured after a change is valid only for comparisons with future
changes, never as evidence of a historical before/after delta.

```bash
./scripts/bench-criterion save --bench simulation --baseline hotspots-20260224-default
./scripts/bench-criterion save --bench simulation --features parallel --baseline hotspots-20260224-parallel
```

Compare against baselines:

```bash
./scripts/bench-criterion compare --bench simulation --baseline hotspots-20260224-default
./scripts/bench-criterion compare --bench simulation --features parallel --baseline hotspots-20260224-parallel
```

Print a non-failing manual regression summary (default +7% threshold). This is
descriptive only, not a completion gate:

```bash
./scripts/bench-criterion summary --bench simulation --baseline hotspots-20260224-default --threshold 0.07
./scripts/bench-criterion summary --bench simulation --features parallel --baseline hotspots-20260224-parallel --threshold 0.07
```

## Capture-retention heap evidence

The compact-path forward-baseline workflow uses isolated DHAT processes so
allocation statistics from one workload cannot affect another. The fixed case IDs are
`single_full_256`, `batch_none_256x256`, and `batch_all_256x256`; each uses the
deterministic source/sink fixture with a 256-step end condition. Results include
the consumed report checksum, total allocated bytes, peak live bytes, and current
bytes.

Establish the initial versioned forward baseline after the compact-path change:

```bash
./scripts/bench-capture-memory save --baseline capture-retention-v1
```

Compare a future implementation with the same case IDs in isolated DHAT
processes:

```bash
./scripts/bench-capture-memory compare --baseline capture-retention-v1
```

Evidence is stored under `target/capture-memory/` and includes the
`forward_baseline` provenance label plus host and Rust toolchain metadata.
`capture-retention-v1` is post-change evidence for future regression comparisons;
it is not proof of a historical before/after retention delta. Comparison prints and stores
absolute and relative peak-live-heap deltas. It rejects missing, malformed, or
incomparable evidence (including a checksum change), but intentionally does not
enforce an improvement threshold; repeatable regressions need an explicit owner
decision.

## Tracked capture-retention v1 snapshot

The saved baseline artifacts are tracked under
`benchmarks/baselines/capture-retention-v1/` so they remain reviewable after the
local `target/` directory is cleaned. Each snapshot has provenance metadata, the
machine/toolchain metadata produced at capture time, workload checksums, and a
`SHA256SUMS` integrity manifest. These snapshots were captured after the compact
retention change and are deliberately marked `historical_before_after: unavailable`:
they establish a stable reference only for future, same-workload comparisons.

Recreate the checked-in snapshot from a clean same-workload capture:

```bash
./scripts/bench-criterion save --bench simulation --baseline capture-retention-v1
./scripts/bench-criterion archive --baseline capture-retention-v1 \
  --snapshot-dir benchmarks/baselines/capture-retention-v1/criterion-default
./scripts/bench-criterion save --bench simulation --features parallel \
  --baseline capture-retention-v1-parallel
./scripts/bench-criterion archive --baseline capture-retention-v1-parallel \
  --snapshot-dir benchmarks/baselines/capture-retention-v1/criterion-parallel
./scripts/bench-capture-memory save --baseline capture-retention-v1
./scripts/bench-capture-memory archive --baseline capture-retention-v1 \
  --snapshot-dir benchmarks/baselines/capture-retention-v1/dhat-default
```

Verify an archived snapshot from its own directory, because each manifest uses
relative paths:

```bash
(cd benchmarks/baselines/capture-retention-v1/criterion-default && shasum -a 256 -c SHA256SUMS)
(cd benchmarks/baselines/capture-retention-v1/criterion-parallel && shasum -a 256 -c SHA256SUMS)
(cd benchmarks/baselines/capture-retention-v1/dhat-default && shasum -a 256 -c SHA256SUMS)
```

After a normal `target/` cleanup, restore the tracked forward-only snapshots
before running the usual compare commands. Restore verifies the SHA-256 manifest,
captured metadata, and the explicit `historical_before_after: unavailable`
provenance; it refuses to overwrite a local baseline. It does not turn this
post-change snapshot into a historical before/after measurement.

```bash
./scripts/bench-criterion restore --baseline capture-retention-v1 \
  --snapshot-dir benchmarks/baselines/capture-retention-v1/criterion-default
./scripts/bench-criterion restore --bench simulation --features parallel \
  --baseline capture-retention-v1-parallel \
  --snapshot-dir benchmarks/baselines/capture-retention-v1/criterion-parallel
./scripts/bench-capture-memory restore --baseline capture-retention-v1 \
  --snapshot-dir benchmarks/baselines/capture-retention-v1/dhat-default
```

Then run the existing `compare` commands from the baseline matrix or heap
evidence sections above.

## Profiling

Run the hot-path profiling set:

```bash
./benchmarks/run_profiles.sh
BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh
```

Run profiling for all cases:

```bash
./benchmarks/run_profiles_all.sh
BENCH_FEATURES=parallel ./benchmarks/run_profiles_all.sh
```

Generated flamegraphs and derived summaries are written to `benchmarks/profiles/` and include a stable feature label suffix (for example, `__features-default` or `__features-parallel`).
