# Design — 011-artifact-writer

## Scope
src/artifact/**

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Write manifests/events/series/summaries to local files.
