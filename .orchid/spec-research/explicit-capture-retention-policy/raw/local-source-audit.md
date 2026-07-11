# Local Source Audit — 2026-07-11

Commands used (shell fallback because Frigg MCP was unavailable):

```text
rg -n "CaptureConfig|capture|RunReport|BatchReport|aggregate|Rayon|parallel|determin" src tests benches README.md Cargo.toml
nl -ba src/types/config.rs
nl -ba src/types/reports.rs
nl -ba src/engine/mod.rs (run core, transfer, and capture sections)
nl -ba src/batch/mod.rs
nl -ba src/validation/mod.rs (config validation/tests)
rg -n "CaptureConfig" --glob '*.rs' --glob '*.md' --glob '*.json' .
```

Observed ownership summary:

- Public config/wire: `src/types/config.rs`.
- Full report data: `src/types/reports.rs`.
- Step and transfer retention: `src/engine/mod.rs`.
- Full-report batch orchestration/fold: `src/batch/mod.rs`.
- Public façade/event separation: `src/simulator.rs`.
- Final-vs-series assertions: `src/assertions/mod.rs`.
- Empty-series/variable artifact consumers: `src/artifact/mod.rs`.
- Default/parallel determinism: `tests/perf_determinism.rs`.
- Misleading disabled benchmarks: `benches/simulation.rs`.

The detailed source-backed facts were promoted into `03-current-state.md`; workers should consume
the final spec/task Context rather than this raw audit.
