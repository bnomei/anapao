# Tasks — 032-event-order-contract-hardening

Meta:
- Spec: 032-event-order-contract-hardening — Raw-order event contract hardening
- Depends on: spec:008-events-contract, spec:030-live-event-streaming
- Global scope:
  - tests/pikmin_diagram.rs
  - tests/**

## In Progress

## Blocked

## Todo
- (none)

## Done
- [x] T001: Add raw-order monotonic assertion helper for emitted event vectors (owner: mayor) (scope: tests/**) (depends: spec:008-events-contract)
  - Result: Added raw adjacent-order monotonic assertions in simulator and Pikmin contract tests so checks operate on emitted stream order instead of sorted copies.
  - Validation: `cargo test --test pikmin_diagram`, `cargo test simulator::`, and `cargo test`

- [x] T002: Harden Pikmin event-contract test to enforce phase precedence and terminal checkpoint step (owner: mayor) (scope: tests/pikmin_diagram.rs) (depends: T001, spec:030-live-event-streaming)
  - Result: Strengthened contract checks to enforce per-step lifecycle boundaries, phase precedence, terminal-step checkpoint placement, and assertion-checkpoint ordering before terminal `step_end`.
  - Validation: `cargo test --test pikmin_diagram pikmin_diagram_event_contract_contains_core_phases_and_stable_ordering` and `cargo test`

- [x] T003: Add deterministic replay assertion for full event stream stability (owner: mayor) (scope: tests/pikmin_diagram.rs, tests/**) (depends: T002)
  - Result: Added fixed-seed replay test that serializes and compares complete emitted event streams byte-for-byte for deterministic stability.
  - Validation: `cargo test --test pikmin_diagram` and `cargo test`
