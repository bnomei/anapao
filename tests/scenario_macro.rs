use std::cell::Cell;
use std::num::NonZeroU64;

use anapao::error::SetupError;
use anapao::types::{
    ActionMode, CaptureConfig, ConnectionSpec, DelayConfig, EdgeId, EndConditionSpec, MetricKey,
    NodeBehavior, NodeId, NodeModeConfig, PoolConfig, ResourceConnection, RunConfig, Scenario,
    ScenarioBuilder, ScenarioEdge, ScenarioId, ScenarioNode, StateConnection, StateConnectionRole,
    StateTarget, TransferSpec, TriggerMode, VariableRuntimeConfig,
};
use anapao::Simulator;

fn node<'a>(scenario: &'a Scenario, name: &str) -> &'a ScenarioNode {
    &scenario.nodes()[&NodeId::new(name).unwrap()]
}

fn edge<'a>(scenario: &'a Scenario, name: &str) -> &'a ScenarioEdge {
    &scenario.edges()[&EdgeId::new(name).unwrap()]
}

fn counted<T>(counter: &Cell<usize>, value: T) -> T {
    counter.set(counter.get() + 1);
    value
}

fn setup_error_without_unwind(build: impl FnOnce() -> Result<Scenario, SetupError>) -> SetupError {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(build))
        .expect("scenario! must not introduce an unwind")
        .expect_err("fixture must be invalid")
}

fn assert_graph_error(error: SetupError, expected_graph: &str, expected_reference: &str) {
    assert!(
        matches!(
            error,
            SetupError::InvalidGraphReference { ref graph, ref reference }
                if graph == expected_graph && reference == expected_reference
        ),
        "unexpected checked-builder diagnostic: {error}"
    );
}

mod no_implicit_prelude_hygiene {
    #![no_implicit_prelude]

    #[test]
    fn macro_expansion_uses_absolute_trait_paths() {
        let __anapao_builder = 7_u8;
        let __anapao_node_ids = 8_u8;
        let __anapao_mode = 9_u8;

        let scenario = ::anapao::scenario! {
            id: "no-prelude";
            nodes {
                source: Source { initial: 1.0 };
                gate: Drain {
                    trigger: ::anapao::types::TriggerMode::Passive,
                    action: ::anapao::types::ActionMode::PullAny,
                };
                sink: Sink;
            }
            edges {
                flow: source -> sink => remaining;
            }
            track [sink];
            end max_steps(1);
        };

        ::std::assert!(::std::result::Result::is_ok(&scenario));
        ::std::assert_eq!(__anapao_builder, 7_u8);
        ::std::assert_eq!(__anapao_node_ids, 8_u8);
        ::std::assert_eq!(__anapao_mode, 9_u8);
    }
}

#[test]
fn queue_flow_intake_builds_a_checked_scenario() {
    let scenario = anapao::scenario! {
        id: "queue-flow";

        nodes {
            source: Source { initial: 64.0 };
            delay: Delay { steps: 2 };
            sink: Pool;
        }
        edges {
            source_delay: source -> delay => fixed(1.0);
            delay_sink: delay -> sink => remaining;
        }
    }
    .expect("the canonical queue flow should build");

    assert_eq!(scenario.id(), &ScenarioId::new("queue-flow").unwrap());
    assert_eq!(scenario.nodes()[&NodeId::new("source").unwrap()].initial_value(), 64.0);
    assert_eq!(
        scenario.source_spec().end_conditions,
        vec![EndConditionSpec::MaxSteps { steps: 1 }]
    );
}

