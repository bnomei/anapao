# Requirements — 009-monte-carlo-runner

## Goal
Batch runner with optional rayon

## EARS
- WHEN implementation starts for 009-monte-carlo-runner THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:006-step-engine, spec:004-rng-policy, spec:008-events-contract) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test batch::".
