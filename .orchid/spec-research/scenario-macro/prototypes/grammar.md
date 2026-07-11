# Evidence-only `scenario!` grammar sketch

This sketch freezes vocabulary and examples, not matcher implementation. The production macro may
factor matcher arms differently, but it must accept every documented form and reject every
documented invalid combination with focused diagnostics.

## Canonical top-level order

```rust
scenario! {
    id: scenario_id_expr;
    title: title_expr;                         // optional
    description: description_expr;             // optional
    tags [tag_expr, another_tag_expr,];         // optional
    variables: variable_runtime_config_expr;   // optional pass-through
    metadata {
        key_expr => value_expr,
    }                                          // optional

    nodes {
        // zero or more declarations
    }

    edges {
        // zero or more declarations
    }

    track [node_symbol, another_node_symbol,]; // optional
    end max_steps(steps_expr);                 // zero or more; builder default if absent
}
```

Top-level sections use the order above so `macro_rules!` can keep a small, unambiguous public
entry matcher. Repeated entries and property blocks accept an optional trailing comma; declarations
and top-level scalar/end statements accept the shown semicolon. The exact starting example with a
single uncommaed node field remains valid.

`id:` accepts an expression implementing `TryInto<ScenarioId>` so callers may provide `&str`,
`String`, or an already checked `ScenarioId`. A node or edge symbol is a Rust `ident`; its generated
ID string is exactly `stringify!(symbol)`. Node and edge symbols occupy distinct namespaces.

## Node declarations

Configless families:

```rust
source: Source;
process: Process { initial: process_initial };
sink: Sink { label: "Sink", tags: ["terminal",] };
gate: Gate;
custom: Custom(custom_family_expr) { metadata: { "owner" => owner_expr, } };
```

Configured families, including default and native-field forms:

```rust
pool: Pool;
pool_tuned: Pool {
    capacity: pool_capacity_expr,
    allow_negative_start: allow_negative_expr,
    trigger: trigger_mode_expr,
    action: action_mode_expr,
    initial: pool_initial_expr,
};
drain: Drain { mode: node_mode_config_expr };
sorting: SortingGate { trigger: trigger_expr, action: action_expr };
trigger: TriggerGate { mode: mode_expr };
mixed: MixedGate { mode: mode_expr };
converter: Converter { ignore_disabled_inputs: ignore_expr, mode: mode_expr };
trader: Trader { ignore_disabled_inputs: ignore_expr, mode: mode_expr };
register: Register {
    interactive: interactive_expr,
    min_value: min_expr,
    max_value: max_expr,
};
delay: Delay { steps: delay_steps_expr, mode: mode_expr };
queue: Queue {
    capacity: queue_capacity_expr,
    release_per_step: release_expr,
    mode: mode_expr,
};
```

Every configured family also accepts `config: checked_config_expr` as an exclusive escape hatch:

```rust
queue: Queue { config: checked_queue_config_expr, initial: initial_expr };
```

`config:` cannot be combined with family config fields. `mode:` cannot be combined with the
`trigger:`/`action:` shorthands. Omission preserves the checked config default. Pool/Queue
`capacity` and Register `min_value`/`max_value` also accept the literal `none`, mapping to the exact
`without_capacity`/`without_min_value`/`without_max_value` methods frozen by spec 039. The typed
`config:` form handles a dynamically computed optional value.

Common node fields, accepted at most once in any order after or among family fields, are:

```text
label: <Into<String> expression>
initial: <f64 expression>
tags: [<Into<String> expression>, ...]
metadata: { <Into<String> expression> => <Into<String> expression>, ... }
```

Scalar duplicate fields, unknown fields, wrong-family fields, and mutually exclusive config forms
are syntax errors with a macro-owned diagnostic. Repeated values belong inside `tags` or `metadata`,
not repeated scalar properties.

## Transfers and edges

Transfer forms cover every `TransferSpec` variant plus a pass-through escape hatch:

```text
fixed(amount_expr)
fraction(numerator_expr, denominator_expr)
remaining
metric_scaled(node_symbol, factor_expr)
expression(formula_expr)
transfer(transfer_spec_expr)
```

