DEVANA-FINDING: v1
Priority: P3 | Confidence: medium | Security-sensitive: no | Status: open
Location: src/artifact/mod.rs:287 | Slug: series-csv-nonfinite-values

# series.csv (and variables.csv) emit non-reparseable `NaN`/`inf`/`-inf`

## Finding

`write_series_csv` formats every value with `format_f64(point.value)` (src/artifact/mod.rs:287 and the slow path at :310) with no finiteness guard. `format_f64` uses `ryu` (src/artifact/mod.rs:469-472), which renders `f64::NAN` → `"NaN"`, `f64::INFINITY` → `"inf"`, `f64::NEG_INFINITY` → `"-inf"`. The engine permits non-finite values into the series: `canonicalize_float` returns non-finite values unchanged (src/engine/mod.rs:2001-2004). So a non-finite metric value is written as a bare token in the numeric `value` column. `write_variable_csv` has the same unguarded `format_f64` (src/artifact/mod.rs:328, :341).

## Violated Invariant Or Contract

Values in a numeric CSV column should round-trip as parseable `f64` for downstream tooling. The stats/summary path deliberately filters non-finite samples before summarizing (src/stats/mod.rs), establishing that non-finite values are considered possible and must not flow to output as-is. series.csv applies no such guard — an in-crate asymmetry.

## Oracle

The stats path's explicit non-finite filtering (src/stats/mod.rs) versus the series path's lack of it. A consumer doing `field.parse::<f64>()` per row fails (or silently reinterprets) on `NaN`/`inf`/`-inf` tokens, unlike the well-formed numeric rows the tests expect (e.g. `alpha,2,2.0`).

## Counterexample

A metric/node value of `f64::INFINITY` (e.g. an accumulation overflow) for metric `flow` at step 3: `canonicalize_float(inf)` returns `inf`; `write_series_csv` emits the row `flow,3,inf`. A `0.0/0.0`-derived value yields `flow,3,NaN`. Neither is a guaranteed-reparseable numeric CSV token.

## Why It Might Matter

CI-friendly artifacts are a documented selling point; a series.csv containing `NaN`/`inf` breaks numeric ingestion by downstream tooling silently. Impact depends on how easily a non-finite value reaches a series point (the engine guards most transfer math), hence P3.

## Proof

Dataflow trace: engine `canonicalize_float` passes non-finite through (src/engine/mod.rs:2001-2004) -> `SeriesTable.points[].value` -> `write_series_csv` -> `format_f64` (src/artifact/mod.rs:287/310) -> `ryu` formats `NaN`/`inf`/`-inf`. Contrast with stats finite-filtering. `encode_csv_field` does not quote these (no comma/quote/newline), so they land as bare tokens.

## Counterevidence Checked

- The summary.csv path is largely protected because `summarize`/percentile/CI helpers reject non-finite inputs (src/stats/mod.rs), so this is specific to series.csv and variables.csv.
- The engine guards most transfer math against non-finite, lowering the likelihood of an engine-produced non-finite series value (P3, not higher); but artifact writers are public API accepting caller-supplied `RunReport`/`BatchReport`, so the path exists.
- `encode_csv_field` correctly handles separators/quotes; this is specifically a numeric-token reparse issue, not a quoting issue.

## Suggested Next Step

Decide a canonical non-finite representation for CSV output (reject, blank, or a sentinel) and apply it in `format_f64` or at the series/variable write sites, consistent with the stats path's treatment of non-finite values.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.

DEVANA-KEY: src/artifact/mod.rs:287 | P3 | series-csv-nonfinite-values
DEVANA-SUMMARY: Status=open | P3 medium src/artifact/mod.rs:287 - series.csv/variables.csv format values via ryu with no finite guard, emitting non-reparseable NaN/inf/-inf tokens in numeric columns.
