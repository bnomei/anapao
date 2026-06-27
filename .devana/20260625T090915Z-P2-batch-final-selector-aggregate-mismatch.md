DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/assertions/mod.rs:333 | Slug: batch-final-selector-aggregate-mismatch

# Batch MetricSelector::Final uses last aggregate step, not mean of run finals

## Finding

For single runs, `MetricSelector::Final` reads `run_report.final_metrics`. For batches, it reads `batch_report.aggregate_series[metric].points.last()`, which is the mean at the highest step key across runs, counting only runs that captured that step. This diverges from `evaluate_batch_probability_band`, which uses per-run `final_metrics`.

## Violated Invariant Or Contract

Batch scalar expectations with `MetricSelector::Final` should represent the terminal value users expect across Monte Carlo runs. The evidence context string `batch.aggregate_series.final` is internally consistent but conflicts with how batch probability bands and prediction summaries aggregate per-run finals.

## Oracle

`observe_batch_scalar` lines 333–340 vs `evaluate_batch_probability_band` lines 504–519. `aggregate_series` in `src/batch/mod.rs` averages per-step across runs; shorter runs do not contribute to later step averages.

## Counterexample

Two runs tracking `throughput` with default capture:
- Run 0: ends at step 10, `final_metrics[throughput] = 10`.
- Run 1: ends at step 3, `final_metrics[throughput] = 3`.

`aggregate_series[throughput].points.last()` at step 10 is `10.0` (one run). Mean of finals is `6.5`. `Expectation::Equals { selector: Final, expected: 6.5 }` fails while per-run semantics suggest it should pass.

## Why It Might Matter

Batch assertions on `Final` silently test time-series aggregation semantics instead of cross-run final-value aggregation, causing false failures or false passes when runs differ in length.

## Proof

**Contract mismatch:** `evaluate_batch_expectation` → `observe_batch_scalar(Final)` uses aggregate series tail; `evaluate_batch_probability_band` uses `run.final_metrics` for the same batch report type.

## Counterevidence Checked

`batch_scalar_selectors_cover_step_and_missing_metric_contexts` uses equal-length fixture runs where the distinction does not surface. README examples use single-run `Final` assertions only.

## Suggested Next Step

Document the aggregate-series semantics explicitly, or align batch `Final` with the mean/median of per-run `final_metrics` to match probability-band aggregation.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed `observe_batch_scalar(Final)` read `aggregate_series.last()` (the per-step mean at the highest captured step, counting only runs that reached it) while `evaluate_batch_probability_band` reads per-run `final_metrics`. Chose the "align" option over "document": batch `Final` now averages per-run `final_metrics` across runs (`sum / count`, mirroring the aggregate-series mean in batch/mod.rs:134; only runs that have the metric contribute, value is None/missing when none do). Evidence context changed `batch.aggregate_series.final` -> `batch.runs.final_metrics.mean`. Updated `batch_scalar_selectors_cover_step_and_missing_metric_contexts` for the new context. Updated `fixture_batch_report` so throughput appears in both aggregate_series and each run's final_metrics (constant 10.0, so the per-run mean equals the old aggregate tail) — this keeps `batch_probability_band_uses_per_run_final_metrics` valid and makes the fixture realistic (a tracked metric is normally present in both). Added regression test `batch_final_selector_averages_per_run_finals_across_uneven_run_lengths` (the report's counterexample: runs ending at steps 10 and 3 with finals 10 and 3 -> Final == 6.5, not the aggregate tail 10.0). Full `cargo test` green incl. doctests. Note: Step selector still reads aggregate_series (a per-step cross-run mean), which is the correct semantics for a specific step.

DEVANA-KEY: src/assertions/mod.rs:333 | P2 | batch-final-selector-aggregate-mismatch
DEVANA-SUMMARY: Status=fixed | P2 high src/assertions/mod.rs:333 - Batch Final selector read last aggregate-series point, not the mean of per-run final_metrics. Fixed by averaging per-run final_metrics (matching probability-band aggregation); regression test added.