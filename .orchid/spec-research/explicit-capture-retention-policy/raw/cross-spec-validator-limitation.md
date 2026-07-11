# Cross-Spec Task Dependency Validator Evidence — 2026-07-11

The sibling task existed at:

```text
specs/037-compiled-scenario-trust-boundary/tasks/T004.md
```

T001 initially used the documented cross-spec dependency form:

```toml
depends = ["037-compiled-scenario-trust-boundary/T004"]
```

Command:

```text
UV_CACHE_DIR=/private/tmp/anpao-uv-cache uv run python /Users/bnomei/.codex/skills/make-specs/scripts/validate_task_spec.py specs/038-explicit-capture-retention-policy
```

Exact result (both before and after the sibling task file was present):

```text
error: T001: unknown dependency 037-compiled-scenario-trust-boundary/T004
```

Supported active-spec fallback approved for this packet:

- keep `spec.toml.depends_on = ["037-compiled-scenario-trust-boundary"]`;
- use `human_checkpoint = "before-implementation"`;
- keep `depends = []` in T001 until sibling-task resolution is supported;
- add `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` to T001 `read_allowlist`;
- before any edit, require that task file to contain exactly `status = "done"` and
  `verification_status = "passed"`; absence of either predicate is a hard T001 Context/Escalate stop;
- report that the concrete frontmatter edge must be added later rather than treating the prerequisite
  as optional.
