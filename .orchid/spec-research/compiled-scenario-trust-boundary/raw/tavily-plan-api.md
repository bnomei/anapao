# Tavily PRO Research: Immutable Compiled Plan API

Run date: 2026-07-11

Command:

```text
tvly research "For a public Rust library preparing a 0.2 breaking release, validate the idiomatic design for an invariant-bearing compiled execution plan currently exposing mutable public fields. Cover: private fields with read-only accessors; an opaque cheaply cloneable handle backed by Arc; whether Arc is appropriate when Rayon shares the immutable plan across parallel runs; compile-time retention of parsed expression ASTs and routing plans instead of rebuilding per run; TryFrom/checked constructors and custom Serde Deserialize for validated string newtypes so deserialization cannot bypass invariants; Cargo SemVer implications of privatizing public fields or public modules/functions; and whether execution-core modules should be private while a stable facade remains public. Prefer official Rust documentation, Rust API Guidelines, Cargo reference, Serde documentation, and primary sources. Distinguish hard requirements from design tradeoffs and include direct citations." --model pro --timeout 900
```

Initial sandbox result:

```text
Error: [Errno 8] nodename nor servname provided, or not known
```

The command was rerun with approved network access and completed successfully in 159.46 seconds.
The CLI output below has normalized line wrapping; its substantive report text and URLs are
preserved.

## Report Summary

- Make internal data fields private and expose read-only references/iterators as the public API;
  this preserves invariants and permits internal refactors.
- An opaque `Arc`-backed handle is appropriate when a plan must be cheaply cloned and shared across
  threads. `Arc` provides shared ownership and is `Send`/`Sync` when the contained type is.
- Rayon can share an immutable `Arc` plan without locking, provided the plan has no non-`Sync`
  interior mutability. Locks are only needed around genuinely concurrent mutation.
- Enforce construction/deserialization invariants through fallible constructors and a checked
  conversion from an unchecked/proxy representation.
- Keep execution-core modules private with a stable public facade to minimize the compatibility
  surface.

## Field Visibility

The report distinguished the language rule from the design recommendation: non-public fields are
inaccessible outside their module, while private fields plus borrowed accessors are the idiomatic
way to preserve invariants. It noted that changing currently public fields to private will stop
downstream direct-field code from compiling.

## Arc and Rayon

The report found:

- cloning `Arc` increments a reference count rather than cloning the contained plan;
- `Rc` is unsuitable for cross-thread sharing;
- one `Arc` around the complete plan is preferable to many small `Arc` fields;
- immutable retained AST/routing data is simpler for concurrent reuse than mutable caches;
- atomic reference-count overhead is a design tradeoff, not a correctness requirement.

## Retained AST and Routing Data

The report characterized eager retention as a CPU/memory tradeoff. Reusing a compiled scenario
frequently supports eager immutable AST/routing retention; compact/flattened representations can
limit allocation overhead. This is a recommendation based on the repository reuse pattern, not a
Rust language requirement.

## Checked Deserialization

The report recommended deriving/deserializing an unchecked source value and converting through
`TryFrom`, either with a Serde `try_from` attribute or a manual `Deserialize` implementation. The
conversion error must surface through Serde's deserialization error path.

## Evidence Gap Reported by Tavily

This first report did not find sufficient Cargo-specific evidence for exact SemVer classification.
That gap triggered the separate primary-source-only Tavily run saved in
`raw/tavily-primary-sources.md`; the second run closed it.

## URLs Returned

Primary/official sources used by the packet:

1. https://doc.rust-lang.org/std/sync/struct.Arc.html
2. https://rust-lang.github.io/api-guidelines/about.html
3. https://doc.rust-lang.org/reference/visibility-and-privacy.html
4. https://docs.rs/rayon
5. https://serde.rs/field-attrs.html
6. https://rust-lang.github.io/api-guidelines/predictability.html

Additional sources returned by the broad research (retained for provenance, not used as normative
authority in the frozen decisions):

7. https://effective-rust.com/visibility.html
8. https://medium.com/@bhesaniyavatsal/you-dont-really-know-rust-until-you-understand-box-rc-and-arc-50a7342dcbda
9. https://notes.kodekloud.com/docs/Rust-Programming/Fearless-Concurrency/Send-and-Sync-traits/page
10. https://github.com/serde-rs/serde/issues/939
11. https://github.com/serde-rs/serde/issues/1587
12. https://rust-unofficial.github.io/patterns/rust-design-patterns.pdf
13. https://www.cs.cornell.edu/~asampson/blog/flattening.html
14. https://softwareengineering.stackexchange.com/questions/293981/mutable-ast-vs-different-immutable-asts
15. https://users.rust-lang.org/t/how-to-parse-a-type-with-invariants-with-serde/134118
