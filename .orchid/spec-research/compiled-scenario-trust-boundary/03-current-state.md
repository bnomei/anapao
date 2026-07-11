# Current State

## Package and Public Surface

- The package is `anapao` 0.1.1, Rust edition 2021, with MSRV 1.85
  (`Cargo.toml:1-7`). Its changelog states Semantic Versioning adherence
  (`CHANGELOG.md:3-8`).
- `artifact`, `assertions`, `batch`, `engine`, `error`, `events`, `expr`, `prelude`, `rng`,
  `simulator`, `stats`, `stochastic`, `testkit`, `types`, and `validation` are all public modules
  (`src/lib.rs:125-139`). The crate root re-exports `Simulator` and selected assertion, event, and
  DTO/report types, but not `CompiledScenario` (`src/lib.rs:144-150`).
- The prelude similarly re-exports `Simulator` and selected public types, but not
  `CompiledScenario` (`src/prelude.rs:6-12`).
- `Simulator` is documented as the stable compile/run/batch/assert facade
  (`src/simulator.rs:1-25`). `Simulator::compile` takes an owned `ScenarioSpec` but delegates to
  `validation::compile_scenario(&spec)` (`src/simulator.rs:27-35`). Run and batch facade methods
  validate configs and delegate to the engine and batch modules (`src/simulator.rs:42-78`,
  `src/simulator.rs:144-179`).

## Compiled Scenario Representation and Assumptions

- `CompiledScenario` lives in `validation` and derives `Debug`, `Clone`, and `PartialEq`. Its six
  fields are all public: a cloned `ScenarioSpec`, ordered node and edge vectors, node and edge index
  maps, and a metric-name index (`src/validation/mod.rs:20-32`).
- `compile_scenario` validates graph references, conditions, metrics, cycles, connection/node
  invariants, and variable sources before collecting node and edge order from `BTreeMap` keys and
  building indexes (`src/validation/mod.rs:39-101`). The returned source scenario is a clone of
  the input (`src/validation/mod.rs:94-101`).
- The deterministic-index test asserts public fields directly, including order, index maps, and
  source equality (`src/validation/mod.rs:850-892`).
- `init_state` iterates `compiled.node_order`, reads each node back from
  `compiled.scenario.nodes`, and calls `expect("compiled.node_order must reference known nodes")`
  (`src/engine/mod.rs:470-495`). Source generation repeats the same assumption and `expect`
  (`src/engine/mod.rs:704-723`). State-connection execution contains the analogous edge-order
  `expect` (`src/engine/mod.rs:1682-1688`).
- Other execution paths handle impossible compiled-plan misses with `continue`, `None`, zero, or
  default values. Examples include missing expression-cache edges
  (`src/engine/mod.rs:92-123`), node-mode/timeline defaults
  (`src/engine/mod.rs:1555-1661`), node/metric lookup fallbacks
  (`src/engine/mod.rs:1953-1983`), and capture-selection lookups
  (`src/engine/mod.rs:2086-2110`).
- `edge_index_by_id` is constructed and publicly tested but has no engine, batch, simulator, or
  integration-test consumer (`src/validation/mod.rs:83-87`, `src/validation/mod.rs:889-890`).

## Repeated Run-Invariant Work

- Validation parses resource transfer formulas and modifier state formulas through
  `validate_formula` (`src/validation/mod.rs:422-443`, `src/validation/mod.rs:464-482`).
  `validate_formula` creates a fresh `ExprRuntime`, compiles the text, maps the result to `()`, and
  discards the `CompiledExpr` (`src/validation/mod.rs:591-600`).
- `CompiledExpr` is an immutable crate-private AST wrapper, while `ExprRuntime` holds no mutable
  cache (`src/expr/mod.rs:34-42`). `ExprRuntime::compile` returns that AST and compiled evaluation
  methods accept shared references (`src/expr/mod.rs:44-89`).
