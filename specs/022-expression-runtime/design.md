# Design — 022-expression-runtime

## Scope
src/expr/**, src/engine/mod.rs, src/types/mod.rs, src/lib.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Provide math-expression compatibility subset for test scenarios without introducing nondeterministic evaluation.

## Deliverable
Add expression runtime with whitelisted math functions and deterministic evaluation graph.
