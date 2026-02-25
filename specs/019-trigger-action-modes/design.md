# Design — 019-trigger-action-modes

## Scope
src/types/mod.rs, src/engine/mod.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Encode action/trigger semantics exactly as documented and ensure deterministic mode transitions.

## Deliverable
Implement passive/interactive/automatic/enabling trigger modes plus push/pull any/all transfer behavior.
