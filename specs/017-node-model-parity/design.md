# Design — 017-node-model-parity

## Scope
src/types/mod.rs, src/validation/mod.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Mirror documented node semantics (including pool/drain constraints) while preserving deterministic behavior.

## Deliverable
Add typed node families (Pool/Drain/Gate variants/Converter/Trader/Register/Delay/Queue) and validation invariants.
