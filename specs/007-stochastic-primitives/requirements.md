# Requirements — 007-stochastic-primitives

## Goal
Dice and distributions

## EARS
- WHEN implementation starts for 007-stochastic-primitives THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:004-rng-policy, spec:005-setup-validation) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test stochastic::".
