# Design — 027-artifact-schema-v2

## Scope
src/artifact/mod.rs, src/types/mod.rs, tests/artifact_schema_v2.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Version artifact outputs to support richer semantics while preserving CI replay stability.

## Deliverable
Introduce artifact schema v2 including accuracy/debug/history sections and compatibility readers.