#[test]
fn complete_surface_builds_and_preserves_representative_values() {
    let typed_variables = VariableRuntimeConfig::default();
    let typed_end = EndConditionSpec::MaxSteps { steps: 12 };
    let typed_transfer = anapao::TransferSpec::Remaining;
    let typed_pool = anapao::types::PoolConfig::default().with_capacity(99);
    let typed_resource = anapao::types::ResourceConnection::default();
    let typed_state = anapao::types::StateConnection::default();

    let scenario = anapao::scenario! {
        id: String::from("complete-surface");
        title: String::from("Complete surface");
        description: "all families";
        tags [String::from("complete")];
        variables: typed_variables;
        metadata {String::from("suite") => String::from("macro")}
        nodes {
            source: Source { label: "Source", initial: 20.0, tags ["input"], metadata {"kind" => "source"} };
            pool: Pool { config: typed_pool, label: "Pool" };
            drain: Drain { trigger: TriggerMode::Passive, action: ActionMode::PullAny };
            sorting: SortingGate { mode: anapao::types::NodeModeConfig::default() };
            trigger: TriggerGate;
            mixed: MixedGate;
            converter: Converter { ignore_disabled_inputs: true };
            trader: Trader { ignore_disabled_inputs: false };
            register: Register { interactive: true, min_value: none, max_value: 10 };
            delay: Delay { steps: 2 };
            queue: Queue { capacity: none, release_per_step: 2 };
            process: Process;
            sink: Sink;
            gate: Gate;
            custom: Custom(String::from("widget"));
        }
        edges {
            fixed_edge: source -> pool => fixed(1.0) resource { token_size: 1, metadata {"t" => "fixed"} };
            fraction_edge: source -> pool => fraction(1, 2);
            remaining_edge: pool -> sink => remaining;
            metric_edge: source -> pool => metric_scaled(source, 0.5);
            expression_edge: source -> pool => expression("1");
            typed_edge: source -> pool => transfer(typed_transfer) resource { connection: typed_resource, enabled: true };
            converter_in: source -> converter => fixed(1.0);
            converter_out: converter -> sink => remaining;
            trader_in: source -> trader => fixed(1.0);
            trader_out: trader -> sink => remaining;
            state_node: source -> pool => remaining state {
                role: StateConnectionRole::Modifier,
                formula: "+1",
                target: node,
                resource_filter: "any",
                metadata {"target" => "node"},
            };
            state_resource: source -> pool => remaining state {
                target: resource_connection(future_resource),
            };
            state_state: source -> pool => remaining state {
                target: state_connection(state_node),
            };
            state_formula: source -> pool => remaining state {
                target: formula(expression_edge),
            };
            typed_state: source -> pool => remaining state { connection: typed_state };
            future_resource: source -> pool => remaining;
        }
        track [source, pool];
        end max_steps(10);
        end metric_at_least(source, 1);
        end metric_at_most(pool, 100);
        end node_at_least(source, 1);
        end node_at_most(pool, 100);
        end any [max_steps(11), all [node_at_least(source, 1), condition(typed_end.clone())]];
        end condition(typed_end);
    }
    .expect("the complete macro surface should build");

    assert_eq!(scenario.nodes().len(), 15);
    assert!(matches!(
        scenario.nodes()[&NodeId::new("custom").unwrap()].behavior(),
        NodeBehavior::Custom(family) if family == "widget"
    ));
    let NodeBehavior::Drain(drain) = scenario.nodes()[&NodeId::new("drain").unwrap()].behavior()
    else {
        panic!("drain symbol must retain its configured family");
    };
    assert_eq!(drain.mode().trigger_mode, TriggerMode::Passive);
    assert_eq!(drain.mode().action_mode, ActionMode::PullAny);
    assert!(matches!(
        scenario.edges()[&anapao::types::EdgeId::new("state_resource").unwrap()].connection(),
        ConnectionSpec::State(connection)
            if matches!(connection.target(), StateTarget::ResourceConnection(id) if id.as_str() == "future_resource")
    ));
    assert_eq!(
        scenario.source_spec().end_conditions,
        vec![
            EndConditionSpec::MaxSteps { steps: 10 },
            EndConditionSpec::MetricAtLeast {
                metric: MetricKey::new("source").unwrap(),
                value_scaled: 1,
            },
            EndConditionSpec::MetricAtMost {
                metric: MetricKey::new("pool").unwrap(),
                value_scaled: 100,
            },
            EndConditionSpec::NodeAtLeast {
                node_id: NodeId::new("source").unwrap(),
                value_scaled: 1,
            },
            EndConditionSpec::NodeAtMost {
                node_id: NodeId::new("pool").unwrap(),
                value_scaled: 100,
            },
            EndConditionSpec::Any(vec![
                EndConditionSpec::MaxSteps { steps: 11 },
                EndConditionSpec::All(vec![
                    EndConditionSpec::NodeAtLeast {
                        node_id: NodeId::new("source").unwrap(),
                        value_scaled: 1,
                    },
                    EndConditionSpec::MaxSteps { steps: 12 },
                ]),
            ]),
            EndConditionSpec::MaxSteps { steps: 12 },
        ]
    );

    let spec = scenario.source_spec();
    assert_eq!(spec.title.as_deref(), Some("Complete surface"));
    assert_eq!(spec.description.as_deref(), Some("all families"));
    assert_eq!(spec.tags.iter().map(String::as_str).collect::<Vec<_>>(), ["complete"]);
    assert_eq!(spec.metadata.get("suite").map(String::as_str), Some("macro"));
    assert_eq!(spec.variables, VariableRuntimeConfig::default());
    assert_eq!(
        spec.tracked_metrics.iter().map(MetricKey::as_str).collect::<Vec<_>>(),
        ["pool", "source"]
    );

    assert!(matches!(node(&scenario, "source").behavior(), NodeBehavior::Source));
    assert!(matches!(node(&scenario, "pool").behavior(), NodeBehavior::Pool(config)
        if config.capacity() == Some(99)));
    assert!(matches!(node(&scenario, "sorting").behavior(), NodeBehavior::SortingGate(_)));
    assert!(matches!(node(&scenario, "trigger").behavior(), NodeBehavior::TriggerGate(_)));
    assert!(matches!(node(&scenario, "mixed").behavior(), NodeBehavior::MixedGate(_)));
    assert!(matches!(node(&scenario, "converter").behavior(), NodeBehavior::Converter(config)
        if config.ignore_disabled_inputs()));
    assert!(matches!(node(&scenario, "trader").behavior(), NodeBehavior::Trader(config)
        if !config.ignore_disabled_inputs()));
    assert!(matches!(node(&scenario, "register").behavior(), NodeBehavior::Register(config)
        if config.interactive() && config.min_value().is_none() && config.max_value() == Some(10)));
    assert!(matches!(node(&scenario, "delay").behavior(), NodeBehavior::Delay(config)
        if config.delay_steps().get() == 2));
    assert!(matches!(node(&scenario, "queue").behavior(), NodeBehavior::Queue(config)
        if config.capacity().is_none() && config.release_per_step().get() == 2));
    assert!(matches!(node(&scenario, "process").behavior(), NodeBehavior::Process));
    assert!(matches!(node(&scenario, "sink").behavior(), NodeBehavior::Sink));
    assert!(matches!(node(&scenario, "gate").behavior(), NodeBehavior::Gate));
    assert_eq!(node(&scenario, "source").label(), Some("Source"));
    assert_eq!(
        node(&scenario, "source").tags().iter().map(String::as_str).collect::<Vec<_>>(),
        ["input"]
    );
    assert_eq!(
        node(&scenario, "source").metadata().get("kind").map(String::as_str),
        Some("source")
    );

    assert_eq!(edge(&scenario, "fixed_edge").transfer(), &TransferSpec::Fixed { amount: 1.0 });
    assert_eq!(
        edge(&scenario, "fraction_edge").transfer(),
        &TransferSpec::Fraction { numerator: 1, denominator: 2 }
    );
    assert_eq!(edge(&scenario, "remaining_edge").transfer(), &TransferSpec::Remaining);
    assert_eq!(
        edge(&scenario, "metric_edge").transfer(),
        &TransferSpec::MetricScaled { metric: MetricKey::new("source").unwrap(), factor: 0.5 }
    );
    assert_eq!(
        edge(&scenario, "expression_edge").transfer(),
        &TransferSpec::Expression { formula: "1".into() }
    );
    assert_eq!(edge(&scenario, "typed_edge").transfer(), &TransferSpec::Remaining);
    assert!(matches!(edge(&scenario, "fixed_edge").connection(), ConnectionSpec::Resource(config)
        if config.token_size().get() == 1));
    assert!(matches!(edge(&scenario, "state_node").connection(), ConnectionSpec::State(config)
        if config.role() == &StateConnectionRole::Modifier
            && config.formula() == "+1"
            && config.target() == &StateTarget::Node
            && config.resource_filter() == Some("any")));
    assert!(matches!(edge(&scenario, "state_state").connection(), ConnectionSpec::State(config)
        if config.target() == &StateTarget::StateConnection(EdgeId::new("state_node").unwrap())));
    assert!(matches!(edge(&scenario, "state_formula").connection(), ConnectionSpec::State(config)
        if config.target() == &StateTarget::Formula(EdgeId::new("expression_edge").unwrap())));
}

