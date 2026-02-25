# Design — 008-events-contract

## Scope
src/events/**

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Implement stable event payloads and EventSink trait.
