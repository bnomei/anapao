# Design — 016-semantic-rulebook-fixtures

## Scope
docs/machinations.md, docs/parity-rulebook.md, tests/fixtures/parity/**, tests/parity_rulebook.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Use docs/machinations.md as normative source and map each documented semantic rule to at least one fixture id with expected outcomes.

## Deliverable
Extract normative rule tables from machinations research and create fixture-backed parity catalog scaffolding.
