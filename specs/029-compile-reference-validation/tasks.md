# Tasks — 029-compile-reference-validation

Meta:
- Spec: 029-compile-reference-validation — Strict compile-time node/metric reference validation
- Depends on: spec:005-setup-validation, spec:006-step-engine
- Global scope:
  - src/validation/mod.rs
  - src/engine/mod.rs
  - src/error.rs
  - tests/**

## In Progress

## Blocked

## Todo
- (none)

## Done
- [x] T001: Add recursive reference validation for end-condition node/metric references (owner: mayor) (scope: src/validation/mod.rs) (depends: spec:005-setup-validation)
  - Result: `compile_scenario` now validates nested end-condition references (`Any`/`All`) and rejects unresolved `NodeAt*`/`MetricAt*` references with path-scoped `SetupError::InvalidGraphReference`.
  - Validation: `cargo test validation::tests::compile_scenario_rejects_`

- [x] T002: Add metric-reference validation for `TransferSpec::MetricScaled` and tracked metrics (owner: mayor) (scope: src/validation/mod.rs, src/engine/mod.rs) (depends: T001)
  - Result: Compile now rejects unresolved metric references in `TransferSpec::MetricScaled` and `scenario.tracked_metrics`, removing reliance on runtime unresolved-metric fallbacks.
  - Validation: `cargo test`

- [x] T003: Add regression tests covering invalid-reference matrix and a valid Pikmin compile path (owner: mayor) (scope: tests/**) (depends: T001, T002)
  - Result: Added coverage for missing end-condition node ref, missing end-condition metric ref, missing `MetricScaled` ref, unresolved tracked metric ref, and a resolved positive compile case; also added Pikmin compile regression for unresolved tracked metric.
  - Validation: `cargo test validation::tests::compile_scenario_rejects_` and `cargo test --test pikmin_diagram`
