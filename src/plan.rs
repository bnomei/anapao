//! Immutable execution projections produced by checked scenario compilation.
//!
//! This module is the sole owner of the runtime layout behind
//! [`CompiledScenario`].  The public handle intentionally exposes only stable
//! inspection accessors; execution modules use the narrow crate-private query
//! methods below instead of rejoining the source maps and derived indexes.

use std::{collections::BTreeMap, num::NonZeroU64, sync::Arc};

use crate::{
    expr::CompiledExpr,
    types::{
        ActionMode, ConnectionSpec, EdgeId, EndConditionSpec, MetricKey, NodeBehavior, NodeId,
        NodeModeConfig, ResourceConnection, ScenarioEdge, ScenarioId, ScenarioNode, ScenarioSpec,
        StateConnection, StateConnectionRole, StateTarget, VariableRuntimeConfig,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledScenario(Arc<ExecutionPlan>);

#[derive(Debug, PartialEq)]
struct ExecutionPlan {
    source_spec: ScenarioSpec,
    node_ids: Box<[NodeId]>,
    edge_ids: Box<[EdgeId]>,
    nodes: Box<[CompiledNode]>,
    edges: Box<[CompiledEdge]>,
    node_index_by_id: BTreeMap<NodeId, NodeIndex>,
    edge_index_by_id: BTreeMap<EdgeId, EdgeIndex>,
    expressions: CompiledExpressions,
    routing: RoutingPlan,
    metrics: MetricPlan,
    variables: VariableRuntimeConfig,
    end_conditions: Box<[EndConditionSpec]>,
}

pub(crate) struct PlanProjections {
    node_ids: Box<[NodeId]>,
    edge_ids: Box<[EdgeId]>,
    nodes: Box<[CompiledNode]>,
    edges: Box<[CompiledEdge]>,
}

pub(crate) struct PlanIndexes {
    node_index_by_id: BTreeMap<NodeId, NodeIndex>,
    edge_index_by_id: BTreeMap<EdgeId, EdgeIndex>,
}

/// Expression ASTs retained in edge-index order by successful compilation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompiledExpressions {
    transfer_by_edge: Box<[Option<CompiledExpr>]>,
    state_by_edge: Box<[Option<CompiledExpr>]>,
}

/// The two parse results associated with an edge while validation is assembling
/// the immutable plan. `None` means that edge has no expression of that kind.
pub(crate) struct ExpressionSlots {
    pub(crate) transfer: Option<CompiledExpr>,
    pub(crate) state: Option<CompiledExpr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TransferControl {
    PullAny,
    PullAll,
    PushAny,
    PushAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerTarget {
    Node(NodeIndex),
    Edge(EdgeIndex),
}

/// Resolved routing data used unchanged by every run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RoutingPlan {
    resource_groups_by_controller: BTreeMap<NodeIndex, BTreeMap<TransferControl, Box<[EdgeIndex]>>>,
    passive_state_triggers: Box<[(NodeIndex, Box<[TriggerTarget]>)]>,
    trigger_outputs_by_source: BTreeMap<NodeIndex, Box<[TriggerTarget]>>,
}

/// Deterministic metric key ordering and their resolved node projections.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MetricPlan {
    tracked: Box<[MetricKey]>,
    node_index_by_name: BTreeMap<String, NodeIndex>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompiledNode {
    id: NodeId,
    behavior: NodeBehavior,
    initial_value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompiledEdge {
    id: EdgeId,
    from: NodeId,
    to: NodeId,
    transfer: Option<CompiledTransfer>,
    connection: ConnectionSpec,
    enabled: bool,
    from_index: NodeIndex,
    to_index: NodeIndex,
}

/// Transfer projection used by the engine after checked compilation.
///
/// Resource fractions carry a non-zero denominator, so execution never needs
/// to repair an invalid DTO value or branch around division by zero.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompiledTransfer {
    Fixed { amount: f64 },
    Fraction { numerator: u64, denominator: NonZeroU64 },
    Remaining,
    MetricScaled { metric: MetricKey, factor: f64 },
    Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NodeIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EdgeIndex(usize);

impl CompiledScenario {
    /// Returns the stable scenario identifier supplied by the authoring DTO.
    pub fn scenario_id(&self) -> &ScenarioId {
        &self.0.source_spec.id
    }

    /// Returns the canonical source DTO retained for read-only inspection.
    pub fn source_spec(&self) -> &ScenarioSpec {
        &self.0.source_spec
    }

    /// Returns deterministic node IDs in source `BTreeMap` key order.
    pub fn node_ids(&self) -> &[NodeId] {
        &self.0.node_ids
    }

    /// Returns deterministic edge IDs in source `BTreeMap` key order.
    pub fn edge_ids(&self) -> &[EdgeId] {
        &self.0.edge_ids
    }

    pub fn node_count(&self) -> usize {
        self.0.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.0.edges.len()
    }

    pub(crate) fn from_validated(
        source_spec: ScenarioSpec,
        projections: PlanProjections,
        indexes: PlanIndexes,
        expressions: CompiledExpressions,
        routing: RoutingPlan,
        metrics: MetricPlan,
    ) -> Self {
        let variables = source_spec.variables.clone();
        let end_conditions = source_spec.end_conditions.clone().into_boxed_slice();
        Self(Arc::new(ExecutionPlan {
            source_spec,
            node_ids: projections.node_ids,
            edge_ids: projections.edge_ids,
            nodes: projections.nodes,
            edges: projections.edges,
            node_index_by_id: indexes.node_index_by_id,
            edge_index_by_id: indexes.edge_index_by_id,
            expressions,
            routing,
            metrics,
            variables,
            end_conditions,
        }))
    }

    /// Returns a node whose identity was already resolved by checked compilation.
    ///
    /// Engine callers use this only for IDs derived from the immutable plan; a
    /// missing entry is therefore an invariant breach, never a runtime default.
    pub(crate) fn required_node(&self, id: &NodeId) -> &CompiledNode {
        &self.0.nodes[self.0.node_index_by_id[id].0]
    }

    /// Returns a node index whose identity was already resolved by compilation.
    pub(crate) fn required_node_index(&self, id: &NodeId) -> usize {
        self.0.node_index_by_id[id].0
    }

    pub(crate) fn node_id_at(&self, index: usize) -> Option<&NodeId> {
        self.0.nodes.get(index).map(|node| &node.id)
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = (&NodeId, &CompiledNode)> {
        self.0.nodes.iter().map(|node| (&node.id, node))
    }

    pub(crate) fn edge_at(&self, index: EdgeIndex) -> &CompiledEdge {
        &self.0.edges[index.value()]
    }

    pub(crate) fn node_id_at_index(&self, index: NodeIndex) -> &NodeId {
        &self.0.nodes[index.value()].id
    }

    pub(crate) fn edges(&self) -> impl Iterator<Item = &CompiledEdge> {
        self.0.edges.iter()
    }

    pub(crate) fn node_index(&self, id: &NodeId) -> Option<usize> {
        self.0.node_index_by_id.get(id).map(|index| index.0)
    }

    pub(crate) fn edge_index(&self, id: &EdgeId) -> Option<usize> {
        self.0.edge_index_by_id.get(id).map(|index| index.0)
    }

    pub(crate) fn metric_node_index(&self, metric: &MetricKey) -> Option<usize> {
        self.0.metrics.node_index_by_name.get(metric.as_str()).map(|index| index.0)
    }

    pub(crate) fn tracked_metrics(&self) -> &[MetricKey] {
        self.0.metrics.tracked()
    }

    pub(crate) fn variables(&self) -> &crate::types::VariableRuntimeConfig {
        &self.0.variables
    }

    pub(crate) fn end_conditions(&self) -> &[crate::types::EndConditionSpec] {
        &self.0.end_conditions
    }

    pub(crate) fn expressions(&self) -> &CompiledExpressions {
        &self.0.expressions
    }

    pub(crate) fn routing(&self) -> &RoutingPlan {
        &self.0.routing
    }
}

impl CompiledNode {
    pub(crate) fn new(id: NodeId, node: &ScenarioNode) -> Self {
        Self { id, behavior: node.behavior().clone(), initial_value: node.initial_value() }
    }

    pub(crate) fn behavior(&self) -> &NodeBehavior {
        &self.behavior
    }

    pub(crate) fn initial_value(&self) -> f64 {
        self.initial_value
    }
}

impl PlanProjections {
    pub(crate) fn new(
        node_ids: Box<[NodeId]>,
        edge_ids: Box<[EdgeId]>,
        nodes: Box<[CompiledNode]>,
        edges: Box<[CompiledEdge]>,
    ) -> Self {
        Self { node_ids, edge_ids, nodes, edges }
    }
}

impl PlanIndexes {
    pub(crate) fn new(
        node_index_by_id: BTreeMap<NodeId, NodeIndex>,
        edge_index_by_id: BTreeMap<EdgeId, EdgeIndex>,
    ) -> Self {
        Self { node_index_by_id, edge_index_by_id }
    }
}

impl NodeIndex {
    pub(crate) fn new(value: usize) -> Self {
        Self(value)
    }

    pub(crate) fn value(self) -> usize {
        self.0
    }
}

impl EdgeIndex {
    pub(crate) fn new(value: usize) -> Self {
        Self(value)
    }

    pub(crate) fn value(self) -> usize {
        self.0
    }
}

impl CompiledExpressions {
    pub(crate) fn from_slots(slots: Vec<ExpressionSlots>) -> Self {
        let (transfer, state): (Vec<_>, Vec<_>) =
            slots.into_iter().map(|slot| (slot.transfer, slot.state)).unzip();
        Self {
            transfer_by_edge: transfer.into_boxed_slice(),
            state_by_edge: state.into_boxed_slice(),
        }
    }

    pub(crate) fn transfer(&self, edge: EdgeIndex) -> Option<&CompiledExpr> {
        self.transfer_by_edge.get(edge.value()).and_then(Option::as_ref)
    }

    pub(crate) fn state(&self, edge: EdgeIndex) -> Option<&CompiledExpr> {
        self.state_by_edge.get(edge.value()).and_then(Option::as_ref)
    }
}

impl RoutingPlan {
    pub(crate) fn from_checked(
        nodes: &BTreeMap<NodeId, ScenarioNode>,
        edges: &BTreeMap<EdgeId, ScenarioEdge>,
        node_index_by_id: &BTreeMap<NodeId, NodeIndex>,
        edge_index_by_id: &BTreeMap<EdgeId, EdgeIndex>,
    ) -> Self {
        let mut resource_groups_by_controller =
            BTreeMap::<NodeIndex, BTreeMap<TransferControl, Vec<EdgeIndex>>>::new();
        let mut passive_state_triggers = Vec::new();
        let mut trigger_outputs_by_source = BTreeMap::<NodeIndex, Vec<TriggerTarget>>::new();

        for (edge_id, edge) in edges {
            if !edge.enabled() {
                continue;
            }
            let edge_index = edge_index_by_id[edge_id];
            let source = node_index_by_id[edge.from()];
            let target = node_index_by_id[edge.to()];
            if matches!(edge.connection(), ConnectionSpec::Resource(_)) {
                let target_action = normalized_action_mode(action_mode_for_node(nodes, edge.to()));
                let (controller, control) = match target_action {
                    TransferControl::PullAny | TransferControl::PullAll => (target, target_action),
                    TransferControl::PushAny | TransferControl::PushAll => {
                        (source, normalized_action_mode(action_mode_for_node(nodes, edge.from())))
                    }
                };
                resource_groups_by_controller
                    .entry(controller)
                    .or_default()
                    .entry(control)
                    .or_default()
                    .push(edge_index);
            }
            let ConnectionSpec::State(state) = edge.connection() else {
                continue;
            };
            if !matches!(state.role(), StateConnectionRole::Trigger) {
                continue;
            }
            let targets = trigger_targets(state.target(), target, edge_index_by_id);
            trigger_outputs_by_source.entry(source).or_default().extend(targets.iter().copied());
            if !is_trigger_gate(nodes, edge.from()) {
                passive_state_triggers.push((source, targets.into_boxed_slice()));
            }
        }
        Self {
            resource_groups_by_controller: resource_groups_by_controller
                .into_iter()
                .map(|(controller, groups)| {
                    (
                        controller,
                        groups
                            .into_iter()
                            .map(|(control, edges)| (control, edges.into_boxed_slice()))
                            .collect(),
                    )
                })
                .collect(),
            passive_state_triggers: passive_state_triggers.into_boxed_slice(),
            trigger_outputs_by_source: trigger_outputs_by_source
                .into_iter()
                .map(|(source, targets)| (source, targets.into_boxed_slice()))
                .collect(),
        }
    }

    pub(crate) fn resource_group(
        &self,
        controller: NodeIndex,
        control: TransferControl,
    ) -> Option<&[EdgeIndex]> {
        self.resource_groups_by_controller.get(&controller)?.get(&control).map(Box::as_ref)
    }

    pub(crate) fn passive_state_triggers(&self) -> &[(NodeIndex, Box<[TriggerTarget]>)] {
        &self.passive_state_triggers
    }

    pub(crate) fn trigger_outputs(&self, source: NodeIndex) -> Option<&[TriggerTarget]> {
        self.trigger_outputs_by_source.get(&source).map(Box::as_ref)
    }
}

impl MetricPlan {
    pub(crate) fn from_spec(
        spec: &ScenarioSpec,
        node_index_by_id: &BTreeMap<NodeId, NodeIndex>,
    ) -> Self {
        Self {
            tracked: spec.tracked_metrics.iter().cloned().collect(),
            node_index_by_name: node_index_by_id
                .iter()
                .map(|(node_id, index)| (node_id.as_str().to_string(), *index))
                .collect(),
        }
    }

    pub(crate) fn tracked(&self) -> &[MetricKey] {
        &self.tracked
    }
}

fn action_mode_for_node(nodes: &BTreeMap<NodeId, ScenarioNode>, node_id: &NodeId) -> ActionMode {
    node_mode(nodes[node_id].behavior())
        .map(|mode| mode.action_mode.clone())
        .unwrap_or(ActionMode::PushAny)
}

fn node_mode(behavior: &NodeBehavior) -> Option<&NodeModeConfig> {
    match behavior {
        NodeBehavior::Pool(config) => Some(config.mode()),
        NodeBehavior::Drain(config) => Some(config.mode()),
        NodeBehavior::SortingGate(config) => Some(config.mode()),
        NodeBehavior::TriggerGate(config) => Some(config.mode()),
        NodeBehavior::MixedGate(config) => Some(config.mode()),
        NodeBehavior::Converter(config) => Some(config.mode()),
        NodeBehavior::Trader(config) => Some(config.mode()),
        NodeBehavior::Delay(config) => Some(config.mode()),
        NodeBehavior::Queue(config) => Some(config.mode()),
        NodeBehavior::Source
        | NodeBehavior::Register(_)
        | NodeBehavior::Process
        | NodeBehavior::Sink
        | NodeBehavior::Gate
        | NodeBehavior::Custom(_) => None,
    }
}

fn normalized_action_mode(mode: ActionMode) -> TransferControl {
    match mode {
        ActionMode::PullAny => TransferControl::PullAny,
        ActionMode::PullAll => TransferControl::PullAll,
        ActionMode::PushAll => TransferControl::PushAll,
        ActionMode::PushAny | ActionMode::Custom(_) => TransferControl::PushAny,
    }
}

fn is_trigger_gate(nodes: &BTreeMap<NodeId, ScenarioNode>, node_id: &NodeId) -> bool {
    matches!(nodes[node_id].behavior(), NodeBehavior::TriggerGate(_) | NodeBehavior::MixedGate(_))
}

fn trigger_targets(
    target: &StateTarget,
    node_target: NodeIndex,
    edge_index_by_id: &BTreeMap<EdgeId, EdgeIndex>,
) -> Vec<TriggerTarget> {
    match target {
        StateTarget::Node => vec![TriggerTarget::Node(node_target)],
        StateTarget::ResourceConnection(id) | StateTarget::StateConnection(id) => {
            vec![TriggerTarget::Edge(edge_index_by_id[id])]
        }
        StateTarget::Formula(_) => Vec::new(),
    }
}

impl CompiledEdge {
    pub(crate) fn new(
        id: EdgeId,
        edge: &ScenarioEdge,
        transfer: Option<CompiledTransfer>,
        from_index: usize,
        to_index: usize,
    ) -> Self {
        Self {
            id,
            from: edge.from().clone(),
            to: edge.to().clone(),
            transfer,
            connection: edge.connection().clone(),
            enabled: edge.enabled(),
            from_index: NodeIndex(from_index),
            to_index: NodeIndex(to_index),
        }
    }

    pub(crate) fn id(&self) -> &EdgeId {
        &self.id
    }

    pub(crate) fn from(&self) -> &NodeId {
        &self.from
    }

    pub(crate) fn to(&self) -> &NodeId {
        &self.to
    }

    pub(crate) fn resource(&self) -> Option<(&ResourceConnection, &CompiledTransfer)> {
        match (&self.connection, &self.transfer) {
            (ConnectionSpec::Resource(connection), Some(transfer)) => Some((connection, transfer)),
            _ => None,
        }
    }

    pub(crate) fn state(&self) -> Option<&StateConnection> {
        match &self.connection {
            ConnectionSpec::State(connection) => Some(connection),
            ConnectionSpec::Resource(_) => None,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn source_index(&self) -> usize {
        self.from_index.0
    }

    pub(crate) fn target_index(&self) -> usize {
        self.to_index.0
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{CompiledScenario, EdgeIndex, NodeIndex, TransferControl, TriggerTarget};
    use crate::{
        testkit::{deterministic_run_config, fixture_scenario},
        types::{
            ConnectionKind, EdgeConnectionConfig, EdgeId, EdgeSpec, MetricKey, NodeId, NodeKind,
            NodeSpec, Scenario, ScenarioId, ScenarioSpec, StateConnectionConfig,
            StateConnectionRole, StateConnectionTarget, TransferSpec,
        },
        Simulator,
    };

    #[test]
    fn compiled_scenario_clones_share_one_arc_allocation() {
        let compiled = Simulator::compile(fixture_scenario()).expect("fixture should compile");
        let clone = compiled.clone();
        assert!(Arc::ptr_eq(&compiled.0, &clone.0));
    }

    #[test]
    fn compiled_scenario_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CompiledScenario>();
    }

    #[test]
    fn source_spec_clone_isolation_preserves_the_compiled_plan() {
        let compiled = Simulator::compile(fixture_scenario()).expect("fixture should compile");
        let expected_scenario_id = compiled.scenario_id().clone();
        let expected_node_ids = compiled.node_ids().to_vec();
        let expected_edge_ids = compiled.edge_ids().to_vec();
        let expected_report = Simulator::run(&compiled, &deterministic_run_config())
            .expect("compiled fixture should run");

        let mut inspection_copy = compiled.source_spec().clone();
        inspection_copy.id = ScenarioId::fixture("mutated-inspection-copy");
        inspection_copy.nodes.clear();
        inspection_copy.edges.clear();

        assert_eq!(compiled.scenario_id(), &expected_scenario_id);
        assert_eq!(compiled.node_ids(), expected_node_ids);
        assert_eq!(compiled.edge_ids(), expected_edge_ids);
        assert_eq!(
            Simulator::run(&compiled, &deterministic_run_config())
                .expect("compiled fixture should remain runnable"),
            expected_report
        );
    }

    #[test]
    fn compile_rejects_collection_key_and_embedded_id_drift() {
        let mut scenario = fixture_scenario();
        let (_, mut edge) = scenario.edges.pop_first().expect("fixture should contain one edge");
        let collection_key = EdgeId::fixture("collection-key");
        edge.id = EdgeId::fixture("embedded-id");
        scenario.edges.insert(collection_key.clone(), edge);

        let error = Simulator::compile(scenario).expect_err("edge ID drift must fail checking");
        assert!(error.to_string().contains("edges.collection-key.id"));
    }

    #[test]
    fn compile_retains_expression_routing_and_metric_slots_by_typed_index() {
        let source = NodeId::fixture("a-source");
        let target = NodeId::fixture("b-target");
        let transfer_edge = EdgeId::fixture("a-transfer");
        let modifier_edge = EdgeId::fixture("m-modifier");
        let trigger_edge = EdgeId::fixture("z-trigger");
        let metric = MetricKey::fixture("b-target");

        let state_connection = |role| EdgeConnectionConfig {
            kind: ConnectionKind::State,
            state: StateConnectionConfig {
                role,
                formula: "+1".to_string(),
                target: StateConnectionTarget::Node,
                target_connection: None,
                resource_filter: None,
            },
            ..EdgeConnectionConfig::default()
        };
        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("compiled-plan-slots"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Pool).with_initial_value(1.0))
            .with_node(NodeSpec::new(target.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                transfer_edge.clone(),
                source.clone(),
                target.clone(),
                TransferSpec::Expression { formula: "from".to_string() },
            ))
            .with_edge(
                EdgeSpec::new(
                    modifier_edge.clone(),
                    source.clone(),
                    target.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(state_connection(StateConnectionRole::Modifier)),
            )
            .with_edge(
                EdgeSpec::new(
                    trigger_edge.clone(),
                    source.clone(),
                    target.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(state_connection(StateConnectionRole::Trigger)),
            );
        scenario.tracked_metrics.insert(metric.clone());

        let compiled = Simulator::compile(scenario).expect("scenario should compile");

        assert_eq!(compiled.edge_ids(), &[transfer_edge, modifier_edge, trigger_edge]);
        assert!(compiled.expressions().transfer(EdgeIndex::new(0)).is_some());
        assert!(compiled.expressions().state(EdgeIndex::new(1)).is_some());
        assert!(compiled.expressions().transfer(EdgeIndex::new(1)).is_none());
        assert!(compiled.expressions().state(EdgeIndex::new(0)).is_none());
        assert!(compiled.expressions().state(EdgeIndex::new(2)).is_none());

        assert_eq!(
            compiled.routing().resource_group(NodeIndex::new(0), TransferControl::PushAny),
            Some([EdgeIndex::new(0)].as_slice())
        );
        assert_eq!(
            compiled.routing().trigger_outputs(NodeIndex::new(0)),
            Some([TriggerTarget::Node(NodeIndex::new(1))].as_slice())
        );
        assert_eq!(compiled.metric_node_index(&metric), Some(1));
        assert_eq!(compiled.tracked_metrics(), [metric]);
    }

    #[test]
    fn dto_and_checked_compile_paths_produce_identical_formula_results() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let scenario = ScenarioSpec::new(ScenarioId::fixture("checked-formula-parity"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(8.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("transfer"),
                source,
                sink,
                TransferSpec::Expression { formula: "from / 2".into() },
            ));

        let dto_compiled = Simulator::compile(scenario.clone()).expect("DTO compile succeeds");
        let checked = Scenario::try_from(scenario).expect("scenario checking succeeds");
        let checked_compiled =
            Simulator::compile_checked(checked).expect("checked compile succeeds");

        let config = deterministic_run_config();
        assert_eq!(
            Simulator::run(&dto_compiled, &config).expect("DTO plan runs"),
            Simulator::run(&checked_compiled, &config).expect("checked plan runs")
        );
    }

    #[test]
    fn disabled_formula_edges_keep_their_deterministic_expression_slots() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let modifier = EdgeConnectionConfig {
            kind: ConnectionKind::State,
            state: StateConnectionConfig {
                role: StateConnectionRole::Modifier,
                formula: "+1".into(),
                target: StateConnectionTarget::Node,
                target_connection: None,
                resource_filter: None,
            },
            ..EdgeConnectionConfig::default()
        };
        let mut transfer_edge = EdgeSpec::new(
            EdgeId::fixture("a-transfer"),
            source.clone(),
            sink.clone(),
            TransferSpec::Expression { formula: "from".into() },
        );
        transfer_edge.enabled = false;
        let mut state_edge = EdgeSpec::new(
            EdgeId::fixture("b-state"),
            source.clone(),
            sink.clone(),
            TransferSpec::Remaining,
        )
        .with_connection(modifier);
        state_edge.enabled = false;
        let scenario = ScenarioSpec::new(ScenarioId::fixture("disabled-formula-slots"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(transfer_edge)
            .with_edge(state_edge);

        let compiled = Simulator::compile(scenario).expect("disabled formulas still validate");
        assert!(compiled.expressions().transfer(EdgeIndex::new(0)).is_some());
        assert!(compiled.expressions().state(EdgeIndex::new(1)).is_some());
    }
}
