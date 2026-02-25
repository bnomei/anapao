# Design — 035-perf-gate-routing-and-batch

## Scope
- src/engine/mod.rs
- src/batch/mod.rs
- benches/simulation.rs

## Approach
- Refactor gate-routing lane runtime to index-based selection in hot loops.
- Remove repeated lane lookup scans by consuming selected lane index directly.
- Replace map-key-heavy deterministic balancer state with compact index-addressed scores.
- Remove unconditional `run_reports.sort_by_key` after batch execution and rely on stable collection order.

## Determinism and Compatibility
- Keep routing weight semantics unchanged for ratio/percentage/chance modes.
- Keep output run ordering identical to current observable behavior.
- Preserve fallback behavior when `parallel` feature is absent.

## Deliverable
Lower-allocation gate-routing and batch aggregation paths with parallel determinism validation and hotspot benchmark coverage.
