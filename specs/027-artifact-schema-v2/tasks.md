# Tasks — 027-artifact-schema-v2

Meta:
- Spec: 027-artifact-schema-v2 — Schema v2 artifacts with compatibility path
- Depends on: spec:011-artifact-writer, spec:024-accuracy-indicator, spec:025-debugger-and-history
- Global scope:
  - src/artifact/mod.rs, src/types/mod.rs, tests/artifact_schema_v2.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 027-artifact-schema-v2 (owner: worker:019c903c-8e81-70f2-8490-6a1996216add) (scope: src/artifact/mod.rs, src/types/mod.rs, tests/artifact_schema_v2.rs) (depends: spec:011-artifact-writer, spec:024-accuracy-indicator, spec:025-debugger-and-history)
  - Context: Version artifact outputs to support richer semantics while preserving CI replay stability.
  - DoD: Introduce artifact schema v2 including accuracy/debug/history sections and compatibility readers.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test artifact_schema_v2 && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib artifact::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T16:19:45Z
  - Completed_at: 2026-02-24T16:38:31Z
  - Completion note: Added schema-v2 manifest sections/versioning and compatibility readers with upgrade path for v1 payloads.
  - Validation result: mayor recheck passed for artifact schema v2 and artifact library tests.
