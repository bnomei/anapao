# Local source inventory

Captured 2026-07-11 from `/Users/bnomei/PROJECTS/anpao` while specs 037-039 were being authored in
the shared worktree. No source file was edited.

## Commands

```text
git status --short --branch
rg --files src tests benches specs/039-checked-scenario-authoring \
  .orchid/spec-research/checked-scenario-authoring Cargo.toml README.md | sort
nl -ba src/types/scenario.rs
nl -ba src/types/identifiers.rs
nl -ba src/types/mod.rs
nl -ba src/error.rs
nl -ba src/lib.rs
nl -ba src/prelude.rs
nl -ba Cargo.toml
rg -n "macro_rules|macro_export|trybuild" src tests Cargo.toml
rg -n "end_conditions|EndConditionSpec::Any|EndConditionSpec::All" \
  src/validation/mod.rs src/engine/mod.rs tests README.md
rg -n "delay_steps|release_per_step|token_size|denominator" \
  src/validation/mod.rs tests
cat specs/039-checked-scenario-authoring/{requirements.md,design.md,spec.toml}
cat specs/039-checked-scenario-authoring/tasks/T00{1,2,3,4,5,6}.md
cat .orchid/spec-research/checked-scenario-authoring/05-implementation-shape.md
cat .orchid/spec-research/checked-scenario-authoring/07-make-specs-handoff.md
```

## Source handles

- `src/types/scenario.rs:14-201`: 15 node families, mode enums, and ten configured-family DTOs.
- `src/types/scenario.rs:203-225`: all transfer and end-condition variants.
- `src/types/scenario.rs:227-254`: variable timing, sources, and runtime config.
- `src/types/scenario.rs:256-344`: resource/state connection DTO vocabulary.
- `src/types/scenario.rs:353-425`: common node/edge fields and current DTO constructors.
- `src/types/scenario.rs:427-583`: complete scenario document fields and current authoring helpers.
- `src/types/identifiers.rs:21-91`: validated ID constructors plus panic-based fixture constructors.
- `src/error.rs:17-26`: public `SetupError` variants.
- `src/lib.rs:123-150` and `src/prelude.rs:1-12`: current module and facade exports.
- `Cargo.toml:1-47`: edition 2021, MSRV 1.85, and no current `trybuild` dependency.
- `src/validation/mod.rs:415-436`, `795-833`: established token, fraction, delay, and queue
  positive-value error paths.
- `src/engine/mod.rs:1985-2017`: top-level end conditions are OR; nested `Any` and `All` preserve
  explicit composition.
- `src/validation/mod.rs:249-300`: tracked and referenced metrics resolve to node IDs.
- `specs/039-checked-scenario-authoring/tasks/T002.md`: complete builder authoring categories.
- `specs/039-checked-scenario-authoring/tasks/T005.md`: final public re-export/docs implementation.
- `specs/039-checked-scenario-authoring/tasks/T006.md`: independent full-contract review and
  remediation gate.

## Search result

The macro search found only the private `define_identifier!` implementation macro in
`src/types/identifiers.rs`; the crate has no exported public macro and no `trybuild` UI harness.
