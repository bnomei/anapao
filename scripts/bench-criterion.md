# Criterion baseline workflow (anapao)

Goal: keep deterministic benchmark baselines so simulation performance regressions are easy to detect after refactors.

## Helper script

Use `./scripts/bench-criterion` to standardize:
- `CARGO_HOME` (defaults to `/tmp/anapao-cargo-home`)
- bench target selection (`--bench simulation`)
- saving/comparing Criterion baselines (`save`, `compare`)

## Recommended workflow

1) Save a baseline on your reference branch/commit:

```sh
./scripts/bench-criterion save --bench simulation
# prints: baseline: <name>
```

2) Compare a branch/refactor against that baseline:

```sh
./scripts/bench-criterion compare --bench simulation --baseline <name-from-step-1>
```

3) Run without baseline operations when iterating:

```sh
./scripts/bench-criterion run --bench simulation
```

## Current benchmark groups

- `simulation.guardrails/*`
  - compile + representative run/batch/artifact paths kept stable for continuity.
- `simulation.hotspots/*`
  - expression fanout
  - sorting-gate routing
  - state modifier formulas
  - large topology compile
  - expanded artifact serialization

## Notes

- Baselines are local-only under `target/criterion/**/<baseline-name>/`.
- Run comparisons on the same machine and similar power/load conditions.
- You can pass additional Criterion arguments after `--` if needed.

## Profiling (flamegraphs)

Criterion profiling mode is wired to `pprof` and writes flamegraphs under
`target/criterion/<group>/<case>/profile/flamegraph.svg`.

Profile one case directly:

```sh
PROFILE_FREQ=100 \
  ./scripts/bench-criterion run --bench simulation -- --profile-time 30 \
  '^simulation\\.hotspots/single_run_expression_fanout$'
```

Generate the standard hot-path flamegraphs plus CSV/JSON summaries:

```sh
./benchmarks/run_profiles.sh
```

Generate flamegraphs for every benchmark case:

```sh
./benchmarks/run_profiles_all.sh
```