#[test]
fn symbols_use_exact_spelling_and_node_edge_namespaces_are_distinct() {
    let scenario = anapao::scenario! {
        id: "symbol-contract";
        nodes { same_spelling: Source { initial: 1.0 }; sink: Sink; }
        edges { same_spelling: same_spelling -> sink => remaining; }
        track [same_spelling];
        end node_at_least(same_spelling, 0);
    }
    .unwrap();

    assert_eq!(node(&scenario, "same_spelling").id().as_str(), "same_spelling");
    assert_eq!(edge(&scenario, "same_spelling").id().as_str(), "same_spelling");
    assert_eq!(edge(&scenario, "same_spelling").from().as_str(), "same_spelling");
    assert!(scenario
        .source_spec()
        .tracked_metrics
        .contains(&MetricKey::new("same_spelling").unwrap()));
}

#[test]
fn checked_builder_duplicates_retain_the_first_definition() {
    let same = NodeId::new("same").unwrap();
    let sink = NodeId::new("sink").unwrap();
    let edge_id = EdgeId::new("same").unwrap();
    let first_node = ScenarioNode::source(same.clone()).with_label("first");
    let first_edge = ScenarioEdge::resource(
        edge_id.clone(),
        same.clone(),
        sink.clone(),
        TransferSpec::Fixed { amount: 1.0 },
        ResourceConnection::default(),
    )
    .with_metadata("definition", "first");
    let mut builder = ScenarioBuilder::new(ScenarioId::new("duplicate-retention").unwrap());
    builder.insert_node(first_node.clone()).unwrap();
    let node_error =
        builder.insert_node(ScenarioNode::custom(same.clone(), "replacement")).unwrap_err();
    assert_eq!(
        node_error.to_string(),
        "invalid parameter `nodes.same`: a definition with this id already exists"
    );
    builder.insert_node(ScenarioNode::sink(sink.clone())).unwrap();
    builder.insert_edge(first_edge.clone()).unwrap();
    let edge_error = builder
        .insert_edge(ScenarioEdge::resource(
            edge_id.clone(),
            same.clone(),
            sink,
            TransferSpec::Fixed { amount: 2.0 },
            ResourceConnection::default(),
        ))
        .unwrap_err();
    assert_eq!(
        edge_error.to_string(),
        "invalid parameter `edges.same`: a definition with this id already exists"
    );
    let scenario = builder.build().unwrap();
    assert_eq!(&scenario.nodes()[&same], &first_node);
    assert_eq!(&scenario.edges()[&edge_id], &first_edge);
}

