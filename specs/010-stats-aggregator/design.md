# Design — 010-stats-aggregator

## Scope
src/stats/**

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Implement n/mean/variance/stddev/min/max/quantiles.
