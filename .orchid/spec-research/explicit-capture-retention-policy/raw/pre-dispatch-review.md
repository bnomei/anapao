# Strong Pre-Dispatch Review — Spec 038

- Date: 2026-07-11
- Reviewer: `/root/research_capture_policy` (strong/high-reasoning authoring pass)
- Artifact: `specs/038-explicit-capture-retention-policy`
- Review result: GREEN

## Machine gates

```text
Research packet valid: /Users/bnomei/PROJECTS/anpao/.orchid/spec-research/explicit-capture-retention-policy
validated /Users/bnomei/PROJECTS/anpao/specs/038-explicit-capture-retention-policy: 5 tasks, 12 requirements
```

The task validator also passed with
`--forbid-reference .orchid/spec-research/explicit-capture-retention-policy`; no worker-facing file
depends on the broad packet.

## Semantic review

- All R001-R012 requirements have acceptance anchors and task coverage.
- Task scopes are contained by `global_scope`; every implementation task has explicit lowercase
  worker model/effort, dependencies, validation, and stop conditions.
- Intermediate states remain buildable: T001 keeps batch's existing field until T002; T002 keeps the
  full-report execution path only long enough to save an honest baseline; T003 removes that adapter
  and ships the final compact path.
- `CaptureConfig` has private fields/builders/accessors, non-exhaustive typed enums, four explicit
  diagnostic channels, and final/event independence. No hidden public worker choice remains.
- Legacy default, disabled-looking, selective, invalid-zero, and nested batch mappings are frozen;
  current serialization has one tagged shape.
- Batch validates concrete metrics once, executes private samples, explicitly sorts run indices, and
  folds `f64` sequentially. No parallel reduction or duplicated simulation loop is allowed.
- Performance proof is ordered correctly: fixed Criterion/DHAT cases and pre-change baselines land in
  T002; T003 compares identical inputs. The fixed DHAT `256x256` workload, checksums, host metadata,
  and thresholds make evidence reviewable.
- README, CHANGELOG Unreleased, rustdoc, exports, testkit, assertions, events, artifacts, benchmark
  docs, and local determinism guidance are represented before final review.
- T005 is a fresh Sol/high review-fix task with separate API/Serde, performance, and determinism
  lanes; it cannot be satisfied by earlier worker self-review.

## Fixups made during review

- Added every deprecated `CaptureConfig::disabled()` call-site surface to T001 so `-D warnings` can
  remain green after deprecation.
- Added `Cargo.lock` and pinned dev-only `dhat = "0.3.3"`; documented possible approved dependency
  network access.
- Added `CHANGELOG.md` Unreleased while excluding aggregate version bump/publish ownership.
- Fixed the peak-memory workload and baseline bridge semantics so pre-change transfer retention is
  genuinely measured.
- Required one-time batch metric-selection validation before sequential/Rayon fanout.
- Expanded T005 scope to every compatibility test surface touched by prior tasks.

## Dependency tooling exception

The concrete prerequisite is real: 038/T001 must consume 037/T004's verified plan/facade. The
current validator rejected the documented sibling-task frontmatter syntax with:

```text
error: T001: unknown dependency 037-compiled-scenario-trust-boundary/T004
```

The approved green fallback is internally consistent: spec-level dependency on 037,
`human_checkpoint = "before-implementation"`, and a hard T001 Context/Escalate guard. Before any
edit, that guard reads the sibling T004 task file and requires exactly `status = "done"` and
`verification_status = "passed"`; either absent predicate stops the task. Adding the concrete task
edge remains a later metadata action when sibling resolution is supported.

No semantic blocker or unresolved implementation choice remains.

OVERALL: GREEN
