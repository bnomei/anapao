# Requirements — 011-artifact-writer

## Goal
Persist run artifacts

## EARS
- WHEN implementation starts for 011-artifact-writer THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:008-events-contract, spec:010-stats-aggregator, spec:003-error-taxonomy) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test artifact::".
