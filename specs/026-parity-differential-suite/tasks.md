# Tasks — 026-parity-differential-suite

Meta:
- Spec: 026-parity-differential-suite — Executable parity differential suite
- Depends on: spec:016-semantic-rulebook-fixtures, spec:020-gate-routing-engine, spec:021-delay-queue-timeline, spec:023-variable-update-timing, spec:024-accuracy-indicator, spec:025-debugger-and-history
- Global scope:
  - tests/parity/**, src/testkit/mod.rs, tests/rstest_testkit.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 026-parity-differential-suite (owner: worker:019c9067-ad36-7e61-bf48-1fa6c28564cb) (scope: tests/parity/**, src/testkit/mod.rs, tests/rstest_testkit.rs) (depends: spec:016-semantic-rulebook-fixtures, spec:020-gate-routing-engine, spec:021-delay-queue-timeline, spec:023-variable-update-timing, spec:024-accuracy-indicator, spec:025-debugger-and-history)
  - Context: Build parity confidence by encoding rule expectations as machine-verifiable golden fixtures and checks.
  - DoD: Implement differential parity suite that validates each documented rule against deterministic fixture outputs.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test parity::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T16:19:45Z
  - Completed_at: 2026-02-24T16:38:31Z
  - Completion note: Added parity catalog/testkit helpers and rstest-driven differential parity checks across fixture ids with evidence-rich failure formatting.
  - Validation result: mayor recheck passed for parity test suite.
