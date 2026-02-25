# Design — 014-analysis-polars-optional

## Scope
src/analysis/**, Cargo.toml

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Implement optional feature-gated shaping helpers.
