# Tavily Pro Research — Typed Rust API And Serde Migration

- Date: 2026-07-11
- Request id: `63fdbce4-e56b-4ce3-82ba-5bc1eaa9d842`
- Model: `pro`
- Response time: 162.62 seconds
- Status: completed

The research prompt requested primary-source validation for replacing boolean/sentinel capture
configuration with enums, `NonZeroU64`, explicit generic selection, dual-shape deserialization,
canonical serialization, pre-1.0 SemVer handling, deterministic Rayon aggregation, and performance
measurement.

Useful findings:

- Rust API Guidelines and standard-library `NonZeroU64` documentation support richer types and a
  non-zero stride invariant.
- Serde supports tagged enum representations, untagged compatibility intermediates, defaults, and
  custom deserialization; a typed legacy/current intermediate is an engineering application of
  those features.
- Cargo classifies public struct-field changes that break struct literals as breaking; moving the
  field migration to `0.2` is consistent with the crate's pre-1.0 compatibility boundary.
- Compact per-run samples and a run-index fold were correctly labeled engineering inference rather
  than requirements imposed by Rust or Serde.

Evidence gaps reported by Tavily:

- It did not obtain a sufficiently explicit primary statement for a blanket Rayon
  `collect::<Vec<_>>()` ordering guarantee.
- It did not obtain Criterion, floating-point, or peak-memory primary docs in this run.

Primary URLs returned and retained:

- https://rust-lang.github.io/api-guidelines/type-safety.html
- https://doc.rust-lang.org/std/num/type.NonZeroU64.html
- https://serde.rs/enum-representations.html
- https://serde.rs/examples.html
- https://doc.rust-lang.org/cargo/reference/semver.html

The gap drove an explicit local sort decision and a targeted follow-up rather than an unsupported
ordering claim.
