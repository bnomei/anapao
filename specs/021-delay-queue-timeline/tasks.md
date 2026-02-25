# Tasks — 021-delay-queue-timeline

Meta:
- Spec: 021-delay-queue-timeline — Implement delay and queue timeline semantics
- Depends on: spec:018-edge-state-connection-parity, spec:019-trigger-action-modes
- Global scope:
  - src/engine/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 021-delay-queue-timeline (owner: worker:019c9067-ad36-7e61-bf48-1fa6c28564cb) (scope: src/engine/mod.rs) (depends: spec:018-edge-state-connection-parity, spec:019-trigger-action-modes)
  - Context: Match documented delay/queue behavior (queue one-at-a-time, delay timing rules) with stable scheduling order.
  - DoD: Add delay/queue scheduling and retry/defer semantics in the step timeline.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T16:07:27Z
  - Completed_at: 2026-02-24T16:19:45Z
  - Completion note: Added timeline runtime scheduling for delay/queue semantics, queue release budgets, and deterministic replay tests for delay/queue behavior.
  - Validation result: mayor recheck passed for engine test suite.
