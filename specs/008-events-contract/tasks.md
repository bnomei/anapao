# Tasks — 008-events-contract

Meta:
- Spec: 008-events-contract — Run event schema and sink
- Depends on: spec:006-step-engine
- Global scope:
  - src/events/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 008-events-contract (owner: worker:019c9002-5f9c-7483-8d44-7237aee28279) (scope: src/events/**) (depends: spec:006-step-engine)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement stable event payloads and EventSink trait.
  - Validation: cargo test events::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:44:00Z
  - Completed_at: 2026-02-24T00:49:00Z
  - Completion note: Added run event schema with ordering metadata, sink trait/error types, and deterministic sorting helpers.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib events::` passed.
