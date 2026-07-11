//! Immutable execution projections produced by checked scenario compilation.
//!
//! This module is the sole owner of the runtime layout behind
//! [`CompiledScenario`].  The public handle intentionally exposes only stable
//! inspection accessors; execution modules use the narrow crate-private query
//! methods below instead of rejoining the source maps and derived indexes.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::types::{EdgeId, EdgeSpec, MetricKey, NodeId, NodeSpec, ScenarioId, ScenarioSpec};

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
    metric_index_by_name: BTreeMap<String, NodeIndex>,
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
    metric_index_by_name: BTreeMap<String, NodeIndex>,
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
    ) -> Self {
        Self(Arc::new(ExecutionPlan {
            source_spec,
            node_ids: projections.node_ids,
            edge_ids: projections.edge_ids,
            nodes: projections.nodes,
            edges: projections.edges,
            node_index_by_id: indexes.node_index_by_id,
            edge_index_by_id: indexes.edge_index_by_id,
            metric_index_by_name: indexes.metric_index_by_name,
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

    pub(crate) fn edge(&self, id: &EdgeId) -> Option<&CompiledEdge> {
        self.edge_index(id).and_then(|index| self.0.edges.get(index))
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
        self.0.metric_index_by_name.get(metric.as_str()).map(|index| index.0)
    }

    pub(crate) fn tracked_metrics(&self) -> &BTreeSet<MetricKey> {
        &self.0.source_spec.tracked_metrics
    }

    pub(crate) fn variables(&self) -> &crate::types::VariableRuntimeConfig {
        &self.0.source_spec.variables
    }

    pub(crate) fn end_conditions(&self) -> &[crate::types::EndConditionSpec] {
        &self.0.source_spec.end_conditions
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
        metric_index_by_name: BTreeMap<String, NodeIndex>,
    ) -> Self {
        Self { node_index_by_id, edge_index_by_id, metric_index_by_name }
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

    use super::CompiledScenario;
    use crate::{
        testkit::{deterministic_run_config, fixture_scenario},
        types::{EdgeId, ScenarioId},
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
}
