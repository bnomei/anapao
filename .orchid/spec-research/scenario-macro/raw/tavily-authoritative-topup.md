# Tavily authoritative-source top-up

Captured 2026-07-11 through targeted `tvly search` and `tvly extract` calls. The installed CLI
rejected the documented `--include-raw-content markdown` and `--chunks-per-source` arguments, so
the searches were rerun without those options and the exact pages were then extracted.

## Extracted primary sources and facts

### Rust Reference: macros by example

Source: https://doc.rust-lang.org/reference/macros-by-example.html

- Macro-by-example has mixed-site hygiene.
- `$crate` refers to the defining crate, and non-macro items require a fully qualified module path.
- `$crate` does not bypass visibility; expansion-referenced items must still be public at the
  invocation site.
- Fragment follow-set restrictions continue through repetitions and their separators.

### Rust Reference: macro ambiguity

Source: https://doc.rust-lang.org/reference/macro-ambiguity.html

- `expr` fragments have a restricted legal follow set that includes `=>`, comma, and semicolon.
- Repetition separators participate in FOLLOW calculations; unconstrained repeated token trees can
  be locally ambiguous even when the macro definition is accepted.

### Rust API Guidelines: macros

Source: https://rust-lang.github.io/api-guidelines/macros.html

- Public macro input should be evocative of the produced Rust shape and use familiar, cohesive
  punctuation and keywords.
- Fragment matchers should accept the complete Rust fragment category they claim to accept.

### trybuild documentation and repository

Sources: https://docs.rs/trybuild/latest/trybuild/ and https://github.com/dtolnay/trybuild

- `TestCases::pass` compiles and executes pass binaries; a panic fails the case.
- `TestCases::compile_fail` requires compilation to fail and compares compiler output with an
  adjacent `.stderr` snapshot.
- Missing snapshots are written under `wip/`; `TRYBUILD=overwrite` refreshes them in place and the
  resulting diff must be reviewed.
- Project dev-dependencies are available to the UI cases.
- The documentation advises testing user-facing output that the crate intentionally cares about,
  rather than exhaustively snapshotting arbitrary type errors.

### Cargo development dependencies

Source: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies

- Dev-dependencies compile tests, examples, and benchmarks but are not propagated to downstream
  packages.
- A versioned dev-dependency can be included for packagers that run published-crate tests.

### Cargo SemVer reference

Source: https://doc.rust-lang.org/cargo/reference/semver.html

- API compatibility is evaluated by whether downstream example usage continues to compile before
  and after a compatible update.
- Moving, renaming, or removing a public item is major API breakage.
- Introducing new lints is usually minor, but documented macro grammar removal or return-type
  changes can directly make downstream usage stop compiling.

### Rust Reference: `#[track_caller]`

Source: https://doc.rust-lang.org/reference/attributes/codegen.html#the-track_caller-attribute

- `#[track_caller]` propagates the caller location through attributed function calls to
  `Location::caller`, enabling a normal assertion method to report the downstream callsite without
  requiring an assertion macro.

### Rust Book: Result versus panic

Source: https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html

- Returning `Result` is the default when library failure is recoverable or expected because it
  leaves recovery policy to the caller; panicking makes that decision on the caller's behalf.

## Research conclusion

The authoritative sources support one public, Rust-shaped macro; `$crate`-qualified public
expansion paths; narrow fragment matchers with legal separators; single evaluation; a recoverable
`Result`; focused `trybuild` UI snapshots; and normal function APIs for assertion ergonomics.
