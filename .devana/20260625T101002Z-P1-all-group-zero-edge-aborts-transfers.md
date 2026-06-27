DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: open
Location: src/engine/mod.rs:985 | Slug: all-group-zero-edge-aborts-transfers

# A single zero-request edge aborts an entire PullAll/PushAll transfer group

# Finding

In `apply_all_edge_group` (src/engine/mod.rs:954), each edge is planned with `plan_edge_transfer_all`. When that returns `None` — which happens for any edge whose resolved transfer is `<= 0.0` — the loop executes `return Ok(false)` (src/engine/mod.rs:985), discarding every already-planned sibling transfer in the group. The "Any" counterpart (`apply_any_edge_group`) instead uses `continue` (src/engine/mod.rs:944-946), correctly skipping a zero edge. So in an all-or-nothing group a single trivially-satisfiable zero-request edge silently suppresses all the other (non-zero, fully fundable) transfers.

## Violated Invariant Or Contract

PullAll/PushAll ("All") semantics are atomic over the amounts edges *request*: the group should fire all transfers when every edge's request can be funded, and abort only when a request that is genuinely `> 0` cannot be funded (the real availability check at src/engine/mod.rs:1000-1008 already enforces that). An edge requesting `0` is trivially satisfiable (`0 <= available`) and must not block siblings.

## Oracle

The sibling function `apply_any_edge_group` (src/engine/mod.rs:944) treats a `None`/zero plan with `continue`, so a zero edge does not stop other transfers. The "All" path diverges with an early `return Ok(false)`. The dedicated availability check (src/engine/mod.rs:1005) would correctly pass a 0 request, confirming "skip" — not "abort" — is the intended handling of zero.

## Counterexample

Node `A` (initial 10) in `ActionMode::PushAll` with two resource output edges in its group:
- `e1 = Fixed { amount: 2 }` -> `B`
- `e2 = Fraction { numerator: 0, denominator: 1 }` -> `C`  (or `e2 = MetricScaled { metric: m, factor: 1 }` where `m == 0` this step)

Step execution: `apply_all_edge_group` plans `e1` (transfer=2), then plans `e2`: `transfer_request` yields 0, `quantize_requested_amount(.,0)=0`, `plan_edge_transfer_all` returns `None` (src/engine/mod.rs:1383-1385), so line 985 returns `Ok(false)`. Result: neither `e1` nor `e2` fires; `B` stays 0 despite `A` holding ample balance for the only non-zero request. Expected: `e2` contributes nothing, `e1` transfers 2, `B`→2, `A`→8.

## Why It Might Matter

This silently produces wrong, persisted simulation results in a deterministic simulation library — the suppressed transfers never happen, so node balances, metrics, series, and artifacts are all wrong with no error. It is reachable from a validation-passing scenario.

## Proof

Control-flow / dataflow trace: `apply_edge_transfers` -> `apply_all_edge_group` (loop src/engine/mod.rs:969-998) -> `plan_edge_transfer_all` (src/engine/mod.rs:1357) returns `None` when `transfer <= 0.0` (src/engine/mod.rs:1383-1385) -> early `return Ok(false)` at src/engine/mod.rs:985 discards `plans` already collected. Contrast: `apply_any_edge_group` `continue` at src/engine/mod.rs:945.

## Counterevidence Checked

- Distinct from the known finding at src/engine/mod.rs:1204 (`zero-fraction-aborts-gate-routing`): that is `gate_routing_for_group` aborting *weighted gate routing*; this is the plain non-gate transfer function `apply_all_edge_group`. Different function and code path.
- `Fixed { amount: 0 }` is rejected by validation, so `Fixed` cannot trigger it — but `Fraction { numerator: 0 }` and `MetricScaled` with a currently-zero metric are not rejected (validation checks only `Fraction.denominator != 0` and applies no constraint to `MetricScaled`), keeping the bug reachable.
- Confirmed the per-source availability check (src/engine/mod.rs:1000-1008) would pass a 0 request, so skipping rather than aborting preserves all-or-nothing semantics for the non-zero requests.

## Suggested Next Step

In `apply_all_edge_group`, replace the `else { return Ok(false); }` for a `None` plan (src/engine/mod.rs:984-986) with `continue`, matching the "Any" path: a zero/None plan should be skipped, not treated as a group-level funding failure.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.

DEVANA-KEY: src/engine/mod.rs:985 | P1 | all-group-zero-edge-aborts-transfers
DEVANA-SUMMARY: Status=open | P1 high src/engine/mod.rs:985 - In apply_all_edge_group a zero-request edge (Fraction numerator=0 / MetricScaled zero metric) returns Ok(false), silently suppressing all sibling PullAll/PushAll transfers.
