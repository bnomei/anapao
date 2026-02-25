# Tasks — 036-perf-artifact-writer-throughput

Meta:
- Spec: 036-perf-artifact-writer-throughput — Artifact writer throughput and benchmark fidelity
- Depends on: spec:033-perf-matrix-and-baseline-ops
- Global scope:
  - src/artifact/mod.rs
  - benches/simulation.rs
  - benchmarks/run_profiles.sh

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T001: Refactor artifact write path to reduce avoidable clones and sorts (owner: mayor) (scope: src/artifact/mod.rs) (depends: spec:033-perf-matrix-and-baseline-ops)
  - Result: Added ordered-input fast paths for event/history/variable persistence and only sort/clone when required for deterministic output.
  - Validation: `cargo test` and `cargo test --features parallel` passed artifact compatibility and schema tests.

- [x] T002: Add an I/O-focused expanded artifact benchmark case (owner: mayor) (scope: benches/simulation.rs) (depends: T001)
  - Result: Added `simulation.hotspots/artifact_write_expanded_capture_io_only` benchmark case with stable identity for focused writer throughput profiling.
  - Validation: Case executes in Criterion and appears in profile outputs and summary scans.

- [x] T003: Update profiling workflow to include artifact I/O-only case and delta reporting (owner: mayor) (scope: benchmarks/run_profiles.sh) (depends: T001, T002)
  - Result: Profiling scripts now include I/O-only artifact case and emit stable feature-labeled outputs (`features-default`, `features-parallel`).
  - Validation: `./benchmarks/run_profiles.sh` and `BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh` completed with artifact I/O-only outputs.
