# Tasks — 033-perf-matrix-and-baseline-ops

Meta:
- Spec: 033-perf-matrix-and-baseline-ops — Feature-aware perf matrix and profiling baseline ops
- Depends on: -
- Global scope:
  - benches/simulation.rs
  - scripts/bench-criterion
  - benchmarks/run_profiles.sh
  - benchmarks/run_profiles_all.sh
  - benchmarks/README.md
  - README.md

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T001: Add explicit Rayon benchmark cases and feature-gated setup (owner: mayor) (scope: benches/simulation.rs) (depends: -)
  - Result: Added `simulation.guardrails/batch_run_expanded_semantics_rayon` and `simulation.hotspots/batch_run_expression_fanout_rayon` behind `parallel` feature gates while preserving existing case IDs.
  - Validation: `cargo test --features parallel` and targeted bench/profile runs completed with both Rayon IDs present in Criterion output.

- [x] T002: Extend profiling and bench scripts for matrix runs and manual +7% regression summaries (owner: mayor) (scope: scripts/bench-criterion, benchmarks/run_profiles.sh, benchmarks/run_profiles_all.sh) (depends: T001)
  - Result: Added `summary` mode with `--threshold`, `BENCH_FEATURES` pass-through, and stable feature-aware profile artifact naming.
  - Validation: `./scripts/bench-criterion summary --bench simulation --baseline hotspots-20260224-default --threshold 0.07` and `./scripts/bench-criterion summary --bench simulation --features parallel --baseline hotspots-20260224-parallel --threshold 0.07`.

- [x] T003: Update benchmark/profiling docs for default + parallel matrix workflow (owner: mayor) (scope: benchmarks/README.md, README.md) (depends: T001, T002)
  - Result: Documented matrix baseline save/compare/summary commands and feature-aware profiling workflows.
  - Validation: Manual doc review against script usage plus successful execution of `./benchmarks/run_profiles.sh` and `BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh`.
