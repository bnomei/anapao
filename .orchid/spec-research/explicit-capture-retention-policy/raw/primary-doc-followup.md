# Primary Documentation Follow-up — 2026-07-11

Directly inspected primary/official sources after the Tavily reports identified gaps.

## Serde and Cargo

- Serde documents adjacently tagged enum shapes using `tag` plus `content`, and separately documents
  untagged enums: https://serde.rs/enum-representations.html
- Cargo documents that changing an all-public struct in a way that breaks external struct literals
  is a major/breaking change and recommends constructors/non-exhaustive design for future types:
  https://doc.rust-lang.org/cargo/reference/semver.html

## Rayon and floating operations

- Rayon 1.12 `IndexedParallelIterator` examples show indexed range operations collecting into
  predictable vectors, but the research did not elevate those examples into a broader guarantee:
  https://docs.rs/rayon/1.12.0/rayon/iter/trait.IndexedParallelIterator.html
- Rust `f64` docs demonstrate that changing the operation/rounding sequence can change a result
  (for example fused versus unfused multiply-add):
  https://doc.rust-lang.org/std/primitive.f64.html
- Engineering conclusion: normalize run order explicitly and preserve the current sequential
  addition order. This is a locally tested contract, not a claim that Rust guarantees cross-target
  byte identity for arbitrary floating algorithms.

## Criterion and peak live heap

- Criterion's official book describes `BenchmarkGroup::throughput` with bytes/elements per
  iteration and configurable statistical precision:
  https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html
- DHAT 0.3.3 documents a global allocator, isolated heap-usage tests, and `HeapStats` fields including
  `total_bytes`, `max_bytes`, and `curr_bytes`. It warns that the crate is experimental and that
  profiler global state makes ordinary parallel tests fragile:
  https://docs.rs/dhat/latest/dhat/
- Engineering conclusion: keep DHAT dev-only in a custom one-case-per-process bench target and use
  it for reproducible before/after high-water-mark evidence, not as ordinary unit-test machinery.
