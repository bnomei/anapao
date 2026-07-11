# Tavily PRO research: public `scenario!` macro

Captured 2026-07-11. The initial sandboxed call failed DNS resolution; the same command was rerun
with approved network access and completed after 225 seconds.

## Query

```text
For a public declarative Rust macro_rules! scenario! DSL that expands only through a checked
public ScenarioBuilder API, research complete idiomatic design guidance. Cover: macro grammar for
nested nodes/edges/config/metadata/end/track declarations; using $crate for hygiene and
compatibility with dependency renaming; fragment specifiers and follow-set restrictions;
evaluating every supplied expression exactly once; returning Result and preserving builder errors
rather than panicking; diagnostics and compile-time versus runtime validation; accepting trailing
separators; public macro export/re-export/prelude behavior; rustdoc examples; trybuild
compile-pass/compile-fail tests including hygiene, crate renaming, ambiguity, unsupported syntax,
and diagnostics; runtime equivalence tests against direct builder use; Cargo dev-dependency
implications; semver implications of exported macro grammar and expansions; and why not to add
expectations!/assertion macros when functions and #[track_caller] suffice. Prioritize official Rust
Reference, Rust standard-library/API Guidelines, Cargo SemVer docs, rustdoc/cargo docs, and official
trybuild docs or repository. Distinguish sourced facts from design inferences and provide citation
URLs.
```

## Returned synthesis

- Prefer a single function-like macro with explicit nested labels, specific `ident`, `path`, and
  `expr` fragments, and `tt` only for recursive syntax that cannot use a narrower fragment.
- Fragment follow sets require legal punctuation after expressions; comma/semicolon/`=>`
  separators and optional trailing separators avoid ambiguous matches.
- `$crate` resolves to the defining crate and must be paired with fully qualified public paths.
  Macro-by-example has mixed-site hygiene, so the expansion should not depend on caller imports.
- Bind each captured expression once inside the emitted block before conversion or builder calls.
- Return `Result` and propagate checked-builder failures; use grammar matching for syntax and keep
  semantic uniqueness/graph checks in the builder.
- Public macro grammar and observable output/error/evaluation behavior are compatibility surfaces;
  stabilize the grammar and avoid adding extra macro-only APIs.
- Use compile-pass, compile-fail, hygiene, crate-alias/rename, trailing-separator, diagnostic, and
  direct-builder runtime-equivalence coverage.
- Prefer functions and `#[track_caller]` for assertion ergonomics instead of expanding the public
  macro surface.

## Reported evidence gaps

The PRO report explicitly lacked authoritative `trybuild`, Cargo dev-dependency, and
`#[track_caller]` sources. Those gaps were closed with targeted Tavily searches/extracts preserved
in `tavily-authoritative-topup.md`; no decision relies on the weaker secondary sources returned by
the initial synthesis.

## Primary URLs returned or subsequently verified

- https://doc.rust-lang.org/reference/macros-by-example.html
- https://doc.rust-lang.org/reference/macro-ambiguity.html
- https://rust-lang.github.io/api-guidelines/macros.html
- https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html
- https://rustc-dev-guide.rust-lang.org/macro-expansion.html
