# Design — 029-compile-reference-validation

## Scope
src/validation/mod.rs, src/engine/mod.rs, src/error.rs, tests/*

## Approach
- Extend compile-time graph validation with recursive end-condition reference checks.
- Add metric-reference validation for:
  - end-condition metric variants,
  - metric-scaled transfer specs,
  - tracked-metric declarations.
- Normalize failure reporting through `SetupError::InvalidGraphReference` to preserve typed error ergonomics.
- Keep runtime deterministic behavior unchanged for valid scenarios while removing unresolved-reference fallback dependency.

## Data Flow
1. `compile_scenario` builds node and edge indexes.
2. Validation pass resolves every node/metric reference used by end conditions and transfer specs.
3. Compile fails early on first unresolved reference with path-rich error detail.
4. Engine executes only validated scenarios; unresolved-reference paths are unreachable in normal execution.

## Compatibility
- Public API surface remains unchanged.
- Error behavior becomes stricter for malformed scenarios (intentional breaking of previously silent invalid setups).

## Deliverable
Reference-safe compile pipeline that prevents typo-driven false positives/false negatives in test scenarios.
