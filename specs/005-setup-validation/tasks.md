# Tasks — 005-setup-validation

Meta:
- Spec: 005-setup-validation — Compile and validate scenarios
- Depends on: spec:002-core-types, spec:003-error-taxonomy, spec:004-rng-policy
- Global scope:
  - src/validation/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 005-setup-validation (owner: worker:019c8ff6-c19f-7520-a0aa-8d58e5a48ea4) (scope: src/validation/**) (depends: spec:002-core-types, spec:003-error-taxonomy, spec:004-rng-policy)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Compile ScenarioSpec into CompiledScenario with fail-fast checks.
  - Validation: cargo test validation::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:30:00Z
  - Completed_at: 2026-02-24T00:33:00Z
  - Completion note: Implemented deterministic compile/index stage with edge-reference and end-condition-shape validation, plus run/batch config validators.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib validation::` passed.
