# Requirements — 006-step-engine

## Goal
Deterministic step engine

## EARS
- WHEN implementation starts for 006-step-engine THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:005-setup-validation, spec:007-stochastic-primitives) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test engine::".