#[test]
fn native_and_typed_node_configs_preserve_every_checked_field() {
    let mode =
        NodeModeConfig { trigger_mode: TriggerMode::Enabling, action_mode: ActionMode::PullAll };
    let scenario = anapao::scenario! {
        id: "config-contract";
        nodes {
            pool_value: Pool { capacity: 7, allow_negative_start: true, mode: mode.clone() };
            pool_none: Pool { capacity: none };
            pool_omitted: Pool;
            drain: Drain { config: anapao::types::DrainConfig::default().with_mode(mode.clone()) };
            sorting: SortingGate { trigger: TriggerMode::Passive, action: ActionMode::PushAll };
            trigger: TriggerGate { config: anapao::types::TriggerGateConfig::default().with_mode(mode.clone()) };
            mixed: MixedGate { config: anapao::types::MixedGateConfig::default().with_mode(mode.clone()) };
            converter: Converter { config: anapao::types::ConverterConfig::default().with_ignore_disabled_inputs(true).with_mode(mode.clone()) };
            trader: Trader { config: anapao::types::TraderConfig::default().with_ignore_disabled_inputs(true).with_mode(mode.clone()) };
            register_value: Register { interactive: true, min_value: -2, max_value: 8 };
            register_none: Register { min_value: none, max_value: none };
            register_typed: Register { config: anapao::types::RegisterConfig::default().with_min_value(-3).with_max_value(9) };
            delay: Delay { config: DelayConfig::default().with_delay_steps(NonZeroU64::new(3).unwrap()).with_mode(mode.clone()) };
            queue_value: Queue { capacity: 5, release_per_step: 2, mode: mode.clone() };
            queue_none: Queue { capacity: none };
            queue_omitted: Queue;
            queue_typed: Queue { config: anapao::types::QueueConfig::default().with_capacity(NonZeroU64::new(6).unwrap()).with_release_per_step(NonZeroU64::new(4).unwrap()).with_mode(mode.clone()) };
            source: Source;
            sink: Sink;
        }
        edges {
            converter_in: source -> converter => remaining;
            converter_out: converter -> sink => remaining;
            trader_in: source -> trader => remaining;
            trader_out: trader -> sink => remaining;
        }
    }
    .unwrap();

    assert!(matches!(node(&scenario, "pool_value").behavior(), NodeBehavior::Pool(c)
        if c.capacity() == Some(7) && c.allow_negative_start() && c.mode() == &mode));
    assert!(
        matches!(node(&scenario, "pool_none").behavior(), NodeBehavior::Pool(c) if c.capacity().is_none())
    );
    assert!(
        matches!(node(&scenario, "pool_omitted").behavior(), NodeBehavior::Pool(c) if c == &PoolConfig::default())
    );
    assert!(
        matches!(node(&scenario, "drain").behavior(), NodeBehavior::Drain(c) if c.mode() == &mode)
    );
    assert!(matches!(node(&scenario, "sorting").behavior(), NodeBehavior::SortingGate(c)
        if c.mode().trigger_mode == TriggerMode::Passive && c.mode().action_mode == ActionMode::PushAll));
    assert!(
        matches!(node(&scenario, "trigger").behavior(), NodeBehavior::TriggerGate(c) if c.mode() == &mode)
    );
    assert!(
        matches!(node(&scenario, "mixed").behavior(), NodeBehavior::MixedGate(c) if c.mode() == &mode)
    );
    assert!(matches!(node(&scenario, "converter").behavior(), NodeBehavior::Converter(c)
        if c.ignore_disabled_inputs() && c.mode() == &mode));
    assert!(matches!(node(&scenario, "trader").behavior(), NodeBehavior::Trader(c)
        if c.ignore_disabled_inputs() && c.mode() == &mode));
    assert!(matches!(node(&scenario, "register_value").behavior(), NodeBehavior::Register(c)
        if c.interactive() && c.min_value() == Some(-2) && c.max_value() == Some(8)));
    assert!(matches!(node(&scenario, "register_none").behavior(), NodeBehavior::Register(c)
        if c.min_value().is_none() && c.max_value().is_none()));
    assert!(matches!(node(&scenario, "register_typed").behavior(), NodeBehavior::Register(c)
        if c.min_value() == Some(-3) && c.max_value() == Some(9)));
    assert!(matches!(node(&scenario, "delay").behavior(), NodeBehavior::Delay(c)
        if c.delay_steps().get() == 3 && c.mode() == &mode));
    assert!(matches!(node(&scenario, "queue_value").behavior(), NodeBehavior::Queue(c)
        if c.capacity().map(NonZeroU64::get) == Some(5) && c.release_per_step().get() == 2 && c.mode() == &mode));
    assert!(
        matches!(node(&scenario, "queue_none").behavior(), NodeBehavior::Queue(c) if c.capacity().is_none())
    );
    assert!(matches!(node(&scenario, "queue_omitted").behavior(), NodeBehavior::Queue(c)
        if c == &anapao::types::QueueConfig::default()));
    assert!(matches!(node(&scenario, "queue_typed").behavior(), NodeBehavior::Queue(c)
        if c.capacity().map(NonZeroU64::get) == Some(6) && c.release_per_step().get() == 4 && c.mode() == &mode));
}

