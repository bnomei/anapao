# Design

## Objective

Restore the checked-builder invariant that every non-node state target references an existing edge.

## Scope

Change only the formula branch of state-connection validation and the macro regression that
exercises it. Reuse the existing `required_target_edge` helper so the diagnostic shape, graph
location, and available-ID hint remain consistent with resource and state targets.

## Non-goals

- No macro grammar, expansion-hygiene, or macro-side registry changes.
- No new public API.
- No changes to resource or state target behavior beyond shared-helper reuse if required.

## Verification

The direct validation regression and the `scenario!` regression must both assert a missing-edge
`SetupError`. Independent review must confirm that formula targets use the same target-edge
resolution path as resource and state targets.
