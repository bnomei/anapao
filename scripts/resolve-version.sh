#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${RELEASE_TAG:-}" ]]; then
  if [[ "$RELEASE_TAG" == v* ]]; then
    version="${RELEASE_TAG#v}"
  else
    version="$RELEASE_TAG"
  fi
elif [[ "${GITHUB_REF_NAME:-}" == v* ]]; then
  version="${GITHUB_REF_NAME#v}"
else
  version=$(python - <<'PY'
import json
import subprocess

meta = json.loads(
    subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"])
)
print(meta["packages"][0]["version"])
PY
  )
fi

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  echo "version=${version}" >> "$GITHUB_OUTPUT"
else
  echo "$version"
fi