- Each `run_single_internal` creates a new expression runtime, rebuilds
  `EngineExpressionCache::from_compiled`, and rebuilds `EngineStepPlan::from_compiled` before
  initializing per-run variable, gate, and timeline state (`src/engine/mod.rs:529-548`).
- `EngineExpressionCache::from_compiled` walks every ordered enabled edge, reparses transfer
  expressions, and reparses modifier-node state formulas (`src/engine/mod.rs:85-135`).
- `EngineStepPlan::from_compiled` walks every ordered enabled edge and rebuilds resource control
  groups plus trigger routing tables (`src/engine/mod.rs:746-809`).
- Variable values/RNG, gate RNG/balancers, timeline queues, engine values/metrics, captured-step
  state, and transfer logs are mutated independently per run
  (`src/engine/mod.rs:42-83`, `src/engine/mod.rs:137-228`, `src/engine/mod.rs:280-451`,
  `src/engine/mod.rs:538-548`).
- Batch execution passes the same `&CompiledScenario` into every sequential run and, with the
  `parallel` feature, every Rayon parallel iterator item (`src/batch/mod.rs:60-80`,
  `src/batch/mod.rs:87-97`). Therefore every seed repeats the expression-cache and step-plan
  construction performed by `run_single_internal`.

## Identifier Construction and Serde

- The identifier macro derives transparent `Serialize` and `Deserialize` on a private `String`
  field (`src/types/identifiers.rs:21-26`).
- `new()` separately rejects trimmed-empty strings and any control character
  (`src/types/identifiers.rs:28-39`). Existing `TryFrom<&str>` and `TryFrom<String>` implementations
  delegate to `new()` (`src/types/identifiers.rs:64-77`).
- The macro instantiates the same behavior for `ScenarioId`, `NodeId`, `EdgeId`, and `MetricKey`
  (`src/types/identifiers.rs:88-91`).
- Existing identifier tests cover invalid values passed to `new()` but do not attempt invalid
  Serde deserialization (`src/types/mod.rs:20-33`).

## Downstream Consumers of the Current Shape

- The README compile example and its executable/drift tests read `compiled.scenario.id` directly
  (`README.md:65-84`, `tests/readme_snippets.rs:20-27`,
  `tests/readme_snippets.rs:68-84`).
- The simulation benchmark imports `anapao::validation::CompiledScenario` and directly reads
  ordered node/edge fields in its checksum (`benches/simulation.rs:15-28`,
  `benches/simulation.rs:63-75`).
- `tests/perf_determinism.rs`, `tests/rstest_testkit.rs`, and
  `tests/parity/differential.rs` import raw engine/batch/validation functions rather than using only
  `Simulator` (`tests/perf_determinism.rs:1-12`, `tests/rstest_testkit.rs:1-10`,
  `tests/parity/differential.rs:1-17`).
- The Pikmin testkit test reads `compiled.scenario.tracked_metrics` directly
  (`src/testkit/pikmin.rs:373-383`).
- Existing Criterion groups include scenario compilation, single run, batch run, expression
  fanout, gate routing, state modifiers, and Rayon batch paths (`benches/simulation.rs:380-611`).

## External Evidence (2026-07-11)

- Tavily's primary-source-only pass found Cargo's compatibility guide classifies removing public
  items as breaking and treats loss of public field access as a compatibility break. It found the
  Rust API Guidelines warn that public fields commit representation/invariants, Serde documents
  `#[serde(try_from = "FromType")]`, and Rust API guidance assigns fallible conversion to
  `TryFrom`. Sources: Cargo SemVer, Rust API Guidelines future-proofing/interoperability, and Serde
  container attributes (`raw/tavily-primary-sources.md`).
- Tavily's broader plan/concurrency pass found `Arc<T>` provides cheap shared ownership and is
  `Send`/`Sync` when `T` is, while immutable plan data needs no lock; it also identified eager AST
  retention as a CPU/memory tradeoff rather than a language requirement. Primary sources in that
  report include the standard-library `Arc` docs, Rust visibility reference, Rayon docs, and Rust
  API Guidelines (`raw/tavily-plan-api.md`).
