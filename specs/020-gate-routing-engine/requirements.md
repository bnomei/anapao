# Requirements — 020-gate-routing-engine

## Goal
Implement sorting/trigger/mixed gate routing

## EARS
- WHEN implementation starts for 020-gate-routing-engine THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:018-edge-state-connection-parity, spec:019-trigger-action-modes) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib stochastic::".
