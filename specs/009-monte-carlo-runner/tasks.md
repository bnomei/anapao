# Tasks — 009-monte-carlo-runner

Meta:
- Spec: 009-monte-carlo-runner — Batch runner with optional rayon
- Depends on: spec:006-step-engine, spec:004-rng-policy, spec:008-events-contract
- Global scope:
  - src/batch/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 009-monte-carlo-runner (owner: worker:019c9005-fe8f-7bf0-bf4d-64f86d23ecaf) (scope: src/batch/**) (depends: spec:006-step-engine, spec:004-rng-policy, spec:008-events-contract)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement sequential and parallel batch execution with stable merge order.
  - Validation: cargo test batch::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:51:00Z
  - Completed_at: 2026-02-24T00:56:00Z
  - Completion note: Implemented deterministic Monte Carlo batch runner with feature-gated rayon path and stable run ordering.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib batch::` and `--features parallel` both passed.
