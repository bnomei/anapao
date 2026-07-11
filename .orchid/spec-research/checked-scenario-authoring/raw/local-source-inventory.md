# Local source inventory

Date: 2026-07-11
Repository: `/Users/bnomei/PROJECTS/anpao`
Discovery mode: shell fallback (`rg`, `nl`, `sed`) because Frigg MCP was unavailable.

## Commands used

```text
rg -n "pub (struct|enum) ...|TryFrom<ScenarioSpec>|NonZeroU64|must_use|with_" src tests README.md Cargo.toml
nl -ba src/types/scenario.rs | sed -n '1,620p'
nl -ba src/validation/mod.rs | sed -n '1,130p'
nl -ba src/validation/mod.rs | sed -n '380,610p'
nl -ba src/validation/mod.rs | sed -n '660,835p'
rg -n "NodeConfig::|connection.kind|target_connection|edge.id" src/engine/mod.rs
nl -ba src/engine/mod.rs | sed -n '1480,1810p'
nl -ba src/types/mod.rs | sed -n '80,330p'
nl -ba src/simulator.rs | sed -n '1,250p'
nl -ba src/lib.rs | sed -n '1,180p'
rg -n "ScenarioSpec|NodeConfig|ConnectionKind|StateConnectionTarget" tests specs
```

## Concrete anchors

- `src/types/scenario.rs:14-34`: `NodeKind` is one serde enum, including legacy aliases and
  `Custom(String)`.
- `src/types/scenario.rs:77-183`: family DTO configs expose public fields; delay steps, queue
  release, and queue capacity are plain `u64`/`Option<u64>`.
- `src/types/scenario.rs:185-201`: `NodeConfig` is a second, independently selectable tagged enum.
- `src/types/scenario.rs:256-344`: connection kind, both payload structs, and state target plus
  optional target ID are independently stored.
- `src/types/scenario.rs:353-425`: `NodeSpec` and `EdgeSpec` expose all fields publicly and their
  consuming `with_*` methods have no `#[must_use]` annotation.
- `src/types/scenario.rs:460-582`: `ScenarioSpec` owns public `BTreeMap` collections. `with_node`
  and `with_edge` insert by embedded ID and replace an existing entry with the same ID.
- `src/types/mod.rs:80-194`: unit tests pin missing node config defaults, missing edge connection
  defaults, omission of the default resource object on reserialization, legacy `target_edge` and
  `filter` aliases, default `+1`, and connection round trips.
- `src/types/mod.rs:256-279`: a builder test pins deterministic `BTreeMap` ordering, but there is no
  duplicate-rejection test.
- `src/validation/mod.rs:39-101`: compilation validates edge endpoints, then derives node/edge
  order and indexes from map keys; it does not compare keys with `node.id` or `edge.id` first.
- `src/validation/mod.rs:391-461`: connection validation rejects the inactive payload only when it
  differs from its default and rejects zero resource token size.
- `src/validation/mod.rs:498-559`: state target and optional target ID are reconciled by validation;
  missing IDs and node-plus-ID combinations remain representable before compile.
- `src/validation/mod.rs:677-833`: node validation matches on `NodeKind`; pool, delay, and queue
  helpers accept a different `NodeConfig` by substituting family defaults.
- `src/engine/mod.rs:1506-1521`: trigger target derivation drops a missing target ID to an empty
  target list.
- `src/engine/mod.rs:1590-1661`: delay, queue, capacity, and mode helpers re-match `NodeConfig` and
  return default behavior for absent or mismatched payloads.
- `src/engine/mod.rs:1757-1764` and `src/engine/mod.rs:1826-1832`: fraction and token execution retain
  zero guards despite compile validation.
- `src/engine/mod.rs:1771-1792`: expression cache lookup and error paths use embedded `edge.id`, not
  necessarily the `BTreeMap` key used to build order/indexes.
- `src/simulator.rs:27-35`: the stable facade consumes `ScenarioSpec` by value and returns
  `Result<CompiledScenario, SetupError>`.
- `src/types/mod.rs:7-17`, `src/lib.rs:125-150`, and `src/prelude.rs:6-11`: scenario DTOs are broadly
  re-exported; checked domain types do not exist.
- `README.md:42-84` and `tests/readme_snippets.rs:9-27`: the documented happy path builds a
  `ScenarioSpec` directly, compiles it, and inspects the old public compiled field.
- `Cargo.toml:1-43`: crate version is 0.1.1, MSRV is 1.85, serde is unconditional, and no builder or
  compile-fail helper crate is present.

## Dependency coordination fact

The 037 research owner supplied the frozen predecessor contract:

- folder: `specs/037-compiled-scenario-trust-boundary/`
- gate task: `037-compiled-scenario-trust-boundary/T004`
- after T004, `CompiledScenario` is an opaque `Clone` handle;
- construction remains `Simulator::compile(ScenarioSpec) -> Result<CompiledScenario, SetupError>`
  plus `TryFrom<ScenarioSpec, Error = SetupError>`;
- read-only accessors are `scenario_id`, `source_spec`, `node_ids`, `edge_ids`, `node_count`, and
  `edge_count`;
- engine, batch, and raw validation modules are private.

This packet treats that contract as a prerequisite rather than redefining it.
