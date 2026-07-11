//! Immutable execution projections produced by checked scenario compilation.
//!
//! This module is the sole owner of the runtime layout behind
//! [`CompiledScenario`].  The public handle intentionally exposes only stable
//! inspection accessors; execution modules use the narrow crate-private query
//! methods below instead of rejoining the source maps and derived indexes.

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    expr::CompiledExpr,
    types::{
        ActionMode, ConnectionKind, EdgeId, EdgeSpec, MetricKey, NodeConfig, NodeId,
        NodeModeConfig, NodeSpec, ScenarioId, ScenarioSpec, StateConnectionRole,
        StateConnectionTarget,
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
    spec: NodeSpec,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompiledEdge {
    id: EdgeId,
    spec: EdgeSpec,
    from_index: NodeIndex,
    to_index: NodeIndex,
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
        }))
    }

    /// Returns a node whose identity was already resolved by checked compilation.
    ///
    /// Engine callers use this only for IDs derived from the immutable plan; a
    /// missing entry is therefore an invariant breach, never a runtime default.
    pub(crate) fn required_node(&self, id: &NodeId) -> &NodeSpec {
        &self.0.nodes[self.0.node_index_by_id[id].0].spec
    }

    /// Returns a node index whose identity was already resolved by compilation.
    pub(crate) fn required_node_index(&self, id: &NodeId) -> usize {
        self.0.node_index_by_id[id].0
    }

    pub(crate) fn node_id_at(&self, index: usize) -> Option<&NodeId> {
        self.0.nodes.get(index).map(|node| &node.id)
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = (&NodeId, &NodeSpec)> {
        self.0.nodes.iter().map(|node| (&node.id, &node.spec))
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
        &self.0.source_spec.variables
    }

    pub(crate) fn end_conditions(&self) -> &[crate::types::EndConditionSpec] {
        &self.0.source_spec.end_conditions
    }

    pub(crate) fn expressions(&self) -> &CompiledExpressions {
        &self.0.expressions
    }

    pub(crate) fn routing(&self) -> &RoutingPlan {
        &self.0.routing
    }
}

impl CompiledNode {
    pub(crate) fn new(id: NodeId, spec: NodeSpec) -> Self {
        Self { id, spec }
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
    pub(crate) fn from_spec(
        spec: &ScenarioSpec,
        node_index_by_id: &BTreeMap<NodeId, NodeIndex>,
        edge_index_by_id: &BTreeMap<EdgeId, EdgeIndex>,
    ) -> Self {
        let mut resource_groups_by_controller =
            BTreeMap::<NodeIndex, BTreeMap<TransferControl, Vec<EdgeIndex>>>::new();
        let mut passive_state_triggers = Vec::new();
        let mut trigger_outputs_by_source = BTreeMap::<NodeIndex, Vec<TriggerTarget>>::new();

        for (edge_id, edge) in &spec.edges {
            if !edge.enabled {
                continue;
            }
            let edge_index = edge_index_by_id[edge_id];
            let source = node_index_by_id[&edge.from];
            let target = node_index_by_id[&edge.to];
            if matches!(edge.connection.kind, ConnectionKind::Resource) {
                let target_action = normalized_action_mode(action_mode_for_node(spec, &edge.to));
                let (controller, control) = match target_action {
                    TransferControl::PullAny | TransferControl::PullAll => (target, target_action),
                    TransferControl::PushAny | TransferControl::PushAll => {
                        (source, normalized_action_mode(action_mode_for_node(spec, &edge.from)))
                    }
                };
                resource_groups_by_controller
                    .entry(controller)
                    .or_default()
                    .entry(control)
                    .or_default()
                    .push(edge_index);
            }
            if !matches!(edge.connection.kind, ConnectionKind::State)
                || !matches!(edge.connection.state.role, StateConnectionRole::Trigger)
            {
                continue;
            }
            let targets = trigger_targets(
                &edge.connection.state.target,
                edge.connection.state.target_connection.as_ref(),
                target,
                edge_index_by_id,
            );
            trigger_outputs_by_source.entry(source).or_default().extend(targets.iter().copied());
            if !is_trigger_gate(spec, &edge.from) {
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

fn action_mode_for_node(spec: &ScenarioSpec, node_id: &NodeId) -> ActionMode {
    let node = &spec.nodes[node_id];
    node_mode(&node.config).map(|mode| mode.action_mode.clone()).unwrap_or(ActionMode::PushAny)
}

fn node_mode(config: &NodeConfig) -> Option<&NodeModeConfig> {
    match config {
        NodeConfig::Pool(config) => Some(&config.mode),
        NodeConfig::Drain(config) => Some(&config.mode),
        NodeConfig::SortingGate(config) => Some(&config.mode),
        NodeConfig::TriggerGate(config) => Some(&config.mode),
        NodeConfig::MixedGate(config) => Some(&config.mode),
        NodeConfig::Converter(config) => Some(&config.mode),
        NodeConfig::Trader(config) => Some(&config.mode),
        NodeConfig::Delay(config) => Some(&config.mode),
        NodeConfig::Queue(config) => Some(&config.mode),
        NodeConfig::None | NodeConfig::Register(_) => None,
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

fn is_trigger_gate(spec: &ScenarioSpec, node_id: &NodeId) -> bool {
    matches!(spec.nodes[node_id].config, NodeConfig::TriggerGate(_) | NodeConfig::MixedGate(_))
}

fn trigger_targets(
    target: &StateConnectionTarget,
    target_connection: Option<&EdgeId>,
    node_target: NodeIndex,
    edge_index_by_id: &BTreeMap<EdgeId, EdgeIndex>,
) -> Vec<TriggerTarget> {
    match target {
        StateConnectionTarget::Node => vec![TriggerTarget::Node(node_target)],
        StateConnectionTarget::ResourceConnection | StateConnectionTarget::StateConnection => {
            target_connection
                .and_then(|id| edge_index_by_id.get(id).copied())
                .map(TriggerTarget::Edge)
                .into_iter()
                .collect()
        }
        StateConnectionTarget::Formula => Vec::new(),
    }
}

impl CompiledEdge {
    pub(crate) fn new(id: EdgeId, spec: EdgeSpec, from_index: usize, to_index: usize) -> Self {
        Self { id, spec, from_index: NodeIndex(from_index), to_index: NodeIndex(to_index) }
    }

    pub(crate) fn id(&self) -> &EdgeId {
        &self.id
    }

    pub(crate) fn spec(&self) -> &EdgeSpec {
        &self.spec
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
            NodeSpec, ScenarioId, ScenarioSpec, StateConnectionConfig, StateConnectionRole,
            StateConnectionTarget, TransferSpec,
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
    fn compiled_edge_uses_collection_key_when_embedded_id_differs() {
        let mut scenario = fixture_scenario();
        let (_, mut edge) = scenario.edges.pop_first().expect("fixture should contain one edge");
        let collection_key = EdgeId::fixture("collection-key");
        edge.id = EdgeId::fixture("embedded-id");
        scenario.edges.insert(collection_key.clone(), edge);

        let compiled = Simulator::compile(scenario)
            .expect("collection and embedded IDs may differ in this migration slice");
        let report = Simulator::run(&compiled, &deterministic_run_config())
            .expect("compiled mismatched-key fixture should run");

        assert_eq!(compiled.edge_ids(), std::slice::from_ref(&collection_key));
        assert_eq!(report.transfers[0].edge_id, collection_key);
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
}
