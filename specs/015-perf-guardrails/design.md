# Design — 015-perf-guardrails

## Scope
benches/**, benchmarks/**

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Rebuild criterion benches for engine/batch/artifacts.
