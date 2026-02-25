# Tasks — 000-program-control

Meta:
- Spec: 000-program-control — Program control and handoff state
- Depends on:  - 
- Global scope:
  - specs/index.md, specs/_handoff.md

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 000-program-control (owner: mayor) (scope: specs/index.md, specs/_handoff.md) (depends: -)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Create and maintain orchestrator DAG and handoff.
  - Validation: test -f specs/index.md && test -f specs/_handoff.md
  - Escalate if: Required code edits are needed outside the listed scope.
  - Completed_at: 2026-02-24T00:10:00Z
  - Completion note: Program DAG and handoff buffers were created with all planned micro-specs and wave ordering.
  - Validation result: Verified specs/index.md and specs/_handoff.md both exist.
