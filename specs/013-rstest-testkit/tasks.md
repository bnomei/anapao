# Tasks — 013-rstest-testkit

Meta:
- Spec: 013-rstest-testkit — Fixture-first harness
- Depends on: spec:002-core-types, spec:012-assertion-engine
- Global scope:
  - src/testkit/**, tests/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 013-rstest-testkit (owner: worker:019c900b-4f2a-7773-970c-f1899cb5f73a) (scope: src/testkit/**, tests/**) (depends: spec:002-core-types, spec:012-assertion-engine)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Provide rstest fixtures and parameterized harness patterns.
  - Validation: cargo test
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:59:00Z
  - Completed_at: 2026-02-24T01:07:00Z
  - Completion note: Added fixture-first rstest testkit helpers and replaced legacy integration tests with deterministic run/batch fixture coverage.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test` passed.
