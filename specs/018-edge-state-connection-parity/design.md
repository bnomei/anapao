# Design — 018-edge-state-connection-parity

## Scope
src/types/mod.rs, src/validation/mod.rs, src/engine/mod.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Implement documented connection semantics: default formulas, filter matching, and trigger/modifier targeting constraints.

## Deliverable
Introduce resource/state connection variants including activator/trigger/modifier/filter semantics with validation.
