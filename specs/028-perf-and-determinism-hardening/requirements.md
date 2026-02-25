# Requirements — 028-perf-and-determinism-hardening

## Goal
Performance and determinism hardening for expanded semantics

## EARS
- WHEN implementation starts for 028-perf-and-determinism-hardening THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:020-gate-routing-engine, spec:021-delay-queue-timeline, spec:024-accuracy-indicator, spec:027-artifact-schema-v2) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test perf_determinism && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo bench --no-run".
