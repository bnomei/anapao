# Tasks — 015-perf-guardrails

Meta:
- Spec: 015-perf-guardrails — Benchmark guardrails
- Depends on: spec:006-step-engine, spec:009-monte-carlo-runner, spec:011-artifact-writer
- Global scope:
  - benches/**, benchmarks/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 015-perf-guardrails (owner: worker:019c9014-721f-70f0-b15a-d306c4002507) (scope: benches/**, benchmarks/**) (depends: spec:006-step-engine, spec:009-monte-carlo-runner, spec:011-artifact-writer)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Rebuild criterion benches for engine/batch/artifacts.
  - Validation: cargo bench --no-run
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T01:14:00Z
  - Completed_at: 2026-02-24T01:20:00Z
  - Completion note: Replaced legacy benches with a single deterministic Criterion guardrail benchmark target and added benchmark run documentation.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo bench --no-run` passed.
