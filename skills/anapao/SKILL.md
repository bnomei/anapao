---
name: anapao
description: Add and fix deterministic tests for anapao and explain which anapao API surface to use. Use when writing or updating tests that involve ScenarioSpec setup, Simulator compile/run paths, assertions, event sinks, artifacts, batch Monte Carlo behavior, parity fixture coverage, and deterministic seed/replay guarantees.
---

# Anapao

Write deterministic, evidence-rich tests for `anapao` from public crate surfaces.

## Load References Only As Needed

- Use `references/api-surfaces.md` to choose the right module and entrypoint.
- Use `references/test-recipes.md` to implement copy-ready test patterns.
- Use `references/parity-semantics.md` when changing parity fixture coverage.
- Use `references/determinism-checklist.md` before finalizing assertions.

## Core Workflow

1. Classify test intent: compile validation, run behavior, batch behavior, assertions, event contract, artifact schema, or parity semantics.
2. Prefer `anapao::testkit` fixtures first. Build custom `types::*` scenarios only when existing fixtures are insufficient.
3. Compile once with `Simulator::compile` and reuse `CompiledScenario`.
4. Execute with the smallest matching API:
   - `Simulator::run`
   - `Simulator::run_with_assertions`
   - `Simulator::run_batch`
   - `Simulator::run_batch_with_assertions`
5. Assert deterministic invariants first:
   - fixed seeds replay identically,
   - run indexes remain ordered and complete,
   - execution mode behavior is explicit.
6. Assert domain behavior second: metrics, node states, probabilities, parity outcomes, or schema sections.
7. Run focused tests while iterating, then run the full validation gates.

## API Surface Selection Guide

- `types`: build `ScenarioSpec`, `RunConfig`, `BatchConfig`, and inspect reports/manifests.
- `Simulator`: orchestrate compile/run/batch with optional assertions and sinks.
- `assertions`: express typed expectations and evaluate run or batch reports.
- `events`: inspect timeline phases, ordering, and sink behavior.
- `artifact`: write and read run/batch artifact packs and manifest compatibility.
- `stats`: derive prediction indicators and metric summaries for distribution guardrails.
- `testkit`: reuse deterministic fixtures, parity helpers, and fixture catalog loaders.
- `analysis` (feature-gated): convert reports into Polars frames for analysis pipelines.

## Test Authoring Rules

- Pin `RunConfig.seed` and `BatchConfig.base_seed` for deterministic tests.
- Prefer integration tests in `tests/*.rs`.
- Reuse `anapao::testkit` fixtures before adding new setup helpers.
- Validate event ordering whenever using `VecEventSink` or custom sinks.
- Use schema compatibility checks when touching `artifact` or manifest behavior.
- Keep parity updates synchronized across catalog declarations and differential mappings.
- Use probability bands or tolerances for stochastic checks instead of brittle exact-value assertions.

## Validation Commands

Run these before handoff:

```bash
cargo test --test <file>
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```
