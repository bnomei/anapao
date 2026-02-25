# Design — 021-delay-queue-timeline

## Scope
src/engine/mod.rs

## Approach
- Constrain edits to write scope.
- Preserve deterministic and reproducible execution guarantees.
- Encode semantics with fixture-driven tests wherever possible.

## Normative Context
Match documented delay/queue behavior (queue one-at-a-time, delay timing rules) with stable scheduling order.

## Deliverable
Add delay/queue scheduling and retry/defer semantics in the step timeline.
