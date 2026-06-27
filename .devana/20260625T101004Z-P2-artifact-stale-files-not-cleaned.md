DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: open
Location: src/artifact/mod.rs:202 | Slug: artifact-stale-files-not-cleaned

# Artifact pack written into a reused directory leaves stale files inconsistent with its manifest

## Finding

`ensure_output_dir` (src/artifact/mod.rs:202-205) only `create_dir_all`s the output directory; it never purges it. Every data file is written with `File::create` (truncate-overwrite of that exact name). The module never reads or removes the directory. So re-running an artifact write into a previously-populated directory replaces only the files whose names collide; files unique to the prior write survive on disk while the new `manifest.json` no longer references them. The directory becomes a union of two packs, while the manifest — documented as the authoritative pack index — under-declares what is present.

## Violated Invariant Or Contract

`manifest.json` is documented as the machine-readable index / entrypoint for the artifact pack (src/types/artifacts.rs). The implied invariant is that the files in the output directory equal the set declared in `manifest.artifacts` (plus `manifest.json`). Re-writing into the same directory breaks that equality.

## Oracle

After `write_X(dir, ...)`, the directory should contain exactly the files listed in the returned manifest's `artifacts` map. A second write into the same dir should leave no file from the first write that is not in the second manifest.

## Counterexample

Same-kind:
1. `write_run_artifacts_with_assertions(dir, run1, events1, Some(&assertions))` → writes `events.jsonl`, `variables.csv`, `history.json`, `replay.json`, `series.csv`, `assertions.json`, `manifest.json`; manifest has key `"assertions"`.
2. `write_run_artifacts(dir, run2, events2)` (same dir, no assertions) → overwrites everything except `assertions.json`; new manifest has no `"assertions"` key.
3. Result: `dir/assertions.json` is the stale run-1 report, still on disk but unreferenced.

Cross-kind (stronger):
1. `write_batch_artifacts(dir, batch)` → writes `prediction.json`, `summary.csv`, `series.csv`, `manifest.json`.
2. `write_run_artifacts(dir, run, events)` into the same dir → `series.csv` and `manifest.json` are overwritten with run semantics, but `prediction.json` and `summary.csv` (aggregate-batch files) remain — a run pack contaminated with leftover batch files.

## Why It Might Matter

A consumer that globs the directory (rather than strictly reading the manifest) ingests mixed/stale data from a previous run — wrong assertions, wrong batch summaries — silently. Reusing a fixed output path is a normal pattern (the crate's own doc example writes to a stable `temp_dir().join("anapao-doc-playbook")`).

## Proof

Control-flow: `ensure_output_dir` (src/artifact/mod.rs:202) does only `create_dir_all`; no `remove_file`/`remove_dir`/`read_dir` exists in the module. Per-file writers use `File::create` (truncate one named file). The run path conditionally emits `assertions.json` (src/artifact/mod.rs:134-140); the batch path emits a different file set (src/artifact/mod.rs:169-189). So the on-disk file set is a union across writes while `manifest.artifacts` reflects only the latest write.

## Counterevidence Checked

- Manifest is written last (src/artifact/mod.rs:145, 193), so a single failed write never leaves a manifest referencing missing data — but that does not address stale files from a *prior* run.
- No internal caller pre-cleans the dir (only the module, lib.rs doc, tests, benches reference these functions).
- The manifest carries no per-file counts or hashes (`ArtifactRef` holds only kind/path/content_type), so the inconsistency is silent — nothing surfaces the mismatch.
- Nothing in the API/docs forbids reusing an output directory; the functions accept an arbitrary `output_dir`.

## Suggested Next Step

In `ensure_output_dir` (or at the start of each `write_*` entry point), clear the target directory of known artifact files (or recreate it) before writing, so the resulting directory matches the manifest exactly.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.

DEVANA-KEY: src/artifact/mod.rs:202 | P2 | artifact-stale-files-not-cleaned
DEVANA-SUMMARY: Status=open | P2 high src/artifact/mod.rs:202 - ensure_output_dir only create_dir_all's, so re-writing a pack into a reused directory leaves stale files the new manifest no longer references.
