DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/engine/mod.rs:72 | Slug: variable-source-validation-gap

# Invalid scenario variable sources pass compile and are silently dropped at runtime

## Finding

`compile_scenario` (src/validation/mod.rs) validates nodes, edges, metrics, end conditions, and connections, but never validates `spec.variables.sources`. The string `sources` does not appear anywhere in src/validation/mod.rs. At runtime, `VariableRuntimeState::refresh_all` (src/engine/mod.rs:70-76) samples each source via `sample_variable_source` (src/engine/mod.rs:453-462), which calls `sample_closed_interval`/`sample_from_list`/`sample_from_matrix` and applies `.ok()` to their `Result`. A structurally-invalid source therefore yields `None` and the variable is silently skipped — never inserted into the values map.

## Violated Invariant Or Contract

The crate documents compilation as producing a "validated executable model" (README) — the compile step is the validation boundary. The engine's samplers impose hard invariants on variable sources (`min <= max` for intervals; non-empty and all-finite for lists/matrix rows). Those invariants are neither enforced at compile nor surfaced as a runtime error: the errors are swallowed by `.ok()`.

## Oracle

Every constraint the engine enforces on a field should be either validated at compile (the documented contract) or surfaced as a runtime error — not silently swallowed. The samplers in src/stochastic/mod.rs return `Err` for these cases; `refresh_all` discards those errors.

## Counterexample

```rust
let mut spec = ScenarioSpec::source_sink(TransferSpec::Expression { formula: "roll".into() });
spec.variables.sources.insert(
    "roll".to_string(),
    VariableSourceSpec::RandomInterval { min: 6, max: 1 }, // reversed bounds
);
let compiled = Simulator::compile(spec).unwrap(); // PASSES despite invalid source
let report = Simulator::run(&compiled, &RunConfig::for_seed(1));
```

At runtime `sample_closed_interval(6, 1, rng)` returns `Err`, `.ok()` → `None` (src/engine/mod.rs:457), so `refresh_all` never inserts `"roll"`. The transfer expression `"roll"` then fails with `ExprError::UnknownVariable` — a confusing runtime expression error from what was actually an invalid config that should have been rejected at compile. Same class: `RandomList { values: [] }`, `RandomMatrix { values: [vec![]] }`, and list/matrix entries containing non-finite values.

## Why It Might Matter

A scenario the user believes is validated compiles cleanly, then either fails far downstream with a misleading error or silently runs with a missing variable (wrong, deterministic results). Both defeat the "compile = validated" contract and are hard to diagnose.

## Proof

Static dataflow trace: `compile_scenario` validator list (src/validation/mod.rs) contains no `variables.sources` check (confirmed by absence of `sources` in the file) -> engine `refresh_all` (src/engine/mod.rs:70-76) -> `sample_variable_source` (src/engine/mod.rs:453-462) applies `.ok()` to sampler `Result`s -> invalid source becomes `None` -> variable absent -> downstream `ExprError::UnknownVariable` (or silent absence if unreferenced). The sampler error conditions live in src/stochastic/mod.rs (`min <= max`, non-empty, finite).

## Counterevidence Checked

- Verified no other validation entry point inspects variable sources: `simulator.rs` routes only through `compile_scenario`, `validate_run_config`, `validate_batch_config`, none of which mention variables.
- Verified the samplers genuinely return `Err` for these inputs, and `Constant` correctly guards non-finite (`value.is_finite().then_some`).
- Verified the path is reachable under default `EveryStep` timing (and `RunStart`), so `refresh_all` runs.

## Suggested Next Step

Add a `validate_variable_sources` pass to `compile_scenario` that rejects `RandomInterval` with `min > max`, empty `RandomList`/`RandomMatrix` (or empty rows), and any non-finite values — mirroring the invariants the samplers already enforce — so the error is reported at compile rather than swallowed at run.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed `compile_scenario` never inspected `spec.variables.sources`; the engine samples them with `.ok()`, silently dropping invalid sources. Added a `validate_variable_sources` pass to `compile_scenario` (after `validate_node_invariants`) mirroring the exact sampler invariants in src/stochastic/mod.rs: `RandomInterval` requires min <= max; `RandomList` must be non-empty and all-finite; `RandomMatrix` must be non-empty, each row non-empty, all values finite; `Constant` must be finite. Errors are `SetupError::InvalidParameter` with `variables.sources.<name>...` paths, consistent with other validators. Added regression test `compile_scenario_rejects_invalid_variable_sources` covering reversed interval, empty list, non-finite list element, empty matrix, empty matrix row, non-finite matrix cell, and non-finite constant, plus a valid source compiling. Full `cargo test` green (no existing fixture relied on an invalid source).

DEVANA-KEY: src/engine/mod.rs:72 | P2 | variable-source-validation-gap
DEVANA-SUMMARY: Status=fixed | P2 high src/engine/mod.rs:72 - Variable sources were never validated at compile and invalid ones were silently dropped via .ok() in refresh_all. Fixed by adding validate_variable_sources to compile_scenario mirroring the sampler invariants; regression test added.
