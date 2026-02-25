# Tasks — 025-debugger-and-history

Meta:
- Spec: 025-debugger-and-history — Rule violation debugger events and replay history
- Depends on: spec:008-events-contract, spec:011-artifact-writer, spec:019-trigger-action-modes
- Global scope:
  - src/events/mod.rs, src/artifact/mod.rs, src/error.rs, src/types/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 025-debugger-and-history (owner: worker:019c903c-8e81-70f2-8490-6a1996216add) (scope: src/events/mod.rs, src/artifact/mod.rs, src/error.rs, src/types/mod.rs) (depends: spec:008-events-contract, spec:011-artifact-writer, spec:019-trigger-action-modes)
  - Context: Capture execution anomalies and history metadata for deterministic replay and diagnostics.
  - DoD: Add structured violation/debug events and replay/history indexing artifacts.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib events:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib artifact::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T16:07:27Z
  - Completed_at: 2026-02-24T16:19:45Z
  - Completion note: Added typed debug/violation events, replay/history artifact indexes, and typed violation error surface for deterministic diagnostics.
  - Validation result: mayor recheck passed for events and artifact test suites.
