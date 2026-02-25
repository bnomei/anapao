# Tasks — 020-gate-routing-engine

Meta:
- Spec: 020-gate-routing-engine — Implement sorting/trigger/mixed gate routing
- Depends on: spec:018-edge-state-connection-parity, spec:019-trigger-action-modes
- Global scope:
  - src/engine/mod.rs, src/stochastic/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 020-gate-routing-engine (owner: worker:019c9056-27f8-7d12-bcca-e8a312b2adfb) (scope: src/engine/mod.rs, src/stochastic/mod.rs) (depends: spec:018-edge-state-connection-parity, spec:019-trigger-action-modes)
  - Context: Implement gate-specific conflict resolution and proportion handling with deterministic tie-breaking.
  - DoD: Add deterministic routing engine for sorting/trigger/mixed gates including weighted and chance behavior.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib stochastic::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T15:48:17Z
  - Recovery_note: Worker session disappeared before completion report; mayor performed orphan audit and validation closure.
  - Completed_at: 2026-02-24T16:06:47Z
  - Completion note: Added gate routing internals for deterministic/chance distribution, ratio/percentage validation, and stochastic helpers for weighted/chance selection in engine routing path.
  - Validation result: mayor recheck passed for engine and stochastic test suites.
