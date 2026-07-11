# Intake: checked scenario authoring

## Raw request

The project owner asked for a complete `make-research` packet followed by a dispatchable
`make-specs` ledger for the third idiomatic Rust finding. The result must cover the entire
solution, not an MVP or first slice, and must be validated with Tavily research.

### 3. Add checked authoring builders backed by sum types

Several domain choices are represented twice:

- `NodeKind` and an independent `NodeConfig` ([scenario.rs](/Users/bnomei/PROJECTS/anpao/src/types/scenario.rs:185)).

- `ConnectionKind` plus both resource and state payloads ([scenario.rs](/Users/bnomei/PROJECTS/anpao/src/types/scenario.rs:335)).

- `StateConnectionTarget` plus an optional target ID.

Mismatched node/config variants are not rejected; validation silently substitutes defaults ([validation/mod.rs](/Users/bnomei/PROJECTS/anpao/src/validation/mod.rs:709)). The engine repeats those fallbacks at runtime ([engine/mod.rs](/Users/bnomei/PROJECTS/anpao/src/engine/mod.rs:1590)).

Map keys can also disagree with embedded `NodeSpec.id` or `EdgeSpec.id`: compilation indexes map keys ([validation/mod.rs](/Users/bnomei/PROJECTS/anpao/src/validation/mod.rs:75)), while expression lookup later uses `edge.id` ([engine/mod.rs](/Users/bnomei/PROJECTS/anpao/src/engine/mod.rs:1771)).

Keep `ScenarioSpec` as the serde-compatible document, but add:

```rust
enum NodeBehavior {
    Source,
    Pool(PoolConfig),
    Delay(DelayConfig),
    Queue(QueueConfig),
}

enum ConnectionSpec {
    Resource(ResourceConnection),
    State(StateConnection),
}

enum StateTarget {
    Node,
    ResourceConnection(EdgeId),
    StateConnection(EdgeId),
    Formula(EdgeId),
}
```

Back these with a conventional checked `ScenarioBuilder`, family-specific constructors, `NonZeroU64` where zero is meaningless, and `TryFrom<ScenarioSpec>` into validated domain values. Avoid a typestate maze; the configuration remains data-oriented and serde-friendly.

Immediately add validation for key/ID mismatches, kind/config mismatches, and duplicate builder insertions. Add `#[must_use]` to consuming `with_*` methods.

Tavily’s research supports single enums for “exactly one choice,” ordinary builders for flexible configuration, and a separate DTO when serialized compatibility differs from internal invariants. [Rust type-safety guidelines](https://rust-lang.github.io/api-guidelines/type-safety.html), [Serde enum representations](https://serde.rs/enum-representations.html), [standard builder example](https://doc.rust-lang.org/std/process/struct.Command.html).

Confidence: high.

## Success signals

- Existing `ScenarioSpec`, `NodeSpec`, `EdgeSpec`, config DTOs, serde field names, defaults,
  aliases, and default-resource omission continue to decode and encode the established JSON shape.
- A new public checked authoring path makes node family/config, connection kind/payload, and state
  target/ID combinations unrepresentable after `build()` or `TryFrom` succeeds.
- `ScenarioSpec -> Scenario` conversion reports deterministic path-rich errors for map-key/ID
  drift, mismatched node config families, wrong connection payloads, and zero-only-invalid values.
- The checked builder rejects duplicate node and edge IDs without replacing the first value.
- Positive delay, queue, token-size, and fraction-denominator values are represented as
  `NonZeroU64` on the checked/execution side.
- The post-037 execution plan and engine consume the checked variants and contain no silent
  default, `.max(1)`, or missing-target fallback for these invariants.
- `ScenarioSpec -> Scenario` and `ScenarioBuilder::build` parse every active resource-transfer
  expression and every modifier-state expression exactly once, retain the returned crate-private
  ASTs inside the opaque checked scenario, and move them into the 037 plan without reparsing.
- Existing DTO compilation remains source compatible while an ergonomic checked-builder path is
  documented, re-exported, integration-tested, and usable by the later `scenario!` macro spec.
- Focused compatibility, conversion, duplicate, engine-parity, doctest, clippy, and full test gates
  pass, followed by an independent Sol/high public API and serde review.

## Constraints

- Rust edition 2021, MSRV 1.85, deterministic `BTreeMap`/`BTreeSet` ordering, max width 100.
- Preserve `Simulator::compile(ScenarioSpec) -> Result<CompiledScenario, SetupError>` and the
  opaque `CompiledScenario` facade established by spec 037.
- Preserve the wire DTO contract; stricter rejection applies at checked conversion/compile, not at
  raw serde parsing.
- Semantic JSON preservation is measured from `serde_json::to_value(&parsed_scenario_spec)`, not
  from raw fixture spelling; aliases, omitted fields, and defaults may normalize during the initial
  deserialize/serialize cycle before checked conversion begins.
- Do not require users to adopt typestate, procedural derives, or a macro to use the checked API.
- Do not re-publicize engine, validation, or execution-plan internals made private by spec 037.
- Use the existing `SetupError` taxonomy and path conventions unless implementation proves a
  matchable new error is necessary; workers must not invent a second overlapping error tree.
- No source implementation occurs during this research/spec-authoring task.

## Initial scope

- Wire DTO/domain boundary and conversions.
- Complete node-family and connection sum types, not only the four illustrative variants.
- Checked scenario/node/edge authoring ergonomics and duplicate policy.
- Key/embedded-ID and tag/payload validation.
- Nonzero checked values that currently have compile checks or runtime fallback.
- Facade and execution-plan integration after spec 037.
- Compatibility fixtures, behavior parity, compile-fail `must_use` coverage, docs, and migration.

## Non-goals

- Implementing `scenario!`; spec 040 owns the macro and consumes this builder contract.
- Replacing report DTOs, run/batch configuration, or capture policy.
- Changing deterministic ordering, simulation behavior for already-valid scenarios, or artifact
  schemas.
- Detecting repeated lexical keys in raw JSON before serde has constructed a `BTreeMap`; the
  requested duplicate guarantee is for checked builder insertions.
- Adding typestate, derive-builder crates, async execution, or a new serialization format.
