# Requirements — 008-events-contract

## Goal
Run event schema and sink

## EARS
- WHEN implementation starts for 008-events-contract THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:006-step-engine) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test events::".
