# Current-State Research Questions

1. Which module and symbols currently own `CompiledScenario`, scenario validation, single-run
   execution, and batch execution?
2. Which `CompiledScenario` fields are public, how are they derived, and which engine paths assume
   they remain synchronized with the source `ScenarioSpec`?
3. Which source-visible failure paths arise only when ordered IDs or indexes disagree with the
   scenario maps?
4. Which computations are performed during validation but discarded, and which run-invariant
   computations are repeated for every single run and every batch seed?
5. Which parts of engine state are genuinely per-run mutable state and therefore must not move
   into a shared immutable plan?
6. How does the Rayon batch path share `CompiledScenario`, and what thread-safety properties does
   the plan need?
7. Which public README examples, integration tests, testkit helpers, and benchmarks import raw
   modules or read `CompiledScenario` fields directly?
8. Which root and prelude exports currently form the documented facade, and is
   `CompiledScenario` nameable without the validation module?
9. What constructor and Serde paths exist for `ScenarioId`, `NodeId`, `EdgeId`, and `MetricKey`,
   and do their tests cover invalid deserialization?
10. What package version and compatibility policy constrain privatizing fields, modules, and
    functions?
11. Which deterministic, parity, parallel, benchmark, rustdoc, and all-feature checks already
    exercise the affected paths?
12. What do primary Rust, Cargo, Serde, and Rayon sources state about private invariant-bearing
    types, `Arc`, checked conversions, deserialization, and compatibility breaks?
