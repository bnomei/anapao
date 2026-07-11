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

Capture default and parallel baselines:

```bash
./scripts/bench-criterion save --bench simulation --baseline hotspots-20260224-default
./scripts/bench-criterion save --bench simulation --features parallel --baseline hotspots-20260224-parallel
```

Compare against baselines:

```bash
./scripts/bench-criterion compare --bench simulation --baseline hotspots-20260224-default
./scripts/bench-criterion compare --bench simulation --features parallel --baseline hotspots-20260224-parallel
```

Print a non-failing manual regression summary (default +7% threshold):

```bash
./scripts/bench-criterion summary --bench simulation --baseline hotspots-20260224-default --threshold 0.07
./scripts/bench-criterion summary --bench simulation --features parallel --baseline hotspots-20260224-parallel --threshold 0.07
```

## Capture-retention heap evidence

The pre/post compact-path comparison uses isolated DHAT processes so allocation
statistics from one workload cannot affect another. The fixed case IDs are
`single_full_256`, `batch_none_256x256`, and `batch_all_256x256`; each uses the
deterministic source/sink fixture with a 256-step end condition. Results include
the consumed report checksum, total allocated bytes, peak live bytes, and current
bytes.

Save a named baseline before changing the batch representation:

```bash
./scripts/bench-capture-memory save --baseline capture-retention-pre
```

Compare a later implementation with the same case IDs:

```bash
./scripts/bench-capture-memory compare --baseline capture-retention-pre
```

Evidence is stored under `target/capture-memory/`. Comparison prints and stores
absolute and relative peak-live-heap deltas. It rejects missing, malformed, or
incomparable evidence (including a checksum change), but intentionally does not
enforce an improvement threshold; repeatable regressions need an explicit owner
decision.

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
