DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/expr/mod.rs:640 | Slug: clamp-panics-on-inverted-bounds

# Expression `clamp(value, min, max)` panics when `min > max`

## Finding

The expression evaluator implements `clamp` as `value.clamp(min, max)` (src/expr/mod.rs:640 and the resolver twin at src/expr/mod.rs:667). `f64::clamp` is documented to panic when `min > max`. The three arguments are arbitrary evaluated sub-expressions with no ordering guard, so a valid, finite expression string can make the whole evaluator panic instead of returning the `Result<f64, ExprError>` it promises everywhere else.

## Violated Invariant Or Contract

`ExprRuntime::evaluate` / `evaluate_compiled*` surface every failure as an `ExprError` (`DivisionByZero`, `Empty`, `FunctionArity`, `NonFiniteResult`, `UnknownFunction`, ...). They must never panic on well-formed, finite input. `f64::clamp` breaks that contract for `min > max`.

## Oracle

The std-library contract for `f64::clamp` ("Panics if `min > max`") versus this module's own pervasive no-panic, error-returning contract (every other arithmetic/function path returns an `ExprError`).

## Counterexample

`runtime.evaluate("clamp(5, 10, 0)", &BTreeMap::new())`:
- parses to `Call{ name: "clamp", args: [5, 10, 0] }`
- each arg evaluates to a finite value `[5.0, 10.0, 0.0]`, passing the `is_finite` guard
- `ternary` arity check passes (3 args)
- closure runs `5.0_f64.clamp(10.0, 0.0)` with `min(10) > max(0)` → std panic

Same via the resolver path: `clamp(v, lo, hi)` where resolved `lo=10, hi=0`.

## Why It Might Matter

A library whose evaluator is reachable from compiled scenario transfer/metric expressions can abort the entire run (and any embedding test/tooling process) with a panic rather than a recoverable `ExprError`. A user-authored expression with the bounds written in the "wrong" order is an easy mistake that should be a typed error, not a crash.

## Proof

Control-flow / dataflow trace from the public API:
`evaluate` -> `compile` (parses `clamp(5,10,0)` cleanly; all numeric/finite) -> `evaluate_compiled` -> `eval_node` on the `Call` -> `eval_call` (src/expr/mod.rs:627) -> args evaluated to `[5.0,10.0,0.0]` (no early error) -> `ternary("clamp", ...)` (src/expr/mod.rs:694) arity passes -> closure `value.clamp(min, max)` -> `f64::clamp` panic. Both `eval_call` (640) and `eval_call_with_resolver` (667) share the identical defect.

## Counterevidence Checked

- Searched for any guard on clamp argument ordering: none exists.
- Confirmed the three args cannot be NaN/Inf at the call site: each is an `eval_node` result, which returns `NonFiniteResult` before returning a non-finite value. So the panic is specifically the `min > max` ordering case, not a NaN case.
- Confirmed both evaluation entry paths share the defect (resolver and non-resolver).

## Suggested Next Step

In the `clamp` arms, either validate `min <= max` and return a typed `ExprError` (e.g. an arity/argument error variant) when violated, or compute `value.max(min).min(max)` after a `min <= max` check, mirroring the no-panic contract used by `min`/`max`/`mod`.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed both `eval_call` and `eval_call_with_resolver` called `value.clamp(min, max)`, which `f64::clamp` documents as panicking when `min > max`; args are arbitrary finite sub-expressions with no ordering guard. Chose option 2 (no-panic computation) over option 1 (typed error): adding an error variant is a public-API change, and graceful continuation suits transfer/metric expressions better than aborting a run for a benign swapped-bounds typo. Changed both arms to `value.clamp(a.min(b), a.max(b))`, normalizing the interval to `[min(a,b), max(a,b)]`. Args are guaranteed finite (eval_node returns NonFiniteResult before yielding non-finite), so min/max are finite and ordered and clamp cannot panic. Added regression test `evaluate_clamp_with_inverted_bounds_returns_error_free_result` covering both the `evaluate` path and the `evaluate_compiled_with_resolver` path (clamp(5,10,0)=5, -3=>0, 99=>10, normal ordering unchanged). All 31 expr unit tests green.

DEVANA-KEY: src/expr/mod.rs:640 | P1 | clamp-panics-on-inverted-bounds
DEVANA-SUMMARY: Status=fixed | P1 high src/expr/mod.rs:640 - Expression clamp(value,min,max) called f64::clamp without a min<=max guard, panicking on inverted bounds. Fixed by clamping to the normalized interval [min(a,b),max(a,b)] in both eval paths; regression test added.
