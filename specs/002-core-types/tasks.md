# Tasks — 002-core-types

Meta:
- Spec: 002-core-types — Core simulation/public types
- Depends on: spec:001-pivot-surface
- Global scope:
  - src/types/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 002-core-types (owner: worker:019c8ff1-76c6-7f90-a647-1bbff10bfbe6) (scope: src/types/**) (depends: spec:001-pivot-surface)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement scenario, config, and report types.
  - Validation: cargo test types::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:22:00Z
  - Completed_at: 2026-02-24T00:27:00Z
  - Completion note: Added deterministic-friendly scenario/config/report type system with sorted containers and builder helpers.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib types::` passed.