#[test]
fn omitted_ends_preserve_the_builder_default() {
    let scenario = anapao::scenario! {
        id: "default-end";
        nodes {}
        edges {}
    }
    .expect("an empty checked document is valid");

    assert_eq!(
        scenario.source_spec().end_conditions,
        vec![EndConditionSpec::MaxSteps { steps: 1 }]
    );
}

#[test]
fn every_caller_expression_category_is_evaluated_once() {
    let counts = (0..39).map(|_| Cell::new(0)).collect::<Vec<_>>();
    let typed_variables = VariableRuntimeConfig::default();
    let typed_pool = PoolConfig::default().with_capacity(9);
    let typed_transfer = TransferSpec::Remaining;
    let typed_resource = ResourceConnection::default();
    let typed_state = StateConnection::default();
    let typed_end = EndConditionSpec::MaxSteps { steps: 4 };

    let scenario = anapao::scenario! {
        id: counted(&counts[0], String::from("expression-count"));
        title: counted(&counts[1], String::from("title"));
        description: counted(&counts[2], String::from("description"));
        tags [counted(&counts[3], String::from("tag"))];
        variables: counted(&counts[4], typed_variables);
        metadata { counted(&counts[5], String::from("key")) => counted(&counts[6], String::from("value")) }
        nodes {
            source: Source {
                label: counted(&counts[7], String::from("source")),
                initial: counted(&counts[8], 20.0),
                tags [counted(&counts[9], String::from("node-tag"))],
                metadata { counted(&counts[10], String::from("nk")) => counted(&counts[11], String::from("nv")) },
            };
            pool: Pool { config: counted(&counts[12], typed_pool) };
            delay: Delay {
                steps: counted(&counts[13], 2),
                trigger: counted(&counts[14], TriggerMode::Automatic),
                action: counted(&counts[15], ActionMode::PushAny),
            };
            custom: Custom(counted(&counts[16], String::from("family")));
            sink: Sink;
        }
        edges {
            fixed: source -> pool => fixed(counted(&counts[17], 1.0)) resource {
                token_size: counted(&counts[18], 1),
                enabled: counted(&counts[19], true),
                metadata { counted(&counts[20], String::from("ek")) => counted(&counts[21], String::from("ev")) },
            };
            fraction: source -> pool => fraction(counted(&counts[22], 1), counted(&counts[23], 2));
            metric: source -> pool => metric_scaled(source, counted(&counts[24], 0.5));
            expression: source -> pool => expression(counted(&counts[25], String::from("1")));
            typed_transfer: pool -> sink => transfer(counted(&counts[26], typed_transfer)) resource {
                connection: counted(&counts[27], typed_resource),
            };
            state: source -> pool => remaining state {
                role: counted(&counts[28], StateConnectionRole::Modifier),
                formula: counted(&counts[29], String::from("+1")),
                resource_filter: counted(&counts[30], String::from("all")),
                enabled: counted(&counts[31], true),
                metadata { counted(&counts[32], String::from("sk")) => counted(&counts[33], String::from("sv")) },
            };
            typed_state: source -> pool => remaining state {
                connection: counted(&counts[34], typed_state),
            };
        }
        end max_steps(counted(&counts[35], 3));
        end metric_at_least(source, counted(&counts[36], 0));
        end node_at_most(pool, counted(&counts[37], 100));
        end condition(counted(&counts[38], typed_end));
    }
    .unwrap();

    assert_eq!(scenario.nodes().len(), 5);
    assert_eq!(
        counts.iter().map(Cell::get).collect::<Vec<_>>(),
        vec![1; counts.len()],
        "each captured caller expression must execute once"
    );
}

