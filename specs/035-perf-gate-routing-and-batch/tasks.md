# Tasks — 035-perf-gate-routing-and-batch

Meta:
- Spec: 035-perf-gate-routing-and-batch — Gate routing and batch orchestration cost reduction
- Depends on: spec:034-perf-expression-fast-path
- Global scope:
  - src/engine/mod.rs
  - src/batch/mod.rs
  - benches/simulation.rs

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T001: Refactor gate routing hot loop to index-based runtime structures (owner: mayor) (scope: src/engine/mod.rs) (depends: spec:034-perf-expression-fast-path)
  - Result: Gate-routing internals now use index-addressed lanes and balancer scores, removing per-token lane scans and avoidable cloning in selection path.
  - Validation: `cargo test` and `cargo test --features parallel` passed including engine determinism tests.

- [x] T002: Remove redundant batch post-sort while preserving deterministic ordering (owner: mayor) (scope: src/batch/mod.rs) (depends: T001)
  - Result: Removed unconditional `run_reports.sort_by_key` and preserved deterministic run-order behavior through stable collection order.
  - Validation: Batch determinism tests passed in both default and parallel test matrices.

- [x] T003: Add/verify perf and determinism coverage for Rayon hotspot benchmark IDs (owner: mayor) (scope: benches/simulation.rs, src/batch/mod.rs) (depends: T001, T002)
  - Result: Added/validated explicit Rayon hotspot benchmark IDs and profile coverage for guardrail/fanout batch cases.
  - Validation: `./scripts/bench-criterion summary --bench simulation --features parallel --baseline hotspots-20260224-parallel --threshold 0.07` and `BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh`.
