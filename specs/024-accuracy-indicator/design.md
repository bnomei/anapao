# Design — 024-accuracy-indicator

## Scope
src/stats/mod.rs, src/types/mod.rs, src/artifact/mod.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Translate machinations-style accuracy guidance into explicit, deterministic, testable metrics.

## Deliverable
Define and compute prediction accuracy/convergence indicators and expose them in reports/artifacts.
