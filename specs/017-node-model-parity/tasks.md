# Tasks — 017-node-model-parity

Meta:
- Spec: 017-node-model-parity — Expand node model to full machinations primitive set
- Depends on: spec:016-semantic-rulebook-fixtures
- Global scope:
  - src/types/mod.rs, src/validation/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 017-node-model-parity (owner: worker:019c902b-e2df-72a2-8151-92d59cfe37bc) (scope: src/types/mod.rs, src/validation/mod.rs) (depends: spec:016-semantic-rulebook-fixtures)
  - Context: Mirror documented node semantics (including pool/drain constraints) while preserving deterministic behavior.
  - DoD: Add typed node families (Pool/Drain/Gate variants/Converter/Trader/Register/Delay/Queue) and validation invariants.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib types:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib validation::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T01:38:00Z
  - Completed_at: 2026-02-24T01:46:00Z
  - Completion note: Expanded node model to machinations-style families and added structural validation invariants for pool/converter/trader/trigger-gate/delay/queue semantics.
  - Validation result: local recheck passed for both types and validation test suites.
