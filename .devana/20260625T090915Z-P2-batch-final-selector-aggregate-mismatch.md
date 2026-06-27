DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: open
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

DEVANA-KEY: src/assertions/mod.rs:333 | P2 | batch-final-selector-aggregate-mismatch
DEVANA-SUMMARY: Status=open | P2 high src/assertions/mod.rs:333 - Batch Final selector reads last aggregate-series point, not the mean of per-run final_metrics.