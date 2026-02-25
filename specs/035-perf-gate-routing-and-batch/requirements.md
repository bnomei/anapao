# Requirements — 035-perf-gate-routing-and-batch

## Goal
Reduce gate-routing and batch orchestration overhead while preserving deterministic outputs.

## EARS
- WHEN implementation starts for 035-perf-gate-routing-and-batch THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN sorting or mixed gate routing executes THE SYSTEM SHALL minimize per-token string cloning and repeated lane scans.
- WHEN batch run orchestration completes THE SYSTEM SHALL preserve deterministic run order without unnecessary post-sort overhead.
- WHERE `parallel` is enabled and `ExecutionMode::Rayon` is requested THE SYSTEM SHALL preserve output equivalence with SingleThread execution.
- WHEN performance comparison runs THE SYSTEM SHALL improve these cases versus baseline:
  - `single_run_sorting_gate_routing`: at least 20%
  - `batch_run_expanded_semantics`: at least 10%
- WHERE `parallel` is enabled THE SYSTEM SHALL keep `batch_run_expression_fanout_rayon` faster than the corresponding single-thread case.
- WHEN validation runs THE SYSTEM SHALL pass:
  - `cargo test --features parallel`
  - `./scripts/bench-criterion compare --bench simulation --baseline hotspots-20260224-default`
  - `./scripts/bench-criterion compare --bench simulation --features parallel --baseline hotspots-20260224-parallel`
  - `BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh`
