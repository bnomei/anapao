# Tasks — 018-edge-state-connection-parity

Meta:
- Spec: 018-edge-state-connection-parity — Add resource/state connection model parity
- Depends on: spec:016-semantic-rulebook-fixtures, spec:017-node-model-parity
- Global scope:
  - src/types/mod.rs, src/validation/mod.rs, src/engine/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 018-edge-state-connection-parity (owner: worker:019c903c-8e81-70f2-8490-6a1996216add) (scope: src/types/mod.rs, src/validation/mod.rs, src/engine/mod.rs) (depends: spec:016-semantic-rulebook-fixtures, spec:017-node-model-parity)
  - Context: Implement documented connection semantics: default formulas, filter matching, and trigger/modifier targeting constraints.
  - DoD: Introduce resource/state connection variants including activator/trigger/modifier/filter semantics with validation.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib types:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib validation:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T15:09:30Z
  - Recovery_note: Reassigned after previous worker session disappeared without report.
  - Reassigned_at: 2026-02-24T15:20:23Z
  - Completed_at: 2026-02-24T15:29:30Z
  - Completion note: Added typed resource/state connection model parity (activator/trigger/modifier/filter), validation invariants, and deterministic engine handling with token quantization and state-modifier next-step effects.
  - Validation result: mayor recheck passed for types/validation/engine suites.
