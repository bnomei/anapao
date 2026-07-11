# Shape Review

## Review scope

Strong semantic review of `05-implementation-shape.md` against the research packet, repository
contracts, and the `make-research` readiness rubric.

## Findings

### Ownership and paths

Green. Every existing owner and every proposed file is concrete and repository-relative. The shape
does not send workers to broad research paths or raw transcripts. New test, benchmark, and script
paths are named exactly.

### Public contract and migration

Green. The shape resolves the schedule, selection, all diagnostic channels, final-value boundary,
event-stream boundary, legacy input mapping, canonical output shape, deprecated constructor behavior,
and `0.2` source-compatibility break. Workers are not left to decide whether legacy empty sets mean
all or none, or whether an old disabled-looking payload should be reinterpreted.

### State and data flow

Green. The single state-transition loop has two private static collectors. Full report and compact
batch sample ownership are explicit. Transfer retention and live emission are separated. Batch data
flows from indexed compact samples through an explicit sort into a sequential ordered fold and then
the existing public summary/report shapes.

### Determinism and concurrency

Green. Seed derivation, run order, step averaging, fallback behavior, and floating-point operand
order are frozen. Rayon owns independent run execution only. The explicit run-index sort avoids a
hidden reliance on generalized iterator-order interpretation. Equality seams are named in unit and
integration tests.

### Performance proof

Green. Measurement tooling lands before the representation change, persists in production-quality
developer tooling, and covers both Criterion throughput and isolated DHAT peak live heap. Case
families, feature modes, checksums, host/toolchain metadata, named-baseline sequencing, and
absolute/relative delta reporting are specified. No invented percentage is a pass/fail gate; the
existing non-failing 7% Criterion summary is optional descriptive context only. The shape also notes
DHAT's experimental/global-state limitations and isolates it from ordinary tests.

### Compatibility consumers

Green. Assertions, artifacts, event sinks, public exports, README snippets, testkit, and local
determinism guidance are included. The shape explicitly protects final-value assertions when series
are disabled and treats missing step series as intentional caller-selected absence.

### Slice quality

Green. The slices are vertical and serial for factual reasons: the first is a complete run-policy
migration, the second captures a pre-refactor batch baseline, the third replaces the representation,
the fourth closes public/docs consumers, and the fifth independently validates/remediates. No slice
is an MVP final state, and no tracer/prototype wording survives into production artifacts.

### Cross-spec dependencies

Green. The sibling compiled-plan design was disclosed before freeze: 037/T004 finalizes the private
plan/accessor and façade/module-visibility contract that 038's collectors and selection validation
consume. The shape encodes the spec-level dependency and a hard T001 prerequisite. Because the
current task validator rejects existing sibling task IDs, the concrete task metadata edge is a
declared later tooling update guarded meanwhile by a before-implementation checkpoint and T001 stop
condition. That stop is concrete: before any edit, T001 reads the sibling T004 task file and requires
exactly `status = "done"` and `verification_status = "passed"`; absence of either predicate stops
dispatch. Checked-authoring and macro siblings remain independent.

## Residual risks routed into tasks

- Custom dual-shape Serde can accidentally accept mixed payloads; literal compatibility vectors and
  a fresh public-API review are mandatory.
- Collector extraction touches the central engine loop; identical state/event results and
  feature-matrix tests are mandatory.
- DHAT is experimental and global; the custom one-case-per-process bench target and independent
  performance review are mandatory.
- Bench results can be noisy; host metadata, existing Criterion confidence output, and patient
  same-environment reruns are required before drawing a conclusion. Repeatable regressions or
  evidence contradicting the compact-allocation premise require explanation and explicit owner
  decision before completion; workers must not invent or silently change thresholds.

No unresolved architecture or product choice remains for implementation workers.

OVERALL: GREEN
cheap_worker_ready: yes
required_fixups: none
