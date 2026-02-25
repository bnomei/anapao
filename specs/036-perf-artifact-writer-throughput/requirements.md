# Requirements — 036-perf-artifact-writer-throughput

## Goal
Reduce artifact writer overhead and improve benchmark fidelity for I/O-heavy paths.

## EARS
- WHEN implementation starts for 036-perf-artifact-writer-throughput THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN run artifacts are written THE SYSTEM SHALL avoid avoidable clones and sorts while preserving deterministic output ordering.
- WHEN artifact benchmarks execute THE SYSTEM SHALL include an I/O-focused case that minimizes cleanup noise.
- IF artifact output format could change THEN THE SYSTEM SHALL preserve compatibility for all persisted files.
- WHEN performance comparison runs THE SYSTEM SHALL meet these criteria:
  - `artifact_write_expanded_capture_io_only`: at least 20% improvement
  - `artifact_write_expanded_capture` and `artifact_write_path`: no regression above +7%, target 10-15% improvement
- WHEN validation runs THE SYSTEM SHALL pass:
  - `cargo test`
  - `./scripts/bench-criterion compare --bench simulation --baseline hotspots-20260224-default`
  - `./benchmarks/run_profiles.sh`
