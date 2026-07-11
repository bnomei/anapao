# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

## [0.1.0] - 2026-02-XX
- Initial Release
