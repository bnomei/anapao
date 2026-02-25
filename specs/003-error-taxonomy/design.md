# Design — 003-error-taxonomy

## Scope
src/error.rs

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Implement SetupError/RunError/AssertionError/ArtifactError + root SimError.
