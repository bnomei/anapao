# Requirements — 004-rng-policy

## Goal
Deterministic RNG and seeding

## EARS
- WHEN implementation starts for 004-rng-policy THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:001-pivot-surface) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test rng::".
