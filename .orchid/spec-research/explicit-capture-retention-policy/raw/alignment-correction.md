# Alignment correction: performance evidence, not invented gates

The original finding requires matched pre/post Criterion throughput evidence and isolated DHAT
peak-live-heap evidence before accepting the compact batch representation. It does not authorize
hard numeric success gates.

The promoted research and spec therefore require named pre-change baselines, identical workloads,
consumed checksums, host/toolchain metadata, absolute and relative deltas, and patient
same-environment reruns for noisy results. `scripts/bench-capture-memory compare` fails only when
evidence is missing, invalid, or incomparable. The repository's existing non-failing 7% Criterion
summary may remain as optional descriptive context, never as a spec pass/fail gate.

Repeatable regressions or results contradicting the compact-allocation premise must be explained and
escalated for an explicit owner decision before completion. Workers must not invent or silently
change thresholds. Correctness and deterministic SingleThread/Rayon behavior remain hard gates.
