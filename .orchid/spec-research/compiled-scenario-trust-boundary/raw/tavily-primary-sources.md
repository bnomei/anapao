# Tavily Research: Primary Cargo, Rust API, and Serde Sources

Run date: 2026-07-11

Command:

```text
tvly research "Using only primary or official Rust ecosystem sources, establish the exact compatibility and API-design facts for a Rust crate release: (1) Cargo SemVer compatibility consequences of changing a public struct field to private and changing a public module or function to private; (2) Rust API Guidelines guidance on private fields, future-proofing, re-exports, and checked conversions with TryFrom; (3) official Serde patterns for deserializing validated newtypes through a checked conversion, including #[serde(try_from = \"String\")] or manual Deserialize. Provide direct URLs and distinguish what is explicitly stated from inference. Exclude blogs, Medium, forums, and Stack Overflow." --model mini --timeout 600
```

Result: success in 38.59 seconds with four sources. The CLI output below is normalized for line
wrapping while preserving the report's fact/inference distinctions.

## Cargo SemVer Compatibility

- Explicit source statement: Cargo's SemVer guide lists adding a private field to an otherwise
  public-field struct as a breaking change because downstream struct literals no longer compile.
- Report inference: changing an existing field from public to private is the symmetric removal of
  direct read/write/literal access and is therefore compatibility-breaking.
- Explicit source statement: renaming, moving, or removing public items is a breaking change.
- Report inference: changing a public module/function to private removes that downstream item path
  and falls under the same rule.

Source: https://doc.rust-lang.org/cargo/reference/semver.html

## Rust API Guidelines

- Explicit source statement: public fields are a strong representation commitment; private fields
  preserve room to enforce invariants and evolve implementation.
- Explicit source statement: `TryFrom` is the standard fallible conversion trait and
  `TryInto` should be obtained from it rather than implemented directly.
- Report inference: a stable root re-export can decouple a supported item path from a private
  implementation module; the API Guidelines source did not contain a dedicated re-export rule in
  the retrieved material.

Sources:

- https://rust-lang.github.io/api-guidelines/future-proofing.html
- https://rust-lang.github.io/api-guidelines/interoperability.html

## Serde Checked Conversion

- Explicit source statement: `#[serde(try_from = "FromType")]` deserializes `FromType` and then
  converts fallibly; the destination must implement `TryFrom<FromType>` and its error must
  implement `Display`.
- Report inference: `#[serde(try_from = "String")]` therefore routes a validated string newtype
  through its existing `TryFrom<String>` checks.
- Explicit source statement: `from`/`try_from` container attributes are alternatives to a custom
  `Deserialize` implementation. A manual implementation can deserialize a `String`, invoke the
  same checked conversion, and map its error when macro-generated attributes are awkward.

Source: https://serde.rs/container-attrs.html

## Consolidated Result

The primary-source run supported the frozen packet decisions to:

1. treat public-field/module removal as a 0.2 compatibility break;
2. make the plan representation private and expose behavior/read-only accessors;
3. use `TryFrom<ScenarioSpec>` as the checked compiled-scenario conversion;
4. route identifier deserialization through `TryFrom<String>` while keeping string serialization.
