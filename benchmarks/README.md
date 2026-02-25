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
- `single_run_expanded_semantics`
- `batch_run_sequential`
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
