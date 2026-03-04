# Improvements PR Summary (W1.1-W1.12)

## Overview
This document summarizes Waves W1.1 through W1.12 that were integrated for the improvements stream and landed via PR #2. Scope covered API ergonomics, validation clarity/safety, statistical reporting, deterministic and failure-path test coverage, CI enforcement, integration closeout, and confidence-interval semantics correction.

## Wave-by-wave summary
- **W1.1** (`1a961a6`): Added fluent builders for `RunConfig` and `BatchConfig` to improve config ergonomics.
- **W1.2** (`7edecf2`): Added `ScenarioSpec` convenience constructors for simpler scenario setup.
- **W1.3** (`589ca89`): Improved validation errors with actionable reference hints.
- **W1.4** (`61071de`): Added compile-time graph/resource cycle detection.
- **W1.5** (`5d61963`, fix `bc0d83a`): Added streaming Welford-based summary moments and stabilized lint/test expectations.
- **W1.6** (`e8a9b3d`, compat fix `7867e00`): Made confidence level configurable while preserving struct-literal compatibility expectations.
- **W1.7** (`dc55be5`, fmt follow-up `4ef0396`): Expanded expression functions/operators and aligned formatting.
- **W1.8** (`3aa4190`): Added determinism/property and cache-reuse regression coverage.
- **W1.9** (`51bdc4b`): Added explicit failure-path batch/event integration coverage.
- **W1.10** (`3338321`): Added explicit CI coverage for parallel failure-path tests.
- **W1.11** (`22f4f9e`): Integrated approved work and aligned one validation expectation after integration.
- **W1.12** (`c66f832`): Fixed confidence interval field-label semantics and added explicit selected-confidence reporting in artifacts.

## Compatibility/Schema notes
- Most wave outputs are additive (new builders, constructors, tests, CI coverage, and expression/stat coverage).
- W1.6 retained compatibility for existing struct-literal expectations while introducing confidence-level configurability.
- **W1.12 semantics fix (explicit):**
  - `confidence_lower_95`, `confidence_upper_95`, and `confidence_margin_95` now remain strict 95% interval fields only.
  - Selected confidence metadata and selected-interval fields are additive (`selected_confidence_level`, `ci_selected_lower`, `ci_selected_upper`, `ci_selected_margin`).
  - Existing `*_95` / `ci95_*` consumers remain supported; downstream CSV consumers should parse by header names (not positional assumptions) for additive columns.

## Verification commands
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

## PR references
- PR: https://github.com/bnomei/anapao/pull/2
- Merge commit on `main`: `07676b2` (`Merge pull request #2 from bnomei/plan-anpao-improvements`).
- Key integration/fix commits:
  - `22f4f9e` (W1.11 integration follow-up alignment)
  - `3338321` (W1.10 CI coverage)
  - `c66f832` (W1.12 confidence semantics fix)
