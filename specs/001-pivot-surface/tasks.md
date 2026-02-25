# Tasks — 001-pivot-surface

Meta:
- Spec: 001-pivot-surface — Pivot crate surface to testing utility
- Depends on: spec:000-program-control
- Global scope:
  - Cargo.toml, README.md, src/lib.rs, src/main.rs, src/format/**, src/layout/**, src/mcp/**, src/model/**, src/ops/**, src/query/**, src/render/**, src/store/**, src/tui/**, src/ui.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 001-pivot-surface (owner: worker:019c8fed-5857-77c2-811b-6f41c945a4b3) (scope: Cargo.toml, README.md, src/lib.rs, src/main.rs, src/format/**, src/layout/**, src/mcp/**, src/model/**, src/ops/**, src/query/**, src/render/**, src/store/**, src/tui/**, src/ui.rs) (depends: spec:000-program-control)
  - Context: This task is part of the hard pivot from diagram/TUI/MCP to deterministic simulation testing utility.
  - DoD: Reset crate metadata, dependencies, and remove legacy module trees.
  - Validation: cargo check
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T00:12:00Z
  - Completed_at: 2026-02-24T00:18:00Z
  - Completion note: Crate metadata and public surface were pivoted to library-first testing utility stubs and legacy module trees were removed.
  - Validation result: cargo check passed.
