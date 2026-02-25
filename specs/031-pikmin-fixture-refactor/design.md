# Design — 031-pikmin-fixture-refactor

## Scope
src/testkit/**, tests/pikmin_diagram.rs

## Approach
- Introduce a dedicated Pikmin fixture module in `testkit` with:
  - typed tuning struct,
  - deterministic default/profile constructors,
  - scenario build/compile helper functions.
- Keep fixture builders pure and deterministic.
- Replace inline scenario graph assembly in `tests/pikmin_diagram.rs` with reusable helpers.

## Model
- `PikminScenarioTuning`:
  - enemy fight throughput,
  - explore throughput,
  - ship-part chance,
  - optional future knobs (spawn/loss ranges).
- `PikminProfiles`:
  - named constructors for common test intent.
- `PikminIds` helper:
  - canonical node and metric id constructors.

## Migration Plan
1. Add helper module and typed profile constructors.
2. Port existing Pikmin tests to helper API without changing assertion intent.
3. Add focused unit tests for config/profile invariants.

## Deliverable
Fixture-first Pikmin scenario setup that is easier to reuse and safer to evolve for more complex scenario tests.
