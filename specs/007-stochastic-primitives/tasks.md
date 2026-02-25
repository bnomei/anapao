# Tasks — 007-stochastic-primitives

Meta:
- Spec: 007-stochastic-primitives — Dice and distributions
- Depends on: spec:004-rng-policy, spec:005-setup-validation
- Global scope:
  - src/stochastic/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 007-stochastic-primitives (owner: worker:019c8ff6-f208-7f41-86e1-499ae8cf8794) (scope: src/stochastic/**) (depends: spec:004-rng-policy, spec:005-setup-validation)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement distribution specs and deterministic sampling adapters.
  - Validation: cargo test stochastic::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:30:00Z
  - Completed_at: 2026-02-24T00:33:00Z
  - Completion note: Implemented validated distribution specs (`UniformInt`, `Bernoulli`, `Dice`, `WeightedDiscrete`) and deterministic sampling helpers.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib stochastic::` passed.
