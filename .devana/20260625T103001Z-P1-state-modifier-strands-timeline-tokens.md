DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/engine/mod.rs:1658 | Slug: state-modifier-strands-timeline-tokens

# State-modifier edge into a Delay/Queue node desyncs node value from timeline schedule

## Finding

`apply_state_connections` applies a Modifier state edge by mutating `state.node_values[to_index]` directly (line 1658), without routing the change through `TimelineRuntimeState::record_arrival`. When the target node is a Delay or Queue node, the injected tokens increase the node's physical value but never enter `delay_scheduled` / `queue_incoming`, so the timeline never schedules them for release. Because release is clamped by `release_budgets` (which is built only from the timeline schedule), those tokens can never leave the node — they are stranded for the rest of the run. A negative Modifier delta has the mirror problem: it lowers `node_values` while leaving already-scheduled tokens in `delay_scheduled`/`queue_ready`, so the schedule references tokens that no longer physically exist.

## Violated Invariant Or Contract

For Delay and Queue nodes the physical token count in `state.node_values[index]` must stay reconciled with the timeline bookkeeping (`delay_scheduled`/`delay_ready` for Delay, `queue_incoming`/`queue_ready` for Queue). Every token entering a timeline node must pass through `record_arrival`; every token leaving must pass through `record_release`. A Delay node must hold a received token for exactly `delay_steps` before it becomes eligible to leave, and total tokens leaving must equal total tokens entering.

## Oracle

The sanctioned mutators of timeline `node_values` all keep both sides in sync: `apply_transfer_plan` (src/engine/mod.rs:1406-1418) writes `node_values` and immediately calls `record_release`/`record_arrival`; `TimelineRuntimeState::from_compiled` (src/engine/mod.rs:284-309) seeds `delay_scheduled`/`queue_ready` from the initial `node_values`. Release eligibility is gated exclusively by `transfer_available_for_source` = `min(node_values, release_budgets)` (src/engine/mod.rs:368-386), and `release_budgets` is derived only from `delay_ready`/`queue_ready` in `begin_step` (src/engine/mod.rs:311-357). Validation places no node-kind restriction on a `StateConnectionTarget::Node` target (src/validation/mod.rs:479-487), so a Modifier edge may legally point at a Delay/Queue node.

## Counterexample

- Pool `P` (initial value > 0), Delay `D` (`delay_steps = 5`, initial 0), Sink `S`.
- Edge `P -> D`: State connection, `role = Modifier`, `target = Node`, formula yielding a positive delta (e.g. `+10`).
- Edge `D -> S`: Resource edge, PushAny.

On each step `apply_state_connections` adds ~10 directly to `node_values[D]` with no `record_arrival`, so `delay_scheduled[D]` stays empty. Every later `begin_step` finds `delay_ready[D] = 0`, so `release_budgets[D]` is absent and `transfer_available_for_source(D) = min(node_values[D], 0) = 0`. The tokens accumulate in `D` and can never transfer to `S` — stranded forever, breaking conservation.

## Why It Might Matter

Any scenario that uses a Modifier state edge to top up or drain a Delay/Queue node silently violates delay/queue semantics and token conservation: injected tokens are permanently trapped (or, with negative deltas, the schedule keeps phantom tokens that release without backing value). Final metrics, series, and assertions over those nodes become wrong, and the defect is deterministic so it corrupts parity baselines too.

## Proof

**State transition mismatch / dataflow trace:** `apply_state_connections` (src/engine/mod.rs:1648-1660) writes `state.node_values[to_index]` with no timeline call. `record_arrival`/`record_release` are invoked only from `apply_transfer_plan` (src/engine/mod.rs:1414/1417). `transfer_available_for_source` (368-386) returns `min(node_values, release_budgets)`; `release_budgets` is populated in `begin_step` (330-353) solely from `delay_ready`/`queue_ready`, which are fed only by `record_arrival` via `delay_scheduled`/`queue_incoming`. A direct `node_values` write therefore raises the physical value while leaving the budget at 0 → permanent strand.

## Counterevidence Checked

- Gate routing also reads raw `node_values`, but `gate_behavior_for_node` returns no sorting/mixed behavior for Delay/Queue, so timeline nodes never enter gate routing — not an additional vector and not a mitigation.
- `apply_source_generation` also writes raw `node_values`, but only for `NodeKind::Source`, which is never a timeline node.
- `from_compiled` reconciles initial `node_values` into the schedule, but only once at init; mid-run Modifier writes are not re-seeded.
- Validation (`validate_state_connection`, src/validation/mod.rs:479-487; `validate_delay_constraints`/`validate_queue_constraints`) imposes no node-kind guard on the Modifier target, so the edge compiles cleanly.

## Suggested Next Step

Either reject (at validation) a `StateConnectionTarget::Node` Modifier edge whose target is a Delay/Queue node, or route Modifier deltas on timeline nodes through `record_arrival`/`record_release` so the schedule and physical value stay reconciled.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed `apply_state_connections` writes `node_values[to_index]` directly with no timeline call, while `release_budgets` is derived only from `delay_ready`/`queue_ready` (fed by `record_arrival`), so a Modifier delta into a Delay/Queue node strands the injected tokens (or, with negative deltas, leaves phantom scheduled tokens). Chose the report's option 1 (reject at validation) over option 2 (route through the timeline): `apply_state_connections` isn't even passed the `TimelineRuntimeState`, and reconciling negative deltas would require correctly un-scheduling future-dated tokens — high risk for a feature that is currently always-broken. Added a guard in `validate_state_connection_invariants` (the `StateConnectionTarget::Node` arm): a Modifier edge whose target node kind is `Delay` or `Queue` is rejected with `edges.<id>.connection.state.target` / "modifier state connections cannot target delay or queue nodes". Turns silent corruption into a clear compile error, consistent with the compile=validated contract. Full `cargo test` already green with the guard (no existing fixture relied on the broken behavior). Added regression test `compile_scenario_rejects_modifier_state_edge_targeting_timeline_node` covering both Delay and Queue targets.

DEVANA-KEY: src/engine/mod.rs:1658 | P1 | state-modifier-strands-timeline-tokens
DEVANA-SUMMARY: Status=fixed | P1 high src/engine/mod.rs:1658 - Modifier state edges mutated a Delay/Queue node's value directly without record_arrival, stranding injected tokens. Fixed by rejecting modifier-into-delay/queue edges at compile (validate_state_connection_invariants); regression test added.