#[test]
fn recoverable_macro_failures_return_setup_errors_without_unwinding() {
    let invalid_id = std::panic::catch_unwind(|| {
        anapao::scenario! {
            id: "   ";
            nodes {}
            edges {}
        }
    })
    .expect("the macro must not unwind");
    assert!(matches!(
        invalid_id,
        Err(anapao::error::SetupError::InvalidParameter { ref name, .. }) if name == "id"
    ));

    let zero_delay = std::panic::catch_unwind(|| {
        anapao::scenario! {
            id: "zero-delay";
            nodes { buffer: Delay { steps: 0 }; }
            edges {}
        }
    })
    .expect("checked shorthand conversion must not unwind");
    assert!(matches!(
        zero_delay,
        Err(anapao::error::SetupError::InvalidParameter { ref name, .. })
            if name == "nodes.buffer.config.delay_steps"
    ));

    let zero_token = std::panic::catch_unwind(|| {
        anapao::scenario! {
            id: "zero-token";
            nodes { source: Source; sink: Sink; }
            edges {
                flow: source -> sink => remaining resource { token_size: 0 };
            }
        }
    })
    .expect("resource shorthand conversion must not unwind");
    assert!(matches!(
        zero_token,
        Err(anapao::error::SetupError::InvalidParameter { ref name, .. })
            if name == "edges.flow.connection.resource.token_size"
    ));

    let missing_endpoint = anapao::scenario! {
        id: "missing-endpoint";
        nodes { source: Source; }
        edges { flow: source -> absent => remaining; }
    };
    assert!(matches!(
        missing_endpoint,
        Err(anapao::error::SetupError::InvalidGraphReference { .. })
    ));

    let duplicate = anapao::scenario! {
        id: "duplicate";
        nodes { same: Source; same: Sink; }
        edges {}
    };
    assert!(matches!(
        duplicate,
        Err(anapao::error::SetupError::InvalidParameter { ref name, .. }) if name == "nodes.same"
    ));
}

