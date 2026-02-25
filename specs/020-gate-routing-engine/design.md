# Design — 020-gate-routing-engine

## Scope
src/engine/mod.rs, src/stochastic/mod.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Implement gate-specific conflict resolution and proportion handling with deterministic tie-breaking.

## Deliverable
Add deterministic routing engine for sorting/trigger/mixed gates including weighted and chance behavior.
