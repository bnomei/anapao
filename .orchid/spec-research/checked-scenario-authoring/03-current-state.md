# Current state: checked scenario authoring

## Public wire model

- `src/types/scenario.rs:14-34` defines serde-enabled `NodeKind` with 14 named families,
  three legacy aliases (`Process`, `Sink`, `Gate`), and `Custom(String)`.
- `src/types/scenario.rs:77-183` defines ten family config DTO structs. Their fields are public.
  `DelayNodeConfig.delay_steps`, `QueueNodeConfig.release_per_step`, and queue capacity are plain
  integer fields whose defaults are `1`, `1`, and `None` respectively.
- `src/types/scenario.rs:185-201` defines `NodeConfig` as a second public tagged enum with `None`
  plus ten configured-family variants. `NodeSpec` stores `NodeKind` and `NodeConfig` independently
  at `src/types/scenario.rs:353-364`.
- `src/types/scenario.rs:256-263` defines `ConnectionKind`. `EdgeConnectionConfig` stores that kind,
  a resource payload, and a state payload at `src/types/scenario.rs:335-344`.
- `src/types/scenario.rs:293-321` stores state target kind and `Option<EdgeId>` independently. The
  target field defaults to `Node`; `target_connection` accepts the legacy alias `target_edge`, and
  `resource_filter` accepts `filter`.
- `src/types/scenario.rs:393-404` embeds `EdgeConnectionConfig` in every `EdgeSpec`. The connection
  object is skipped during serialization when it equals the default resource configuration.
- `src/types/scenario.rs:460-473` exposes every `ScenarioSpec` field, including node and edge
  `BTreeMap`s, publicly. The type derives `Serialize` and `Deserialize` directly.

## Existing serde compatibility contract

- `src/types/mod.rs:80-93` proves a node payload that omits `config` deserializes as
  `NodeConfig::None`.
- `src/types/mod.rs:95-113` proves a typed pool config serializes and deserializes losslessly.
- `src/types/mod.rs:115-137` proves an edge that omits `connection` becomes the default resource
  connection and reserializes without adding a connection object.
- `src/types/mod.rs:139-167` proves state edges accept `target_edge` and `filter`, while omitted
  formula defaults to `+1`.
- `src/types/mod.rs:169-194` proves explicit connection semantics round-trip.
- `src/types/scenario.rs:29-33` and `src/types/mod.rs:36-58` preserve and test legacy node-kind wire
  spellings.
- `src/types/mod.rs:7-17` publicly re-exports all scenario DTOs through `anapao::types`; selected
  top-level and prelude exports are at `src/lib.rs:144-150` and `src/prelude.rs:6-11`.

## Authoring behavior

- `NodeSpec::new`, `EdgeSpec::new`, and `ScenarioSpec::new` construct DTO values directly at
  `src/types/scenario.rs:366-378`, `src/types/scenario.rs:406-418`, and
  `src/types/scenario.rs:475-490`.
- Consuming authoring methods at `src/types/scenario.rs:380-390`,
  `src/types/scenario.rs:420-424`, and `src/types/scenario.rs:551-582` return `Self` and do not carry
  `#[must_use]`.
- `ScenarioSpec::with_node` and `with_edge` use `BTreeMap::insert` keyed by the embedded ID. A
  repeated ID replaces the existing value. Their docs explicitly say “inserts or replaces.”
- `src/types/mod.rs:256-279` proves node and edge iteration is lexicographically deterministic
  because the maps are `BTreeMap`s. There is no test for a repeated checked-builder insertion
  because no separate checked builder exists.
- Callers can separately mutate a map key and its embedded ID through the public fields. No
  authoring API currently reconciles that state until compile.

## Compile validation and errors

- `Simulator::compile` accepts `ScenarioSpec` by value and returns
  `Result<CompiledScenario, SetupError>` at `src/simulator.rs:27-35`.
- `SetupError` has `InvalidGraphReference`, `CyclicGraph`, and `InvalidParameter` variants at
  `src/error.rs:17-26`. Parameter errors carry a string path (`name`) and reason; graph errors carry
  graph and reference strings.
- `src/validation/mod.rs:39-61` starts compilation by checking only edge endpoint references. It
  does not first compare `spec.nodes` keys with `NodeSpec.id` or `spec.edges` keys with
  `EdgeSpec.id`.
- `src/validation/mod.rs:75-92` derives deterministic orders and indexes from map keys.
- `src/validation/mod.rs:391-409` matches connection behavior by `ConnectionKind`.
  A resource edge is rejected when its state payload differs from `StateConnectionConfig::default`;
  a state edge is rejected when its resource payload differs from
  `ResourceConnectionConfig::default` (`src/validation/mod.rs:411-461`).
- `src/validation/mod.rs:498-559` reconciles `StateConnectionTarget` and the optional target ID.
  Node plus an ID is rejected, resource/state/formula targets require an ID, and resource/state
  targets additionally validate the referenced edge family.
- `src/validation/mod.rs:677-706` branches on `NodeKind`. It never performs a general
  `NodeKind`/`NodeConfig` family equality check.
- Pool, delay, and queue validation substitute defaults for every nonmatching config at
  `src/validation/mod.rs:709-713`, `src/validation/mod.rs:794-798`, and
  `src/validation/mod.rs:810-817`. A `Delay` node carrying `Queue` config can therefore pass these
  family-specific validators using default delay settings.