#[test]
fn every_checked_failure_category_is_recoverable_and_keeps_builder_diagnostics() {
    let zero_queue_capacity = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "zero-queue-capacity";
            nodes { queue: Queue { capacity: 0 }; }
            edges {}
        }
    });
    assert_eq!(
        zero_queue_capacity.to_string(),
        "invalid parameter `nodes.queue.config.capacity`: must be greater than 0"
    );

    let zero_queue_release = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "zero-queue-release";
            nodes { queue: Queue { release_per_step: 0 }; }
            edges {}
        }
    });
    assert_eq!(
        zero_queue_release.to_string(),
        "invalid parameter `nodes.queue.config.release_per_step`: must be greater than 0"
    );

    let zero_fraction = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "zero-fraction";
            nodes { source: Source; sink: Sink; }
            edges { flow: source -> sink => fraction(1, 0); }
        }
    });
    assert_eq!(
        zero_fraction.to_string(),
        "invalid parameter `edges.flow.transfer.fraction.denominator`: must be greater than 0"
    );

    let missing_from = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "missing-from";
            nodes { sink: Sink; }
            edges { flow: absent -> sink => remaining; }
        }
    });
    assert_graph_error(
        missing_from,
        "scenario[missing-from].nodes",
        "edges.flow.from references missing nodes.absent; hint: choose one of the available node IDs: [sink]",
    );

    let missing_to = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "missing-to";
            nodes { source: Source; }
            edges { flow: source -> absent => remaining; }
        }
    });
    assert_graph_error(
        missing_to,
        "scenario[missing-to].nodes",
        "edges.flow.to references missing nodes.absent; hint: choose one of the available node IDs: [source]",
    );

    let missing_transfer_metric = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "missing-transfer-metric";
            nodes { source: Source; sink: Sink; }
            edges { flow: source -> sink => metric_scaled(absent, 1.0); }
        }
    });
    assert_graph_error(
        missing_transfer_metric,
        "scenario[missing-transfer-metric].metrics",
        "edges.flow.transfer.metric references unresolved metric `absent`; hint: choose one of the available metric keys: [sink, source]",
    );

    let missing_tracked = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "missing-tracked";
            nodes {}
            edges {}
            track [absent];
        }
    });
    assert_graph_error(
        missing_tracked,
        "scenario[missing-tracked].metrics",
        "tracked_metrics[absent] references unresolved metric `absent`; hint: choose one of the available metric keys: [<none>]",
    );

    for (missing_target, scenario_id) in [
        (
            setup_error_without_unwind(|| {
                anapao::scenario! {
                    id: "missing-resource-target";
                    nodes { source: Source; sink: Sink; }
                    edges { state: source -> sink => remaining state { target: resource_connection(absent) }; }
                }
            }),
            "missing-resource-target",
        ),
        (
            setup_error_without_unwind(|| {
                anapao::scenario! {
                    id: "missing-state-target";
                    nodes { source: Source; sink: Sink; }
                    edges { state: source -> sink => remaining state { target: state_connection(absent) }; }
                }
            }),
            "missing-state-target",
        ),
        (
            setup_error_without_unwind(|| {
                anapao::scenario! {
                    id: "missing-formula-target";
                    nodes { source: Source; sink: Sink; }
                    edges { state: source -> sink => remaining state { target: formula(absent) }; }
                }
            }),
            "missing-formula-target",
        ),
    ] {
        assert_graph_error(
            missing_target,
            &format!("scenario[{scenario_id}].edges"),
            "edges.state.connection.state.target_connection references missing edges.absent; hint: choose one of the available edge IDs: [state]",
        );
    }

    let missing_end_metric = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "missing-end-metric";
            nodes {}
            edges {}
            end metric_at_least(absent, 1);
        }
    });
    assert_graph_error(
        missing_end_metric,
        "scenario[missing-end-metric].metrics",
        "end_conditions[0].metric references unresolved metric `absent`; hint: choose one of the available metric keys: [<none>]",
    );
    let missing_end_node = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "missing-end-node";
            nodes {}
            edges {}
            end node_at_most(absent, 1);
        }
    });
    assert_graph_error(
        missing_end_node,
        "scenario[missing-end-node].nodes",
        "end_conditions[0].node_id references missing nodes.absent; hint: choose one of the available node IDs: [<none>]",
    );

    let duplicate_edge = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "duplicate-edge";
            nodes { source: Source; sink: Sink; }
            edges {
                same: source -> sink => fixed(1.0);
                same: source -> sink => fixed(2.0);
            }
        }
    });
    assert_eq!(
        duplicate_edge.to_string(),
        "invalid parameter `edges.same`: a definition with this id already exists"
    );

    let cycle = setup_error_without_unwind(|| {
        anapao::scenario! {
            id: "cycle";
            nodes { left: Pool; right: Pool; }
            edges {
                left_right: left -> right => remaining;
                right_left: right -> left => remaining;
            }
        }
    });
    assert!(matches!(cycle, SetupError::CyclicGraph { ref graph, ref cycle_path }
        if graph == "scenario[cycle].resource_connections"
            && cycle_path == &["left", "right", "left"]));
}

