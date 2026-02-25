# Requirements — 010-stats-aggregator

## Goal
Statistical summaries

## EARS
- WHEN implementation starts for 010-stats-aggregator THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:009-monte-carlo-runner) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test stats::".
