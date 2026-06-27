DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/engine/mod.rs:439 | Slug: queue-capacity-not-enforced

# Queue node capacity is validated at compile time but not enforced at runtime

## Finding

`validate_queue_constraints` rejects zero capacity and initial values above capacity, but `TimelineRuntimeState::record_arrival` for queue nodes accumulates into `queue_incoming` without reading `QueueNodeConfig.capacity`. No runtime path in `src/engine/mod.rs` references queue capacity.

## Violated Invariant Or Contract

When `QueueNodeConfig.capacity` is set, queued inventory should be bounded across steps.

## Oracle

`grep capacity src/engine/mod.rs` shows capacity only in test fixture defaults (`capacity: None`). `record_arrival` queue branch at lines 439–442 adds arbitrary amounts with no cap check.

## Counterexample

- Queue node with `capacity: Some(2)`, `release_per_step: 1`.
- Source pushes 5 units over 5 steps into the queue.

Expected: at most 2 units held in queue storage. Actual: all arrivals accumulate in `queue_incoming` without rejection or overflow handling.

## Why It Might Matter

Users configuring bounded queues get compile-time validation but unbounded runtime behavior, breaking scenarios that model finite buffers.

## Proof

**Dataflow trace:** resource transfer → `timeline.record_arrival` → `queue_incoming` increment with no capacity lookup from `node.config`.

## Counterevidence Checked

Validation covers static initial value vs capacity only. Engine tests use `capacity: None` exclusively. Pool capacity is validated at compile time for initial values; dynamic pool cap enforcement is a separate concern.

## Suggested Next Step

Read queue capacity in `record_arrival` and reject or clip excess arrivals, matching documented queue semantics.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed no runtime path read `QueueNodeConfig.capacity`. Rather than clip inside `record_arrival` (which only updates the release-scheduling bucket and does not know the committed amount), enforced at `apply_transfer_plan` — the single chokepoint all three transfer paths (any/all/gate) funnel through. Added `queue_capacity_for_node` and `accepted_queue_arrival`, which clip the moved amount to `capacity - held` when the target is a capacity-bounded queue (held == the queue's node value, since arrivals raise it and releases lower it; checked live so capacity freed by an earlier same-step release is reusable). The un-accepted remainder stays at the source (buffer backpressure), and `transferred_amount` in the transfer log reflects the clipped amount while `requested_amount` keeps the original ask. Non-queue targets and uncapped queues are unaffected (pool capacity is a separate report). Added regression test `run_single_queue_capacity_bounds_held_inventory` (capacity-2 queue fed by a source of 5 -> queue holds 2, source keeps 3). Full `cargo test` green.

DEVANA-KEY: src/engine/mod.rs:439 | P2 | queue-capacity-not-enforced
DEVANA-SUMMARY: Status=fixed | P2 high src/engine/mod.rs:439 - Queue capacity config was validated at compile time but never enforced at runtime. Fixed by clipping arrivals to remaining capacity in apply_transfer_plan; regression test added.