# Intake: Explicit Capture And Retention Policy

## Raw starting finding (verbatim)

### 2. Replace capture boolean soup with an explicit retention policy

`CaptureConfig::disabled()` only clears the initial/final flags and inherits `every_n_steps = 1` ([config.rs](/Users/bnomei/PROJECTS/anpao/src/types/config.rs:60)). Every positive step therefore still captures ([engine/mod.rs](/Users/bnomei/PROJECTS/anpao/src/engine/mod.rs:2113)).

Worse, an empty node or metric selection means “all,” so the current type cannot represent “none” ([engine/mod.rs](/Users/bnomei/PROJECTS/anpao/src/engine/mod.rs:2038)). Benchmarks labelled capture-disabled consequently retain per-step data ([simulation.rs](/Users/bnomei/PROJECTS/anpao/benches/simulation.rs:516)). Batch execution then collects complete `RunReport`s, aggregates them, and discards most of their contents ([batch/mod.rs](/Users/bnomei/PROJECTS/anpao/src/batch/mod.rs:33)).

Use explicit states:

```rust
pub enum CaptureSchedule {
    None,
    Final,
    Every {
        stride: NonZeroU64,
        include_initial: bool,
        include_final: bool,
    },
}

pub enum Selection<T> {
    None,
    All,
    Only(BTreeSet<T>),
}
```

Separate batch aggregation from diagnostic retention. Each run can return a compact `BatchSample`; retain only requested aggregate points, then fold samples in run-index order to preserve floating-point determinism.

Migration:

1. Add a genuinely empty `CaptureConfig::none()`.

2. Introduce explicit schedule/selection types with legacy serde conversion.

3. Deprecate or redefine `disabled()` in `0.2`.

4. Benchmark throughput and peak memory before replacing full batch reports internally.

Tavily specifically confirmed enums plus `NonZeroU64` as the idiomatic way to remove sentinel meanings and impossible zero strides. [Rust type-safety guidelines](https://rust-lang.github.io/api-guidelines/type-safety.html), [`NonZeroU64`](https://doc.rust-lang.org/std/num/type.NonZeroU64.html), [Serde enum representations](https://serde.rs/enum-representations.html).

Confidence: very high. This is both an API correction and likely the largest immediate performance win.

## Requested workflow

- Use a dedicated subagent for this finding.
- Run the complete `make-research` workflow, then compile a complete active spec with
  `make-specs`.
- Cover the solution end to end; an MVP, partial migration, or first slice as the final outcome is
  not acceptable.
- Preserve the starting finding above verbatim.
- Do not implement source changes during research/spec authoring.

## Success signals

- A public API can express no capture, final-only capture, periodic capture, all values, no values,
  and a concrete selection without sentinel overloads.
- `CaptureConfig::none()` produces no diagnostic snapshots, series, or transfer retention while
  keeping terminal results usable.
- Legacy serialized run and batch configurations continue to deserialize with their historical
  behavior; new serialization has one canonical shape.
- Batch aggregation no longer constructs full per-run reports that are immediately discarded.
- Single-thread and Rayon batch reports remain byte-for-byte equal apart from the intentional
  `execution_mode` field value.
- Throughput and peak live heap usage are baselined before the batch representation changes and
  compared after it.
- Public docs, examples, fixtures, assertions, artifacts, and feature-matrix tests describe and
  exercise the final semantics.
- Independent Sol/high review covers the public API/serde contract, performance evidence, and
  deterministic aggregation contract before completion.

## Constraints

- Rust edition 2021, MSRV 1.85, and `#![forbid(unsafe_code)]` remain unchanged.
- Deterministic ordering continues to use ordered collections at observable boundaries.
- `RunReport.final_node_values` and `RunReport.final_metrics` are terminal results, not optional
  diagnostics; capture policy must not remove them.
- Live `EventSink` delivery is a separate contract from report retention; disabling report capture
  must not silently suppress streamed events.
- `MetricSelector::Step`, series assertions, and batch aggregate artifacts may legitimately report
  missing data when the caller explicitly selects no series.
- Existing legacy JSON semantics, including empty legacy selection sets meaning all, must be read
  faithfully even though the new API removes that overload.
- The public field-shape change is targeted at the explicitly contemplated `0.2` breaking release.
- No worker may replace the deterministic run-index fold with a parallel floating-point reduction.

## Initial scope

- Capture and aggregation public types, constructors, builders, validation, and serde wire format.
- Single-run snapshot/series/variable/transfer retention and live-event separation.
- Private engine collector seam and private compact batch sample.
- Batch aggregation, per-run summaries, sequential/Rayon ordering, and fallback behavior.
- Assertion and artifact compatibility.
- Criterion throughput coverage plus a dedicated peak-live-heap measurement harness.
- README, rustdoc, testkit, local determinism guidance, and compatibility tests.

## Non-goals

- Changing simulation math, RNG seed derivation, end-condition behavior, or event ordering.
- Making `BatchSample` public or adding a second public batch-report schema.
- Removing terminal final-value maps from reports or per-run batch summaries.
- Parallel floating-point reduction, streaming aggregation over an unbounded reorder buffer, or a
  distributed batch executor.
- Replacing `BTreeMap`/`BTreeSet` at serialized or equality-observable boundaries.
- Implementing any of the sibling compiled-plan, checked-authoring, or macro findings.