Default resource, configured resource, and configured state edges:

```rust
source_delay: source -> delay => fixed(amount_expr);

tokenized: source -> queue => fraction(numerator_expr, denominator_expr) resource {
    token_size: token_size_expr,
    enabled: enabled_expr,
    metadata: { "channel" => channel_expr, },
};

state_gate: gate -> queue => remaining state {
    role: state_role_expr,
    formula: formula_expr,
    target: resource_connection(tokenized),
    resource_filter: filter_expr,
    enabled: state_enabled_expr,
    metadata: { "purpose" => purpose_expr, },
};
```

Resource blocks accept either `connection: ResourceConnection` or `token_size`, never both. State
blocks accept either `connection: StateConnection` or the native role/formula/target/filter fields,
never both. Both accept common `enabled` and `metadata` fields. Omitted connection fields preserve
checked defaults.

State target forms cover all `StateTarget` variants and allow forward edge references because all
edge symbols are registered before edge construction:

```text
node
resource_connection(edge_symbol)
state_connection(edge_symbol)
formula(edge_symbol)
```

## Tracking and end conditions

`track [node_symbol, ...];` derives each `MetricKey` from the declared node symbol. Transfer
`metric_scaled` and metric end conditions use the same node-symbol-to-metric rule, matching the
current source contract that metrics resolve to nodes.

End forms cover every `EndConditionSpec` variant and one pass-through form:

```text
max_steps(steps_expr)
metric_at_least(node_symbol, scaled_value_expr)
metric_at_most(node_symbol, scaled_value_expr)
node_at_least(node_symbol, scaled_value_expr)
node_at_most(node_symbol, scaled_value_expr)
any [<end form>, ...]
all [<end form>, ...]
condition(end_condition_spec_expr)
```

Multiple top-level `end ...;` statements preserve declaration order in
`ScenarioBuilder::with_end_conditions` and therefore retain the current top-level OR semantics.
Nested `any` and `all` produce explicit recursive variants. Empty composites reach checked builder
validation and return the established `SetupError`; the macro does not invent a second validator.

## Complete example

```rust
let scenario = scenario! {
    id: "complete-macro";
    title: title_expr;
    description: "all supported sections";
    tags ["macro", dynamic_tag_expr,];
    variables: variables_expr;
    metadata { "suite" => "ui", }

    nodes {
        source: Source { initial: source_initial_expr };
        pool: Pool { capacity: pool_capacity_expr };
        drain: Drain;
        sorting: SortingGate { mode: sorting_mode_expr };
        trigger_gate: TriggerGate;
        mixed_gate: MixedGate;
        converter: Converter { ignore_disabled_inputs: true };
        trader: Trader { ignore_disabled_inputs: false };
        register: Register { interactive: true, min_value: min_expr, max_value: max_expr };
        delay: Delay { steps: delay_expr };
        queue: Queue { capacity: queue_capacity_expr, release_per_step: release_expr };
        process: Process;
        sink: Sink;
        gate: Gate;
        custom: Custom(custom_family_expr);
    }

    edges {
        fixed_edge: source -> pool => fixed(fixed_expr);
        fraction_edge: pool -> delay => fraction(numerator_expr, denominator_expr);
        remaining_edge: delay -> queue => remaining;
        metric_edge: queue -> sink => metric_scaled(pool, factor_expr);
        expression_edge: process -> sink => expression(formula_expr);
        pass_edge: custom -> sink => transfer(transfer_expr) resource {
            connection: resource_connection_expr,
            enabled: enabled_expr,
        };
        state_edge: gate -> queue => remaining state {
            role: role_expr,
            formula: state_formula_expr,
            target: formula(expression_edge),
            resource_filter: filter_expr,
        };
    }

    track [pool, queue, sink,];
    end max_steps(max_steps_expr);
    end all [
        node_at_least(sink, node_threshold_expr),
        any [
            metric_at_least(queue, metric_min_expr),
            metric_at_most(pool, metric_max_expr),
        ],
    ];
}?;
```
