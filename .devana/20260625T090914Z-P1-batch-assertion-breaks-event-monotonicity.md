DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: open
Location: src/simulator.rs:180 | Slug: batch-assertion-breaks-event-monotonicity

# Batch assertion checkpoints break raw event-stream monotonicity

## Finding

`run_batch_with_assertions_internal` emits all per-run events via `emit_batch_events`, then appends assertion checkpoints with `run_id = "batch"`. The raw sink stream is therefore not monotonically ordered by `RunEventOrder`, because `compare_run_id` ranks `"batch"` before any `"run-{k}"` string, while emission places checkpoints after the last `run-*` events.

## Violated Invariant Or Contract

Spec 032 (`032-event-order-contract-hardening`): raw sink output must be monotonically non-decreasing in `(run_id, step, phase, ordinal)` without pre-sorting. `tests/pikmin_diagram.rs` enforces this for single-run assertion paths.

## Oracle

`events.windows(2).all(|w| w[0].order() <= w[1].order())` fails on `Simulator::run_batch_with_assertions_and_sink` with `runs >= 2`. `compare_run_id` at `src/events/mod.rs:292–298` uses string fallback when `parse_run_index` fails; `"batch"` does not parse as `run-{n}`.

## Counterexample

`runs = 2`, any compiled scenario, passing expectations, `VecEventSink`.

Last per-run event: `step_end` for `run-1` (parsed index 1). First checkpoint: `run_id = "batch"`, step 0. `compare_run_id("run-1", "batch")` is `Greater` because `"batch" < "run-1"` lexicographically, but the checkpoint is emitted after the `run-1` step_end.

## Why It Might Matter

Consumers that validate live streams without sorting (per spec 032) will reject batch assertion output. CI event-contract tests cover single-run paths only, so this regression class is undetected.

## Proof

**Cross-entry mismatch:** Single-run `run_with_assertions_and_sink` preserves monotonicity (tested in `simulator.rs` and `pikmin_diagram.rs`). Batch path emits `run-0…run-N` then `"batch"` checkpoints, inverting order keys at the boundary.

## Counterevidence Checked

`simulator_run_batch_with_assertions_emits_batch_checkpoints` checks event name presence only, not order. `write_run_artifacts` can sort events before persistence, but live sink consumers see the unsorted stream.

## Suggested Next Step

Use a `run_id` that sorts after all batch run ids (for example `run-batch` with a parsed index above N), or emit checkpoints before per-run replays, or extend `compare_run_id` to rank `"batch"` after `run-*` ids.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.

DEVANA-KEY: src/simulator.rs:180 | P1 | batch-assertion-breaks-event-monotonicity
DEVANA-SUMMARY: Status=open | P1 high src/simulator.rs:180 - Batch assertion checkpoints use run_id "batch" that sorts before run-* ids, breaking raw stream monotonicity.