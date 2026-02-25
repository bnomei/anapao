# Tasks — 012-assertion-engine

Meta:
- Spec: 012-assertion-engine — Expectation evaluation
- Depends on: spec:006-step-engine, spec:010-stats-aggregator, spec:011-artifact-writer
- Global scope:
  - src/assertions/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 012-assertion-engine (owner: worker:019c900b-1fb1-71b0-9244-40a0d05b3464) (scope: src/assertions/**) (depends: spec:006-step-engine, spec:010-stats-aggregator, spec:011-artifact-writer)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement expectation evaluation with evidence references.
  - Validation: cargo test assertions::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:59:00Z
  - Completed_at: 2026-02-24T01:07:00Z
  - Completion note: Implemented run/batch expectation evaluators with typed reports and evidence references for pass/fail diagnostics.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib assertions::` passed.
