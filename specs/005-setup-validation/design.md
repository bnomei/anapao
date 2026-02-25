# Design — 005-setup-validation

## Scope
src/validation/**

## Approach
- Keep edits inside write scope only.
- Prefer deterministic behavior and typed errors.
- Keep APIs focused on test utility workflows.

## Deliverable
Compile ScenarioSpec into CompiledScenario with fail-fast checks.
