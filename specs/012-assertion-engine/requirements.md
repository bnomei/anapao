# Requirements — 012-assertion-engine

## Goal
Expectation evaluation

## EARS
- WHEN implementation starts for 012-assertion-engine THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:006-step-engine, spec:010-stats-aggregator, spec:011-artifact-writer) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test assertions::".
