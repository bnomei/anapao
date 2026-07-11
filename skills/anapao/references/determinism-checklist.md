# Determinism Checklist

Run this checklist before finalizing any `anapao` test change.

## Seed Policy

- Set `RunConfig.seed` explicitly for every deterministic single-run test.
- Set `BatchConfig.base_seed` explicitly for every batch test.
- When asserting per-run seeds, verify `derive_run_seed(base_seed, run_index)`.
- Re-run the same config twice and assert structural equality for replay stability.

## Execution Mode Expectations

- Use `ExecutionMode::SingleThread` as baseline determinism reference.
- If testing `ExecutionMode::Rayon`, assert expected behavior for both feature states:
  - with `parallel` feature enabled: parallel mode retained and results stable,
  - without `parallel`: fallback behavior remains deterministic and explicit.

## Event Ordering Expectations

- When using sinks, assert stream is monotonic by `RunEventOrder`.
- Assert `step_start` precedes intermediate phases and `step_end`.
- Assert assertion checkpoints occur on terminal step and in stable position relative to `step_end`.

## Capture Config Implications

- `CaptureConfig::default()` records step-zero/final diagnostics and can increase assertion surface.
- `CaptureConfig::none()` removes retained diagnostic snapshots/series, but terminal node/metric
  maps and live events remain available.
- Use `CaptureConfig::final_only()` or `CaptureSchedule::Every` with a positive `NonZeroU64`
  stride when step evidence is required; do not use deprecated `CaptureConfig::disabled()`.
- Final assertions remain valid without series. Step, monotonic, and series assertions must report
  missing evidence when the relevant capture/aggregation policy did not retain it.
- `AggregationConfig` controls batch aggregate metric series separately from per-run diagnostics.
- Do not assert snapshots or series that are intentionally suppressed by capture/aggregation.

## Batch Aggregation Ordering

- Normalize independently executed batch samples into complete ascending `run_index` order before
  constructing summaries or aggregating metric points.
- Fold floating-point aggregate sums sequentially in that run-index order. Do not perform a
  parallel `f64` reduction, even when execution uses Rayon.

## Common Flake Causes and Fixes

- Cause: unpinned seeds.
  - Fix: set explicit seeds and replay-check equality.
- Cause: exact-value assertions for stochastic outputs.
  - Fix: use `Expectation::Between`, `Expectation::Approx`, or `Expectation::ProbabilityBand`.
- Cause: parity catalog/mapping drift.
  - Fix: update catalog ordering and differential mapping together.
- Cause: event assertions coupled to unstable ordering assumptions.
  - Fix: assert contract phases and monotonic order keys, not incidental vector positions.
- Cause: cross-test state leakage.
  - Fix: rebuild scenarios/configs per test and avoid mutable global state.
