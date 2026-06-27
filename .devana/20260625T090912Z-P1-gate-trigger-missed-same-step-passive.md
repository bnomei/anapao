DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/engine/mod.rs:829 | Slug: gate-trigger-missed-same-step-passive

# Gate-emitted triggers skip earlier passive targets in the same step

## Finding

`apply_edge_transfers` makes a single pass over `compiled.node_order`. When a `TriggerGate` or acting `MixedGate` appends trigger targets via `append_node_trigger_outputs`, passive nodes that appear earlier in `node_order` have already been skipped and are not revisited in that step.

## Violated Invariant Or Contract

When a gate fires and emits state triggers to passive nodes, those targets should be able to act in the same simulation step.

## Oracle

`run_single_passive_trigger_mode_fires_on_state_trigger` passes because pre-loop `collect_step_triggers` handles static triggers. No test covers a gate that dynamically triggers a passive node sorted before the gate in `node_order` (BTree-sorted `NodeId` keys).

## Counterexample

- `actor`: `Passive` pool (5), `PushAny` → `sink`.
- `gate`: `TriggerGate`, `Automatic`, state trigger edge `gate → actor`.
- `node_order`: `actor` before `gate` (alphabetical).
- `MaxSteps { steps: 1 }`.

Expected: `actor` transfers 1 to `sink`. Actual: `gate` appends `actor` to `triggers` after `actor` was already skipped because `controller_can_fire(Passive)` was false without prior trigger membership.

## Why It Might Matter

Trigger-gate wiring order becomes part of simulation semantics. Scenarios where the triggered actor precedes the gate alphabetically silently fail to fire, producing wrong end states and non-obvious determinism drift when node IDs change.

## Proof

**Control-flow trace:** `for node_id in &compiled.node_order` (829) → `actor` iteration: `controller_can_fire` false → `gate` iteration: acts → `append_node_trigger_outputs(gate, triggers)` (906) → no second pass for `actor`.

## Counterevidence Checked

`collect_step_triggers` (827) seeds triggers before the loop for static state connections, which is a separate path from gate-appended triggers. `TriggerMode::Enabling` and `Automatic` nodes are unaffected.

## Suggested Next Step

After gate trigger emission, either re-evaluate affected earlier controllers in the same step or run a second trigger-propagation pass before ending the step.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed `apply_edge_transfers` made a single forward pass over `node_order`, so a gate appending trigger outputs (`append_node_trigger_outputs`) could not activate a passive controller sorted earlier. Because triggers only grow within a step (monotonic, append-only), reworked the pass into a fixpoint loop: each iteration fires only control groups that are newly eligible and not yet settled (tracked by `(node_index, control)` in `settled_groups`), so no group transfers twice, but passive nodes triggered mid-step act on a later pass. The loop continues while a group settled OR the trigger set grew (the latter is needed for pure TriggerGates that emit without settling a group of their own). Gate trigger emission is idempotent (set insert), so re-emitting across passes is safe. Termination: each productive pass settles a new group or grows the trigger set, both finite. Added regression test `run_single_trigger_gate_fires_passive_target_sorted_before_gate` (the report's counterexample: actor sorts before a TriggerGate and now fires in-step). Full `cargo test` green (245 lib tests).

DEVANA-KEY: src/engine/mod.rs:829 | P1 | gate-trigger-missed-same-step-passive
DEVANA-SUMMARY: Status=fixed | P1 high src/engine/mod.rs:829 - TriggerGate outputs appended mid-step cannot activate passive nodes already visited earlier in node_order. Fixed by iterating apply_edge_transfers to a fixpoint over the monotonically-growing trigger set; regression test added.