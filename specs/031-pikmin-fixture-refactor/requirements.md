# Requirements — 031-pikmin-fixture-refactor

## Goal
Refactor Pikmin scenario setup into typed, reusable fixture builders so test setup is explicit, safer, and easier to extend.

## EARS
- WHEN tests construct the Pikmin scenario THE SYSTEM SHALL provide a typed config/profile API instead of positional primitive arguments.
- WHEN a caller uses standard profile constructors (`bad-ending-biased`, `good-ending-biased`, `balanced`) THE SYSTEM SHALL preserve current scenario intent and deterministic behavior.
- WHEN fixture tuning values are invalid THEN THE SYSTEM SHALL fail fast with typed setup errors or explicit guardrail checks.
- WHERE shared node/metric identifiers are needed across tests THE SYSTEM SHALL expose canonical helper constants/builders to avoid duplicated string ids.
- WHEN existing Pikmin tests are migrated THE SYSTEM SHALL keep coverage for ending race, reproducibility, probability band, balance guardrails, and event contract.
- WHEN validation runs THE SYSTEM SHALL pass: `cargo test --test pikmin_diagram`.
