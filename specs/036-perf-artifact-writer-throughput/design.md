# Design — 036-perf-artifact-writer-throughput

## Scope
- src/artifact/mod.rs
- benches/simulation.rs
- benchmarks/run_profiles.sh

## Approach
- Avoid unnecessary intermediate copies for ordered events/series/variable snapshots.
- Sort only when required by input order; otherwise stream existing deterministic order.
- Keep output schema unchanged (`manifest.json`, `events.jsonl`, `history.json`, `replay.json`, `series.csv`, `variables.csv`, etc.).
- Add `artifact_write_expanded_capture_io_only` benchmark case that reuses a stable output directory to isolate write throughput from tempdir teardown cost.

## Compatibility
- Preserve field order/content expectations used by existing tests.
- Keep existing end-to-end artifact benchmark IDs active for continuity.

## Deliverable
Artifact writer path with reduced allocation/sort overhead and an I/O-only benchmark for clearer profiling.
