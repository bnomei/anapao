use anapao::types::{
    ActionMode, ConnectionSpec, EndConditionSpec, NodeBehavior, NodeId, ScenarioId,
    StateConnectionRole, StateTarget, TriggerMode, VariableRuntimeConfig,
};

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
    assert_eq!(scenario.source_spec().end_conditions.len(), 7);
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
