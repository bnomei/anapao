# Tasks — 006-step-engine

Meta:
- Spec: 006-step-engine — Deterministic step engine
- Depends on: spec:005-setup-validation, spec:007-stochastic-primitives
- Global scope:
  - src/engine/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 006-step-engine (owner: worker:019c8ffa-5c72-7950-b8bb-e22628b5d8fd) (scope: src/engine/**) (depends: spec:005-setup-validation, spec:007-stochastic-primitives)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement minimal deterministic execution pipeline.
  - Validation: cargo test engine::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:36:00Z
  - Completed_at: 2026-02-24T00:42:00Z
  - Completion note: Added deterministic step loop with source generation, ordered transfers, capture policy, and end-condition evaluation.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine::` passed.
