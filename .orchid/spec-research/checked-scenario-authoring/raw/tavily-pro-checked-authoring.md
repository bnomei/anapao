# Tavily PRO report: checked Rust authoring APIs

Run date: 2026-07-11
Model: `pro`
Status: completed
Response time: 191.54 seconds
Request ID: `1a7cbfa7-25d6-4067-bf83-52ad0c59dbdb`

## Query

Research idiomatic production-ready Rust patterns for a public library whose serde DTOs contain
duplicated tag/payload state. The requested comparison covered stable wire DTOs versus checked
domain sums, `TryFrom`, path-aware errors, ordinary builders, duplicate insertion, map-key/ID
checks, `NonZeroU64`, `#[must_use]`, engine migration, semver, and tests.

## Returned synthesis

The report recommended a two-stage boundary: deserialize the established DTO first, then perform a
fallible conversion to invariant-preserving domain values. It found that this keeps serde layout
decisions independent from execution invariants and centralizes cross-field validation.

It recommended:

- keeping serde container attributes and legacy DTO shape stable;
- putting cross-field and map-key/embedded-ID validation in the post-deserialize conversion;
- using structured path-bearing errors for conversion failures;
- using a conventional builder with a final checked build instead of typestate when field order is
  not itself a semantic invariant;
- defining duplicate behavior explicitly and preferring an error over silent replacement for a
  checked authoring surface;
- applying `#[must_use]` to builder results and important consuming methods;
- making the engine accept only checked domain values so invalid tag/payload combinations and
  runtime defaults disappear;
- testing serde compatibility, every conversion error branch, duplicate policy, and the public API
  contract.

The report explicitly identified three evidence gaps: authoritative details for serde/nonzero
representation, concrete public-API/semver tooling, and a source prescribing the exact layer for
map-key/embedded-ID checks. Targeted official-source searches were run afterward and are recorded
in `tavily-official-source-topup.md`.

## Sources returned

1. <https://serde.rs/container-attrs.html>
2. <https://docs.rs/serde_path_to_error>
3. <https://docs.rs/thiserror>
4. <https://rust-lang.github.io/api-guidelines/interoperability.html>
5. <https://doc.rust-lang.org/reference/visibility-and-privacy.html>
6. <https://rust-lang.github.io/api-guidelines/type-safety.html>
7. <https://rust-lang.github.io/rfcs/1940-must-use-functions.html>
8. <https://doc.rust-lang.org/std/process/struct.Command.html>
9. <https://serde.rs/enum-representations.html>
10. <https://docs.rs/serde>

Lower-tier community and crate sources also appeared in the response. Design decisions in the
packet rely on the official sources above and repository evidence, not on community opinion.
