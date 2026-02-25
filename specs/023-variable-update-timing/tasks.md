# Tasks — 023-variable-update-timing

Meta:
- Spec: 023-variable-update-timing — Variable update timing and random/list refresh policies
- Depends on: spec:022-expression-runtime
- Global scope:
  - src/engine/mod.rs, src/types/mod.rs, src/stochastic/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 023-variable-update-timing (owner: worker:019c903c-8e81-70f2-8490-6a1996216add) (scope: src/engine/mod.rs, src/types/mod.rs, src/stochastic/mod.rs) (depends: spec:022-expression-runtime)
  - Context: Support timing-sensitive custom variable behavior aligned with documented prediction/play expectations.
  - DoD: Implement variable update timing controls and list/matrix/random refresh semantics.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib stochastic::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T15:41:05Z
  - Completed_at: 2026-02-24T15:48:17Z
  - Completion note: Added typed variable timing/source model, deterministic interval/list/matrix stochastic sampling helpers, and engine integration for seed-stable refresh policies.
  - Validation result: mayor recheck passed for engine and stochastic test suites.