- Zero resource token size and zero fraction denominator are rejected at
  `src/validation/mod.rs:411-438`. Zero delay steps, queue release, and configured queue capacity
  are rejected at `src/validation/mod.rs:794-833`.
- Existing tests pin zero-denominator and zero delay/queue rejection at
  `src/validation/mod.rs:1302-1324` and `src/validation/mod.rs:2105-2160`.
- Resource-connection `TransferSpec::Expression` values are parsed during validation at
  `src/validation/mod.rs:440-442`. Modifier state formulas are parsed at
  `src/validation/mod.rs:473-482`; nonmodifier state control strings are not expression-parsed.
- `validate_formula` creates an `ExprRuntime`, returns `map(|_| ())`, and discards the immutable
  `CompiledExpr` at `src/validation/mod.rs:595-600`. Prior to spec 037, the engine reconstructs its
  expression cache and reparses transfer/modifier-node formulas at `src/engine/mod.rs:85-135`.
- `CompiledExpr` is crate-private, immutable, and cloneable (`src/expr/mod.rs:38-42`). Spec 037
  requires validation's returned AST to populate edge-index-aligned `CompiledExpressions` slots
  without a second parse.

## Engine dependence on unchecked shapes

- `src/engine/mod.rs:1506-1521` maps a state target to runtime triggers. A missing target ID for a
  resource/state target produces an empty target list rather than a typed impossible state.
- `src/engine/mod.rs:1590-1607` retrieves delay and queue timing by re-matching the node config and
  returns `1` for a missing node or wrong variant; both positive values are also clamped with
  `.max(1)`.
- `src/engine/mod.rs:1610-1619` derives capacity by matching pool or queue config and treats every
  other combination as unbounded.
- `src/engine/mod.rs:1645-1661` re-matches all configured node families to retrieve mode and treats
  `None` or register config as no mode.
- `src/engine/mod.rs:1682-1704` reads state kind/payload independently and skips missing compiled
  node references.
- `src/engine/mod.rs:1757-1764` guards zero fraction denominator by returning a zero request.
  `src/engine/mod.rs:1826-1832` clamps zero token size to one during quantization.
- Expression cache lookup uses embedded `edge.id` at `src/engine/mod.rs:1771-1792`, whereas edge
  ordering and lookup were built from the map key.

## Documentation and downstream seams

- `README.md:42-84` presents direct DTO construction as the main authoring path.
- `tests/readme_snippets.rs:9-27` compiles the README scenario and directly inspects the current
  compiled scenario field. Spec 037 owns that field-to-accessor migration.
- Integration and parity suites construct `ScenarioSpec` directly, including
  `tests/perf_determinism.rs`, `tests/parity/differential.rs`, `tests/pikmin_diagram.rs`, and
  `src/testkit/pikmin.rs`.
- `tests/parity/differential.rs:532-660` exercises positive-integer resource transfers, state
  default formula, and signed modifier behavior; these are existing behavior-parity seams.
- `Cargo.toml:40-43` has no compile-fail test dependency. Rustdoc compile-fail examples can enforce
  `#[must_use]` under `#![deny(unused_must_use)]` without adding one.

## Predecessor state

Spec 037 is the required predecessor. Its frozen contract places the opaque `CompiledScenario`,
`ExecutionPlan`, `CompiledNode`, `CompiledEdge`, expression cache, routing plan, and metric plan in
`src/plan.rs`. `src/validation/mod.rs` is the only execution-plan assembler. After
`037-compiled-scenario-trust-boundary/T004`, engine, batch, and raw validation modules are private;
`Simulator::compile(ScenarioSpec)` and `TryFrom<ScenarioSpec> for CompiledScenario` are preserved;
and source inspection occurs through `source_spec()`.

## External source facts

External research was run on 2026-07-11 and saved under `raw/`.

- The Rust API Guidelines describe custom types and enums as tools for making invalid states
  difficult or impossible to represent: <https://rust-lang.github.io/api-guidelines/type-safety.html>.
- Serde documents externally, internally, adjacently, and untagged enum wire representations, and
  container attributes control the selected representation:
  <https://serde.rs/enum-representations.html> and <https://serde.rs/container-attrs.html>.
- The standard library defines `TryFrom` as a fallible conversion trait:
  <https://doc.rust-lang.org/std/convert/trait.TryFrom.html>.
- `NonZeroU64` excludes zero at the type level, and serde publishes `Serialize`/`Deserialize`
  implementations for it: <https://doc.rust-lang.org/std/num/type.NonZeroU64.html>,
  <https://docs.rs/serde/latest/serde/trait.Serialize.html>, and
  <https://docs.rs/serde/latest/serde/trait.Deserialize.html>.
- The Rust Reference permits `#[must_use]` on a type or function/method and permits a custom
  message: <https://doc.rust-lang.org/reference/attributes/diagnostics.html>.
- `std::process::Command` is a standard mutation-style builder whose configuration methods return
  `&mut Command`: <https://doc.rust-lang.org/std/process/struct.Command.html>.
- Cargo's SemVer guide classifies adding a variant to an exhaustive public enum as a major change:
  <https://doc.rust-lang.org/cargo/reference/semver.html>.
