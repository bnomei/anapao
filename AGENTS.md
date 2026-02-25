# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the crate code. `src/lib.rs` exposes core modules (`types`, `engine`, `events`, `batch`, `artifact`, etc.), and `src/main.rs` is the binary entry point.
- `tests/` holds integration tests (for example `tests/readme_playbook.rs`, `tests/perf_determinism.rs`) plus fixtures under `tests/fixtures/` and parity suites under `tests/parity/`.
- `benches/` contains Criterion benches (`simulation`), while `benchmarks/` contains profiling helpers and scripts.
- `specs/` is the spec ledger (`requirements.md`, `design.md`, `tasks.md` per spec id).
- `scripts/` contains release and benchmarking helpers.

## Build, Test, and Development Commands
- `cargo check` - fast compile validation during development.
- `cargo test --all-targets` - run the full local test suite (matches CI).
- `cargo fmt --all -- --check` - formatting check with repository `rustfmt.toml`.
- `cargo clippy --all-targets --all-features -- -D warnings` - lint gate used in CI.
- `cargo bench --bench simulation` - run Criterion benchmark target.
- `./scripts/bench-criterion run --bench simulation` - standardized benchmark run; use `save`/`compare` for baseline regressions.

## Coding Style & Naming Conventions
- Rust edition: 2021, MSRV: 1.70.
- Formatting: rustfmt with max width 100; use default 4-space indentation.
- Naming: modules/files `snake_case`, types/traits `UpperCamelCase`, functions/vars `snake_case`, constants `UPPER_SNAKE_CASE`.
- Keep public surface cohesive via module boundaries rather than large cross-module files.

## Testing Guidelines
- Prefer integration tests in `tests/*.rs` for behavior; keep deterministic seeds explicit.
- Reuse `testkit` fixtures for scenario setup to avoid duplicated test wiring.
- Add parity/fixture data under `tests/parity/` or `tests/fixtures/` when extending model behavior.
- Before opening a PR, run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-targets`.

## Commit & Pull Request Guidelines
- Current history is minimal (`Initial commit`), so use concise imperative commit subjects (<=72 chars), e.g., `Add deterministic batch artifact assertion`.
- Scope commits to one logical change and reference affected areas (`engine`, `artifact`, `tests/parity`, etc.).
- PRs should include: purpose, key design notes, linked issue/spec (for example `specs/032-event-order-contract-hardening`), and exact validation commands run.
