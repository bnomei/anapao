DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: open
Location: src/engine/mod.rs:1204 | Slug: zero-fraction-aborts-gate-routing

# Zero-numerator fraction edge aborts entire gate routing group

## Finding

In `gate_routing_for_group`, when `gate_weight_for_edge` returns `None` for any outbound edge, the function immediately returns `Ok(None)` for the whole gate group. A `TransferSpec::Fraction { numerator: 0, denominator: N }` edge returns `None` at line 1268–1269, which aborts weighted per-token routing for all lanes instead of skipping that zero-weight lane.

## Violated Invariant Or Contract

A zero-probability gate lane should be ignored; remaining lanes should continue through probabilistic or deterministic gate routing.

## Oracle

`validate_resource_connection_invariants` rejects `denominator == 0` but allows `numerator: 0`. Runtime already skips lanes with `weight <= 0.0` at line 1207–1208, showing the intended pattern is lane skipping, not group abort.

## Counterexample

- `SortingGate` with 3 tokens and two outbound edges.
- Edge A: `Fraction { numerator: 70, denominator: 100 }`.
- Edge B: `Fraction { numerator: 0, denominator: 100 }`.

Expected: ~70% to edge A and ~30% implicit drop per token. Actual: `gate_weight_for_edge` returns `None` on edge B → `gate_routing_for_group` returns `None` → fallback to flat `apply_any_edge_group` once for the whole step, not per-token weighted dispatch.

## Why It Might Matter

Sorting and mixed gates with optional zero-weight outputs silently lose weighted routing semantics, changing stochastic distributions and breaking parity with Machinations-style percentage gates.

## Proof

**Control-flow trace:** `gate_routing_for_group` iterates edges → `gate_weight_for_edge(Fraction{0,N})` → `Ok(None)` → `return Ok(None)` at 1204–1205 before `weight <= 0.0` skip logic can apply to other lanes.

## Counterevidence Checked

`Fixed { amount: 0.0 }` is rejected at compile time. Non-fraction zero weights that return a finite weight of 0 are skipped correctly at 1207–1208. No engine test covers sorting gate with zero numerator fraction.

## Suggested Next Step

Treat `gate_weight_for_edge` returning `None` as “skip this edge” (continue loop) rather than aborting the entire gate group.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.

DEVANA-KEY: src/engine/mod.rs:1204 | P1 | zero-fraction-aborts-gate-routing
DEVANA-SUMMARY: Status=open | P1 high src/engine/mod.rs:1204 - A zero-numerator Fraction gate edge aborts all weighted routing instead of being skipped.