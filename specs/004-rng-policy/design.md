# Design — 004-rng-policy

## Scope
src/rng/**

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Implement ChaCha8 RNG helpers and run seed derivation.
