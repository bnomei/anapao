# Shape Review: Compiled Scenario Trust Boundary

## Review Scope

This review evaluates `05-implementation-shape.md` as a complete implementation contract for an
opaque compiled-plan boundary, retained run-invariant work, identifier Serde enforcement, and the
0.2 facade migration.

## Rubric Findings

- **Owners and paths:** GREEN. The new plan owner and every affected existing source, test,
  benchmark, and documentation path are concrete. No glob or placeholder path is required.
- **State ownership:** GREEN. Immutable source/runtime projections live in one `Arc<ExecutionPlan>`;
  RNGs, values, queues, reports, captures, and events remain per run. No hidden shared mutation is
  left to a worker.
- **Data/control flow:** GREEN. The shape specifies `Simulator::compile` -> `TryFrom` -> private
  validator/compiler -> complete plan, then borrowed plan -> per-run engine/batch state.
- **Invariant boundary:** GREEN. Construction is checked, fields are private, internal indexes are
  typed, public reads are borrowed, and no unchecked constructor or mutable plan access exists.
- **Run-invariant retention:** GREEN. The exact discarded/rebuilt structures are named and given
  compile-owned destinations; runtime evaluation/error ownership is distinguished from syntax
  parsing.
- **Public API:** GREEN. The six final accessor signatures, checked conversion, root/prelude export,
  raw-module privacy, consumer migration, and removal of transitional compatibility paths are
  frozen.
- **Compatibility/persistence:** GREEN. `ScenarioSpec` remains the source wire DTO; identifiers keep
  string serialization while deserialization becomes checked; the break is explicitly gated to
  0.2 and release publication remains out of scope.
- **Slices:** GREEN. Four vertical slices can each be validated and committed. The first two may
  run independently; cache migration and final public migration have explicit dependencies. T004
  is a productionization gate, so the final result cannot stop at an intermediate adapter.
- **Test seams:** GREEN. Unit, public integration, determinism, parity, Rayon, README, rustdoc,
  all-feature, Clippy, and Criterion seams are mapped to concrete files/commands.
- **External evidence:** GREEN. Two Tavily reports are saved under `raw/`; the second is restricted
  to primary Cargo/Rust/Serde sources and closes the first report's SemVer evidence gap.
- **Repository conventions:** GREEN. The plan preserves BTree ordering, stable `SetupError` compile
  validation, `Simulator` as the documented facade, explicit seeds, current all-target/all-feature
  gates, and no unsafe code.
- **Cross-spec boundaries:** GREEN. Checked authoring/key-ID validation, capture policy, and macros
  are excluded. The exact downstream dependency target is
  `037-compiled-scenario-trust-boundary/T004`.

## Hidden-Decision Audit

No unresolved worker choice remains concerning module ownership, accessor signatures, constructor
path, Arc/locking posture, compiled-plan contents, identifier Serde strategy, public module policy,
compatibility timing, task order, validation, or escalation. Implementation-level naming inside
private plan helper methods may follow local Rust conventions without changing the frozen contract.

## Risk Review

- The largest risk is semantic drift while replacing repeated map lookups with resolved indexes.
  Existing parity/determinism/event tests plus fresh Sol/high review are explicit gates.
- The public break is broad but mechanically enumerable from repository consumers and has exact
  replacements. T004 owns complete migration and removes transitional APIs.
- `Arc` thread-safety is verified through trait assertions and the existing parallel feature path;
  the design forbids interior mutability.
- Identifier deserialization can affect persisted input, but the new behavior matches the already
  documented constructor invariant and preserves valid JSON shape.

OVERALL: GREEN
cheap_worker_ready: yes
required_fixups: none
