# Design — 001-pivot-surface

## Scope
Cargo.toml, README.md, src/lib.rs, src/main.rs, src/format/**, src/layout/**, src/mcp/**, src/model/**, src/ops/**, src/query/**, src/render/**, src/store/**, src/tui/**, src/ui.rs

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Reset crate metadata, dependencies, and remove legacy module trees.
