# Tasks — 014-analysis-polars-optional

Meta:
- Spec: 014-analysis-polars-optional — Optional polars analysis surface
- Depends on: spec:010-stats-aggregator, spec:011-artifact-writer
- Global scope:
  - src/analysis/**, Cargo.toml

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 014-analysis-polars-optional (owner: worker:019c9013-de37-7181-a67e-5c72dafeb159) (scope: src/analysis/**, Cargo.toml) (depends: spec:010-stats-aggregator, spec:011-artifact-writer)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement optional feature-gated shaping helpers.
  - Validation: cargo test --features analysis-polars
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T01:13:00Z
  - Completed_at: 2026-02-24T01:20:00Z
  - Completion note: Implemented feature-gated Polars DataFrame helpers for run series, batch series, and per-run final metrics tables.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --features analysis-polars --lib analysis::` passed.
