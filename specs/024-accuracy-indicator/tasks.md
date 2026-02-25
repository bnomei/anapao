# Tasks — 024-accuracy-indicator

Meta:
- Spec: 024-accuracy-indicator — Accuracy/convergence indicator for predictions
- Depends on: spec:009-monte-carlo-runner, spec:010-stats-aggregator, spec:023-variable-update-timing
- Global scope:
  - src/stats/mod.rs, src/types/mod.rs, src/artifact/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 024-accuracy-indicator (owner: worker:019c903c-8e81-70f2-8490-6a1996216add) (scope: src/stats/mod.rs, src/types/mod.rs, src/artifact/mod.rs) (depends: spec:009-monte-carlo-runner, spec:010-stats-aggregator, spec:023-variable-update-timing)
  - Context: Translate machinations-style accuracy guidance into explicit, deterministic, testable metrics.
  - DoD: Define and compute prediction accuracy/convergence indicators and expose them in reports/artifacts.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib stats:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib artifact::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T15:48:17Z
  - Completed_at: 2026-02-24T16:06:47Z
  - Completion note: Added deterministic prediction indicators (CI/reliability/convergence), prediction artifact output, and summary CSV extensions with stable ordering.
  - Validation result: mayor recheck passed for stats and artifact test suites.
