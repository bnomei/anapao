# Design — 032-event-order-contract-hardening

## Scope
tests/pikmin_diagram.rs, tests/** (event-contract helpers)

## Approach
- Replace sort-then-assert pattern with raw-order assertions on sink output.
- Add helper assertions for:
  - monotonic `(run_id, step, phase, ordinal)` progression,
  - per-step phase boundary correctness,
  - assertion checkpoint placement at terminal step.
- Add deterministic replay check by serializing event vectors and comparing exact output across identical fixed-seed runs.

## Test Strategy
1. Keep current Pikmin event coverage scenario.
2. Add raw-order contract checks that fail on the first ordering violation.
3. Add deterministic replay assertion for full event stream equality.

## Dependency Note
This spec assumes streaming semantics from `030-live-event-streaming`. If 030 is not implemented yet, keep tasks blocked.

## Deliverable
Robust event-contract tests that verify real emitted order and detect subtle ordering regressions early.
