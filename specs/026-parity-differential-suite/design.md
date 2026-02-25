# Design — 026-parity-differential-suite

## Scope
tests/parity/**, src/testkit/mod.rs, tests/rstest_testkit.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Build parity confidence by encoding rule expectations as machine-verifiable golden fixtures and checks.

## Deliverable
Implement differential parity suite that validates each documented rule against deterministic fixture outputs.
