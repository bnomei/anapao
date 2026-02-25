# Tasks — 030-live-event-streaming

Meta:
- Spec: 030-live-event-streaming — Stream single-run events during execution
- Depends on: spec:006-step-engine, spec:008-events-contract, spec:012-assertion-engine
- Global scope:
  - src/simulator.rs
  - src/engine/mod.rs
  - src/events/mod.rs
  - tests/**

## In Progress

## Blocked

## Todo
- (none)

## Done
- [x] T001: Add engine-side event emission hook for single-run execution (owner: mayor) (scope: src/engine/mod.rs, src/events/mod.rs) (depends: spec:006-step-engine, spec:008-events-contract)
  - Result: Added streaming-capable `run_single_internal` path and public crate-internal entrypoints (`run_single_streaming`, `run_single_streaming_for_assertions`) that emit `step_start`, `transfer`, `metric_snapshot`, and `step_end` events during execution while preserving deterministic `RunReport` output.
  - Validation: `cargo test engine::` and `cargo test events::`

- [x] T002: Wire simulator single-run APIs to streaming path and keep assertion checkpoint semantics (owner: mayor) (scope: src/simulator.rs) (depends: T001, spec:012-assertion-engine)
  - Result: `Simulator::run` and `Simulator::run_with_assertions` now consume engine streaming directly, preserve sink error mapping to `RunError::EventSink`, and keep assertion checkpoints on the terminal step before final terminal `step_end`.
  - Validation: `cargo test simulator::` and `cargo test`

- [x] T003: Add regression tests for streaming order and sink-failure propagation (owner: mayor) (scope: tests/**) (depends: T001, T002)
  - Result: Added raw-order lifecycle monotonicity checks and sink-failure propagation coverage (push failure short-circuits and does not flush) across simulator and Pikmin integration tests.
  - Validation: `cargo test --test pikmin_diagram`, `cargo test simulator::`, and `cargo test`
