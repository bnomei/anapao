# Tasks — 016-semantic-rulebook-fixtures

Meta:
- Spec: 016-semantic-rulebook-fixtures — Machinations semantic rulebook and executable fixture catalog
- Depends on: -
- Global scope:
  - docs/machinations.md, docs/parity-rulebook.md, tests/fixtures/parity/**, tests/parity_rulebook.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 016-semantic-rulebook-fixtures (owner: worker:019c9027-89ae-7621-b242-eb3184344008) (scope: docs/machinations.md, docs/parity-rulebook.md, tests/fixtures/parity/**, tests/parity_rulebook.rs) (depends: -)
  - Context: Use docs/machinations.md as normative source and map each documented semantic rule to at least one fixture id with expected outcomes.
  - DoD: Extract normative rule tables from machinations research and create fixture-backed parity catalog scaffolding.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test parity_rulebook::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T01:31:00Z
  - Completed_at: 2026-02-24T01:36:00Z
  - Completion note: Added parity rulebook document, machine-readable fixture catalog, and parity catalog validation tests.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test parity_rulebook::` passed.
