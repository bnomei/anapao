# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Added checked scenario authoring through `ScenarioBuilder` and `CheckedScenario`, with
  deterministic duplicate handling, typed configuration builders, and checked conversion into the
  compiled simulation plan.
- Added the `scenario!` macro at the crate root and in the prelude, including compile-time grammar
  diagnostics, renamed-crate support, and a complete public syntax reference.

### Changed
- Completed the 0.2 typed capture-policy migration. Use `CaptureConfig::{none, final_only,
  default}` plus typed schedules/selections for per-run diagnostics, and `AggregationConfig` with
  `BatchConfig::with_aggregation` for batch metric sampling. Terminal node/metric results and live
  events remain available when diagnostic or aggregate series retention is disabled.
- Kept persisted configuration migration-compatible: legacy five-field capture JSON and nested
  `BatchRunTemplate.capture` payloads still read, reject zero legacy strides, and re-serialize only
  in the canonical typed shape. Rust struct-literal capture configuration is a 0.2 breaking change;
  deprecated `disabled()`/`with_capture` adapters are transition-only.
- Finalized the 0.2 compiled-scenario facade: compile with `Simulator::compile` (or
  `TryFrom<ScenarioSpec>`), run with `Simulator::run`/`Simulator::run_batch`, and inspect through
  `CompiledScenario::{scenario_id, source_spec, node_ids, edge_ids, node_count, edge_count}`.
- Removed public raw execution paths: replace `anapao::validation::compile_scenario`,
  `anapao::engine::run_single`, and `anapao::batch::run_batch` with the `Simulator` facade.

### Fixed
- Various Bugfixes: hardened identifier deserialization and ensure formula state targets reject
  unresolved edge references through the checked-builder diagnostic path.

## [0.1.0] - 2026-02-XX
- Initial Release