#[test]
fn macro_and_direct_builder_are_checked_and_runtime_equivalent() {
    let macro_scenario = anapao::scenario! {
        id: "macro-direct-parity";
        title: "Parity";
        description: "exact checked authoring equivalence";
        tags ["runtime"];
        metadata {"spec" => "040"}
        nodes {
            source: Source { label: "Input", initial: 6.0, tags ["origin"], metadata {"kind" => "source"} };
            delay: Delay { steps: 2 };
            sink: Sink;
        }
        edges {
            source_delay: source -> delay => fixed(1.0) resource { token_size: 1, metadata {"edge" => "first"} };
            delay_sink: delay -> sink => remaining;
        }
        track [sink];
        end max_steps(4);
    }
    .unwrap();

    let source = NodeId::new("source").unwrap();
    let delay = NodeId::new("delay").unwrap();
    let sink = NodeId::new("sink").unwrap();
    let mut direct = ScenarioBuilder::new(ScenarioId::new("macro-direct-parity").unwrap())
        .with_title("Parity")
        .with_description("exact checked authoring equivalence")
        .with_tag("runtime")
        .with_metadata("spec", "040");
    direct
        .insert_node(
            ScenarioNode::source(source.clone())
                .with_label("Input")
                .with_initial_value(6.0)
                .with_tag("origin")
                .with_metadata("kind", "source"),
        )
        .unwrap();
    direct
        .insert_node(ScenarioNode::delay(
            delay.clone(),
            DelayConfig::default().with_delay_steps(NonZeroU64::new(2).unwrap()),
        ))
        .unwrap();
    direct.insert_node(ScenarioNode::sink(sink.clone())).unwrap();
    direct
        .insert_edge(
            ScenarioEdge::resource(
                EdgeId::new("source_delay").unwrap(),
                source,
                delay.clone(),
                TransferSpec::Fixed { amount: 1.0 },
                ResourceConnection::default().with_token_size(NonZeroU64::MIN),
            )
            .with_metadata("edge", "first"),
        )
        .unwrap();
    direct
        .insert_edge(ScenarioEdge::resource(
            EdgeId::new("delay_sink").unwrap(),
            delay,
            sink,
            TransferSpec::Remaining,
            ResourceConnection::default(),
        ))
        .unwrap();
    let direct_scenario = direct
        .with_tracked_metric(MetricKey::new("sink").unwrap())
        .with_end_conditions([EndConditionSpec::MaxSteps { steps: 4 }])
        .build()
        .unwrap();

    assert_eq!(macro_scenario.source_spec(), direct_scenario.source_spec());
    assert_eq!(macro_scenario.nodes(), direct_scenario.nodes());
    assert_eq!(macro_scenario.edges(), direct_scenario.edges());

    let macro_compiled = Simulator::compile_checked(macro_scenario).unwrap();
    let direct_compiled = Simulator::compile_checked(direct_scenario).unwrap();
    let config =
        RunConfig::for_seed(40).with_max_steps(4).with_capture(CaptureConfig::final_only());
    let macro_report = Simulator::run(&macro_compiled, &config).unwrap();
    let direct_report = Simulator::run(&direct_compiled, &config).unwrap();
    assert_eq!(macro_report, direct_report);
}
