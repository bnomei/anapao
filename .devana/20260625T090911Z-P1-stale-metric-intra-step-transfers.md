DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/engine/mod.rs:1881 | Slug: stale-metric-intra-step-transfers

# Stale tracked metrics during within-step transfers

## Finding

`metric_value` prefers the cached `state.metrics` map, but that map is only refreshed at the end of each step via `refresh_metrics`. When multiple edges in the same step use `TransferSpec::MetricScaled` (or gate weights derived from metrics), later transfers read the start-of-step metric value instead of the value after earlier transfers in the same step.

## Violated Invariant Or Contract

Metric-scaled transfer amounts and metric-derived gate weights should reflect the current simulation state at the moment each edge is evaluated within a step.

## Oracle

Engine tests cover metric-scaled transfers in isolation (`run_single` fixtures) but never two metric-scaled edges in one step that mutate the tracked node between evaluations. `transfer_request` at line 1684 calls `metric_value`, which short-circuits on `state.metrics` before live `node_values`.

## Counterexample

- Pool `source` (20), `sink-a` (4, tracked), `sink-b` (0).
- `edge-1`: `source → sink-a`, `MetricScaled { metric: sink-a, factor: 1 }`, `PushAny`.
- `edge-2`: `source → sink-b`, `MetricScaled { metric: sink-a, factor: 1 }`, `PushAny`.
- One step, `Automatic` trigger.

After `edge-1`, `sink-a` is 8. `edge-2` should request 8 but reads cached metric 4 and transfers 4 to `sink-b`.

## Why It Might Matter

Scenarios with multiple metric-scaled edges from the same controller in one step produce incorrect resource flows, breaking deterministic parity expectations and downstream assertions on final metrics.

## Proof

**Dataflow trace:** `apply_edge_transfers` → `apply_any_edge_group` (per edge) → `plan_edge_transfer_any` → `transfer_request` → `metric_value` returns stale `state.metrics` while `state.node_values` already changed by prior edge in same step. `refresh_metrics` only runs after the full step loop at line 623.

## Counterevidence Checked

End-of-step paths (`end_condition_met`, `capture_step`) call `refresh_metrics` first, so only intra-step transfer/gate evaluation is affected. `metric_value` falls back to `node_values` only when the metric is absent from `state.metrics`; tracked metrics always hit the cache.

## Suggested Next Step

Either refresh affected metrics before each edge evaluation, or change `metric_value` to read live node values for node-backed tracked metrics during step execution.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed `metric_value` short-circuited on the `state.metrics` cache, which is only refreshed end-of-step via `refresh_metrics` (called at the step loop after `apply_edge_transfers`). Reworked `metric_value` to compute live values mirroring `refresh_metrics`: node-backed metrics read `state.node_values[index]`, other tracked metrics read live `total_node_value`, and the cache is only a fallback for untracked keys. This fixes both `MetricScaled` transfers (line 1684) and metric-derived gate weights (line 1276) within a step. End-of-step reads are unaffected since `refresh_metrics` already ran, so live == cached there. Added regression test `run_single_metric_scaled_edges_observe_intra_step_updates` (the report's counterexample: sink-b receives 8, not stale 4). Full `cargo test` suite green.

DEVANA-KEY: src/engine/mod.rs:1881 | P1 | stale-metric-intra-step-transfers
DEVANA-SUMMARY: Status=fixed | P1 high src/engine/mod.rs:1881 - MetricScaled transfers in the same step read stale cached metrics instead of post-transfer node values. Fixed by making metric_value read live state (mirroring refresh_metrics); regression test added.