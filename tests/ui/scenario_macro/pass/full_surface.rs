use anapao::types::{ActionMode, EndConditionSpec, StateConnectionRole, TriggerMode};

fn main() {
    let pool_config = anapao::types::PoolConfig::default().with_capacity(99);
    let resource = anapao::types::ResourceConnection::default();
    let state = anapao::types::StateConnection::default();
    let transfer = anapao::types::TransferSpec::Remaining;
    let end = EndConditionSpec::MaxSteps { steps: 12 };
    let scenario = anapao::scenario! {
        id: String::from("full-surface");
        title: "Full surface";
        description: "all public grammar";
        tags ["ui", String::from("complete")];
        variables: anapao::types::VariableRuntimeConfig::default();
        metadata {"suite" => "trybuild", String::from("kind") => String::from("pass")}
        nodes {
            source: Source { label: "Source", initial: 20.0, tags ["input"], metadata {"kind" => "source"} };
            pool: Pool { capacity: 20, allow_negative_start: false, mode: anapao::types::NodeModeConfig::default() };
            typed_pool: Pool { config: pool_config };
            unbounded_pool: Pool { capacity: none };
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
            fixed_edge: source -> pool => fixed(1.0) resource { token_size: 1, enabled: true, metadata {"t" => "fixed"} };
            fraction_edge: source -> pool => fraction(1, 2);
            remaining_edge: pool -> sink => remaining;
            metric_edge: source -> pool => metric_scaled(source, 0.5);
            expression_edge: source -> pool => expression("1");
            typed_edge: source -> pool => transfer(transfer) resource { connection: resource };
            converter_in: source -> converter => fixed(1.0);
            converter_out: converter -> sink => remaining;
            trader_in: source -> trader => fixed(1.0);
            trader_out: trader -> sink => remaining;
            state_node: source -> pool => remaining state { role: StateConnectionRole::Modifier, formula: "+1", target: node, resource_filter: "any", enabled: true, metadata {"target" => "node"} };
            state_resource: source -> pool => remaining state { target: resource_connection(future_resource) };
            state_state: source -> pool => remaining state { target: state_connection(state_node) };
            state_formula: source -> pool => remaining state { target: formula(expression_edge) };
            typed_state: source -> pool => remaining state { connection: state };
            future_resource: source -> pool => remaining;
        }
        track [source, pool];
        end max_steps(10);
        end metric_at_least(source, 1);
        end metric_at_most(pool, 100);
        end node_at_least(source, 1);
        end node_at_most(pool, 100);
        end any [max_steps(11), all [node_at_least(source, 1), condition(end.clone())]];
        end condition(end);
    }.unwrap();
    assert_eq!(scenario.nodes().len(), 17);
}
