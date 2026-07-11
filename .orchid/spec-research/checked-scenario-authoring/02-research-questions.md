# Current-state research questions

1. Which public DTO types currently encode node family, node config, connection kind, connection
   payload, state target, and target ID, and what serde attributes define their wire shape?
2. Which numeric scenario fields currently reject zero, and which runtime sites defensively replace
   zero with a default?
3. What do the existing consuming scenario authoring methods do on repeated node or edge IDs?
4. Does compilation currently compare `BTreeMap` keys with embedded node and edge IDs before it
   builds deterministic indexes?
5. Which validation paths accept missing family configs as defaults, and which paths accept a
   nonmatching config by silently substituting defaults?
6. Which engine helpers branch on independent tags and payloads or return fallback values when a
   checked scenario invariant is absent?
7. What serde compatibility tests already pin legacy omitted fields, aliases, default formulas, and
   the omitted default-resource connection object?
8. Which public facade and re-export paths accept `ScenarioSpec` today, and which repository docs,
   tests, benches, and fixtures use the current authoring surface?
9. What deterministic error-path and ordering conventions already exist in `SetupError` and the
   compile validator?
10. Which execution-plan API and privacy contract will spec 037 establish before this work runs?
11. What do official Rust, Cargo, and Serde sources say about sum types, fallible conversion,
    `NonZeroU64`, `#[must_use]`, builder receivers, enum representation, and semver exposure?
12. Which current tests are the best seams for proving valid-scenario behavior remains unchanged?
13. Which formula-bearing fields are parsed during validation, which are inactive control/wire
    fields, and where must the returned AST live so the 037 parse-once plan contract survives a new
    intermediate checked `Scenario`?
