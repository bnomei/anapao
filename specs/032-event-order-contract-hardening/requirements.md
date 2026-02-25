# Requirements — 032-event-order-contract-hardening

## Goal
Harden event-contract tests so ordering regressions are detected directly from raw emitted event streams.

## EARS
- WHEN event-contract tests validate run ordering THE SYSTEM SHALL assert monotonic order on raw sink output without pre-sorting.
- WHEN validating per-step lifecycle THE SYSTEM SHALL enforce phase precedence (`step_start` before intermediate events, `step_end` terminal for that step).
- WHEN assertion checkpoints are emitted THE SYSTEM SHALL enforce that checkpoint step equals terminal executed step.
- WHEN fixed-seed deterministic runs are replayed THE SYSTEM SHALL produce byte-stable serialized event streams.
- IF ordering assumptions are violated THEN THE SYSTEM SHALL fail with actionable diagnostics showing first violating pair/context.
- WHEN validation runs THE SYSTEM SHALL pass: `cargo test --test pikmin_diagram`.
