# Requirements — 005-setup-validation

## Goal
Compile and validate scenarios

## EARS
- WHEN implementation starts for 005-setup-validation THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:002-core-types, spec:003-error-taxonomy, spec:004-rng-policy) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test validation::".
