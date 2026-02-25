# Design — 023-variable-update-timing

## Scope
src/engine/mod.rs, src/types/mod.rs, src/stochastic/mod.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Support timing-sensitive custom variable behavior aligned with documented prediction/play expectations.

## Deliverable
Implement variable update timing controls and list/matrix/random refresh semantics.
