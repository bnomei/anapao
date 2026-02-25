# Tasks — 031-pikmin-fixture-refactor

Meta:
- Spec: 031-pikmin-fixture-refactor — Typed fixture builder for Pikmin scenario tests
- Depends on: spec:013-rstest-testkit, spec:006-step-engine
- Global scope:
  - src/testkit/**
  - tests/pikmin_diagram.rs

## In Progress

## Blocked

## Todo
- (none)

## Done
- [x] T001: Add typed Pikmin fixture builders and profile constructors in testkit (owner: mayor) (scope: src/testkit/**) (depends: spec:013-rstest-testkit)
  - Result: Added `src/testkit/pikmin.rs` with typed `PikminFixtureTuning`, `PikminFixtureProfile`, canonical node/metric id helpers, and scenario/compile builders for reusable fixture setup.
  - Validation: `cargo test testkit::` and `cargo test`

- [x] T002: Migrate Pikmin integration tests to typed fixture API (owner: mayor) (scope: tests/pikmin_diagram.rs) (depends: T001)
  - Result: Rewrote Pikmin integration tests to consume the typed fixture API and canonical ids/metric keys, removing duplicated inline positional scenario construction.
  - Validation: `cargo test --test pikmin_diagram` and `cargo test`

- [x] T003: Add fixture guardrail tests for invalid tuning and profile sanity (owner: mayor) (scope: src/testkit/**, tests/pikmin_diagram.rs) (depends: T001)
  - Result: Added fixture guardrail tests for invalid tuning inputs, profile sanity/distinctness, and balanced-profile compile expectations with tracked metric coverage.
  - Validation: `cargo test testkit::`, `cargo test --test pikmin_diagram`, and `cargo test`
