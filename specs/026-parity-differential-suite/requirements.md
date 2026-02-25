# Requirements — 026-parity-differential-suite

## Goal
Executable parity differential suite

## EARS
- WHEN implementation starts for 026-parity-differential-suite THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:016-semantic-rulebook-fixtures, spec:020-gate-routing-engine, spec:021-delay-queue-timeline, spec:023-variable-update-timing, spec:024-accuracy-indicator, spec:025-debugger-and-history) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test parity::".
