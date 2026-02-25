# Requirements — 000-program-control

## Goal
Program control and handoff state

## EARS
- WHEN implementation starts for 000-program-control THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies ( - ) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "test -f specs/index.md && test -f specs/_handoff.md".
