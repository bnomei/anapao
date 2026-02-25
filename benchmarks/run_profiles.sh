#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PROFILE_SECONDS="${PROFILE_SECONDS:-30}"
PROFILE_FREQ="${PROFILE_FREQ:-100}"
PROFILE_DIR="${PROFILE_DIR:-benchmarks/profiles}"
BENCH_TARGET="${BENCH_TARGET:-simulation}"
BENCH_FEATURES="${BENCH_FEATURES:-}"
PROFILE_LABEL="${PROFILE_LABEL:-}"

mkdir -p "$PROFILE_DIR"

if [[ -z "$PROFILE_LABEL" ]]; then
  if [[ -n "$BENCH_FEATURES" ]]; then
    PROFILE_LABEL="features-${BENCH_FEATURES}"
  else
    PROFILE_LABEL="features-default"
  fi
fi
PROFILE_LABEL="$(printf '%s' "$PROFILE_LABEL" | tr ' ,/' '---' | tr -cd 'A-Za-z0-9._-')"

run_py() {
  if command -v python3 >/dev/null 2>&1; then
    python3 "$@"
    return 0
  fi
  if command -v python >/dev/null 2>&1; then
    python "$@"
    return 0
  fi
  if command -v uv >/dev/null 2>&1; then
    UV_CACHE_DIR="${UV_CACHE_DIR:-/tmp/anapao-uv-cache}" uv run python "$@"
    return 0
  fi
  echo "error: no python interpreter found (python3/python/uv)" >&2
  exit 127
}

profile_case() {
  local group="$1"
  local case_id="$2"

  local escaped_group="${group//./\\.}"
  local escaped_case_id="${case_id//./\\.}"
  local filter="^${escaped_group}/${escaped_case_id}$"

  local -a bench_cmd=(./scripts/bench-criterion run --bench "$BENCH_TARGET")
  if [[ -n "$BENCH_FEATURES" ]]; then
    bench_cmd+=(--features "$BENCH_FEATURES")
  fi

  echo "Profiling: bench=${BENCH_TARGET} features=${BENCH_FEATURES:-default} case=${filter}"
  PROFILE_FREQ="$PROFILE_FREQ" "${bench_cmd[@]}" -- --profile-time "$PROFILE_SECONDS" "$filter"

  local src="target/criterion/${group}/${case_id}/profile/flamegraph.svg"
  if [[ ! -f "$src" ]]; then
    echo "error: expected flamegraph not found: $src" >&2
    exit 2
  fi

  local out_svg="${PROFILE_DIR}/${group}__${case_id}__${PROFILE_LABEL}.svg"
  cp "$src" "$out_svg"

  local prefix="${out_svg%.svg}"
  run_py benchmarks/flamegraph_to_csv.py "$out_svg" --out-prefix "$prefix"
}

# Current hot-path cases.
profile_case "simulation.guardrails" "batch_run_expanded_semantics"
profile_case "simulation.guardrails" "artifact_write_path"
profile_case "simulation.hotspots" "single_run_expression_fanout"
profile_case "simulation.hotspots" "single_run_sorting_gate_routing"
profile_case "simulation.hotspots" "single_run_state_modifiers"
profile_case "simulation.hotspots" "batch_run_expression_fanout"
profile_case "simulation.hotspots" "artifact_write_expanded_capture"
profile_case "simulation.hotspots" "artifact_write_expanded_capture_io_only"

if [[ "$BENCH_FEATURES" == *parallel* ]]; then
  profile_case "simulation.guardrails" "batch_run_expanded_semantics_rayon"
  profile_case "simulation.hotspots" "batch_run_expression_fanout_rayon"
fi
