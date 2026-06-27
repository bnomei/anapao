DEVANA-FINDING: v1
Priority: P3 | Confidence: medium | Security-sensitive: no | Status: fixed
Location: src/assertions/mod.rs:415 | Slug: monotonic-nan-false-pass

# `MonotonicNonDecreasing` silently passes when a NaN sits between a decreasing pair

## Finding

The monotonic-non-decreasing assertion detects a violation only via the strict comparison `window[0].value > window[1].value` over `points.windows(2)` (src/assertions/mod.rs:415-421). Because every IEEE-754 ordered comparison with NaN is `false`, a NaN between two otherwise-decreasing points hides the decrease: each window touching the NaN compares `false`, `find_map` returns `None`, and the function falls through to the unconditional `passed: true` branch (src/assertions/mod.rs:443-449). The assertion infers "ok" from the absence of a detected decrease rather than from a positive non-decreasing check.

## Violated Invariant Or Contract

A `MonotonicNonDecreasing` expectation must FAIL whenever the series is not non-decreasing. The series `[5.0, NaN, 1.0]` is plainly not non-decreasing (5.0 → 1.0), so the expectation must report `passed: false`.

## Oracle

The semantics of the expectation itself (a non-decreasing series), and the scalar assertion paths in the same module (`Equals`, `Between`, `Approx`) which correctly evaluate to `false` on NaN. The monotonic path is the lone outlier that treats "no decrease detected" as success.

## Counterexample

`series[metric].points = [SeriesPoint::new(0, 5.0), SeriesPoint::new(1, f64::NAN), SeriesPoint::new(2, 1.0)]`.
Windows: `(5.0, NaN)` → `5.0 > NaN` is `false`; `(NaN, 1.0)` → `NaN > 1.0` is `false`. `find_map` returns `None` → returns `passed: true` with actual `"series is non-decreasing across 3 points"` — a false pass that contradicts the very values it inspected.

## Why It Might Matter

A correctness assertion silently reports success on a series that is not monotone, masking a real regression. Impact is bounded by how easily a NaN reaches a series point (see counterevidence), hence P3.

## Proof

Manual IEEE-754 reasoning over the two `windows(2)` comparisons at src/assertions/mod.rs:415-421: the only failure detector is `>`, NaN makes `>` false on both sides of the NaN, so no offending window is found and control reaches the unconditional `passed: true` at src/assertions/mod.rs:443.

## Counterevidence Checked

- The engine canonicalizes metric values via `canonicalize_float` (src/engine/mod.rs:2001), which returns non-finite values unchanged, and most transfer paths are finiteness-guarded, so an engine-produced NaN series point is uncommon — this lowers likelihood (P3, not higher).
- However, the assertion layer is public API (`evaluate_run_expectations`/`evaluate_batch_expectations`) accepting arbitrary `RunReport`/`BatchReport`, and nothing validates series finiteness before evaluation, so the path exists.
- For non-NaN inputs the strict `>` correctly allows equal values (proper non-decreasing semantics), so a fix that adds a finiteness/NaN check introduces no false-fail regression.

## Suggested Next Step

Detect non-finite points explicitly (fail, or surface a distinct error) and/or rewrite the check to a positive predicate (`window[1].value >= window[0].value` for all windows, failing when any window is not `>=` — which NaN fails). Decide intended handling of NaN in a series and encode it.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed the only violation detector was `window[0].value > window[1].value`, which IEEE-754 makes false on both sides of a NaN, so `[5.0, NaN, 1.0]` fell through to the unconditional `passed: true`. Chose the most consistent of the report's options: explicitly reject any non-finite point (NaN or inf, anywhere in the series) before the ordering check, rather than only the positive-predicate rewrite (which would leave inf handling position-dependent). A series with NaN/inf is not a well-defined monotonic series. The subsequent `>` decrease check is unchanged and now provably runs over finite values only, so equal/increasing finite series still pass with no false-fail regression. Failure surfaces a distinct `non-finite series value <v> at step <s>` message with step evidence. Added regression test `run_monotonic_fails_on_non_finite_series_value` (the report's counterexample). All assertions unit tests green.

DEVANA-KEY: src/assertions/mod.rs:415 | P3 | monotonic-nan-false-pass
DEVANA-SUMMARY: Status=fixed | P3 medium src/assertions/mod.rs:415 - MonotonicNonDecreasing detected decreases only via `>`, so a NaN between two decreasing points yielded a false pass. Fixed by explicitly failing on any non-finite series point before the ordering check; regression test added.
