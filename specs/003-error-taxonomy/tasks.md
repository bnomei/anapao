# Tasks — 003-error-taxonomy

Meta:
- Spec: 003-error-taxonomy — Typed error hierarchy
- Depends on: spec:001-pivot-surface
- Global scope:
  - src/error.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 003-error-taxonomy (owner: worker:019c8ff1-9dc6-79a1-8450-4b939b48010c) (scope: src/error.rs) (depends: spec:001-pivot-surface)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement SetupError/RunError/AssertionError/ArtifactError + root SimError.
  - Validation: cargo test error::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:22:00Z
  - Completed_at: 2026-02-24T00:27:00Z
  - Completion note: Added typed `thiserror` hierarchy, conversions into `SimError`, and unit tests for display + conversion behavior.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib error::` passed.
