DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: open
Location: src/engine/mod.rs:1409 | Slug: pool-capacity-not-enforced

# Pool capacity is validated at compile time but never enforced at runtime

## Finding

`validate_node_invariants` rejects a Pool whose `initial_value` exceeds `config.capacity` (src/validation/mod.rs:620-625), establishing capacity as a hard upper bound on the pool's stored value. The engine never re-checks this bound when crediting resource transfers: `apply_transfer_plan` does `*value = canonicalize_float(*value + plan.transfer)` (src/engine/mod.rs:1409-1411) with no cap, and no other engine path reads `config.capacity`. A pool therefore freely grows past the capacity that validation treats as binding.

## Violated Invariant Or Contract

If `capacity` is a meaningful ceiling worth rejecting an over-capacity initial value for, a pool's value must never exceed `capacity` during a run; otherwise validation enforces a guarantee the engine breaks.

## Oracle

`validate_node_invariants` (src/validation/mod.rs:608-625) rejects `node.initial_value > capacity as f64` with reason `"must not exceed config.capacity (...)"`, and the test `compile_scenario_rejects_pool_initial_value_above_capacity` (src/validation/mod.rs:1239) locks this contract in. Capacity is asserted as a bound at setup but read nowhere in src/engine.

## Counterexample

- Pool `P` with `capacity: Some(10)`, `initial_value: 0`.
- Source feeding `P` with `Fixed { amount: 5 }`, Automatic trigger.

After 3 steps `P` holds 15 > capacity 10. The run completes successfully with no clamp and no error; `final_metrics`/series for `P` report 15.

## Why It Might Matter

Scenarios that model a bounded pool (inventory cap, buffer limit, max capacity) get silently wrong results once inflow exceeds capacity: the cap is advertised and validated but has no runtime effect. Downstream assertions, capacity-dependent gate logic, and parity baselines all diverge from the declared model.

## Proof

**Contract mismatch / control-flow trace:** `apply_edge_transfers` -> `apply_any_edge_group`/`apply_all_edge_group` -> `plan_edge_transfer_*` (no capacity read) -> `apply_transfer_plan` (src/engine/mod.rs:1409-1411) credits the target with `+ plan.transfer` unconditionally. A repo-wide search shows `capacity` is referenced in the engine only by `Vec::with_capacity`/tests — there is no target-side cap logic. Validation (src/validation/mod.rs:620) and runtime disagree about whether capacity bounds the stored value.

## Counterevidence Checked

- The only runtime caps are the timeline release-per-step budget (src/engine/mod.rs:349-353) and the from-value clamp `requested.min(available)` (src/engine/mod.rs:1739) — neither relates to target-side capacity.
- `allow_negative_start` and the negative-value floor (`.max(0.0)`) bound the low end only; there is no high-end clamp.
- Queue `capacity` has the same gap but is already tracked separately (queue-capacity-not-enforced); this report is specifically the Pool path with its own validation rule and credit site.

## Suggested Next Step

Decide whether `capacity` is a hard bound. If so, clamp the target credit in `apply_transfer_plan` (or reject the transfer) when it would exceed `config.capacity`; if not, drop the validation rule so setup and runtime agree.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.

DEVANA-KEY: src/engine/mod.rs:1409 | P2 | pool-capacity-not-enforced
DEVANA-SUMMARY: Status=open | P2 high src/engine/mod.rs:1409 - Pool capacity is validated as a ceiling at compile time but apply_transfer_plan credits the pool with no cap, so pool values freely exceed the validated capacity.
