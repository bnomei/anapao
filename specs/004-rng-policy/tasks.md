# Tasks — 004-rng-policy

Meta:
- Spec: 004-rng-policy — Deterministic RNG and seeding
- Depends on: spec:001-pivot-surface
- Global scope:
  - src/rng/**

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 004-rng-policy (owner: worker:019c8ff1-c68b-7770-a189-1974f5309b35) (scope: src/rng/**) (depends: spec:001-pivot-surface)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Implement ChaCha8 RNG helpers and run seed derivation.
  - Validation: cargo test rng::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:22:00Z
  - Completed_at: 2026-02-24T00:27:00Z
  - Completion note: Added deterministic seed derivation, run RNG constructors, and reusable draw helpers on top of `ChaCha8Rng`.
  - Validation result: `CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib rng::` passed.
