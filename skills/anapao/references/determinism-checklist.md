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

- `CaptureConfig::default` records step-zero/final snapshots and can increase assertion surface.
- `CaptureConfig::disabled` favors throughput but removes some snapshot evidence.
- `capture.every_n_steps` must remain `> 0`; invalid values should fail setup validation.
- Do not assert snapshots that are intentionally suppressed by capture settings.

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
