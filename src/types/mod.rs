//! Core domain types for scenario definitions, execution reports, and artifacts.

mod artifacts;
mod config;
mod identifiers;
mod reports;
mod scenario;

pub use artifacts::*;
pub use config::*;
pub use identifiers::*;
pub use reports::*;
pub use scenario::*;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::Simulator;

    #[test]
    fn identifiers_validate_empty_and_control_values() {
        let empty = ScenarioId::new("   ");
        assert!(matches!(empty, Err(IdentifierError::Empty { .. })));

        let contains_control = NodeId::new("a\nb");
        assert!(matches!(contains_control, Err(IdentifierError::ContainsControl { .. })));
    }

    #[test]
    fn node_kind_supports_new_and_legacy_families() {
        assert_eq!(
            serde_json::from_str::<NodeKind>("\"pool\"").expect("pool kind should deserialize"),
            NodeKind::Pool
        );
        assert_eq!(
            serde_json::from_str::<NodeKind>("\"trigger_gate\"")
                .expect("trigger_gate kind should deserialize"),
            NodeKind::TriggerGate
        );
        assert_eq!(
            serde_json::from_str::<NodeKind>("\"process\"")
                .expect("legacy process kind should deserialize"),
            NodeKind::Process
        );
        assert_eq!(
            serde_json::to_string(&NodeKind::Queue).expect("queue kind should serialize"),
            "\"queue\""
        );
        assert_eq!(
            serde_json::to_string(&NodeKind::Gate).expect("legacy gate kind should serialize"),
            "\"gate\""
        );
    }

    #[test]
    fn node_mode_defaults_and_action_aliases_are_stable() {
        let defaults = NodeModeConfig::default();
        assert_eq!(defaults.trigger_mode, TriggerMode::Automatic);
        assert_eq!(defaults.action_mode, ActionMode::PushAny);

        let trigger: TriggerMode =
            serde_json::from_str("\"automatic\"").expect("automatic trigger mode should parse");
        assert_eq!(trigger, TriggerMode::Automatic);

        let action: ActionMode =
            serde_json::from_str("\"pull-any\"").expect("kebab-case alias should deserialize");
        assert_eq!(action, ActionMode::PullAny);

        let encoded = serde_json::to_string(&ActionMode::PushAll)
            .expect("action mode should serialize in snake_case");
        assert_eq!(encoded, "\"push_all\"");
    }

    #[test]
    fn node_spec_defaults_config_when_missing() {
        let encoded = r#"{
            "id":"pool-a",
            "kind":"pool",
            "label":null,
            "initial_value":0.0,
            "tags":[],
            "metadata":{}
        }"#;
        let node: NodeSpec =
            serde_json::from_str(encoded).expect("node spec should deserialize without config");
        assert_eq!(node.config, NodeConfig::None);
    }

    #[test]
    fn node_spec_round_trips_typed_config() {
        let node = NodeSpec::new(NodeId::fixture("pool-a"), NodeKind::Pool).with_config(
            NodeConfig::Pool(PoolNodeConfig {
                capacity: Some(10),
                allow_negative_start: true,
                mode: NodeModeConfig {
                    trigger_mode: TriggerMode::Passive,
                    action_mode: ActionMode::PullAll,
                },
            }),
        );

        let encoded = serde_json::to_string(&node).expect("node spec should serialize");
        let decoded: NodeSpec =
            serde_json::from_str(&encoded).expect("node spec should deserialize");

        assert_eq!(decoded, node);
    }

    #[test]
    fn edge_spec_defaults_resource_connection_when_missing() {
        let encoded = r#"{
            "id":"edge-a",
            "from":"source",
            "to":"sink",
            "transfer":"remaining",
            "enabled":true,
            "metadata":{}
        }"#;
        let edge: EdgeSpec =
            serde_json::from_str(encoded).expect("legacy edge spec should deserialize");

        assert_eq!(edge.connection.kind, ConnectionKind::Resource);
        assert_eq!(edge.connection.resource.token_size, 1);
        assert_eq!(edge.connection.state.formula, "+1");

        let reencoded = serde_json::to_value(&edge).expect("edge spec should serialize");
        assert!(
            reencoded.get("connection").is_none(),
            "default resource connection should preserve legacy payload shape"
        );
    }

    #[test]
    fn edge_spec_state_connection_defaults_formula_and_accepts_legacy_aliases() {
        let encoded = r#"{
            "id":"edge-state",
            "from":"source",
            "to":"sink",
            "transfer":"remaining",
            "enabled":true,
            "metadata":{},
            "connection":{
                "kind":"state",
                "state":{
                    "role":"modifier",
                    "target":"formula",
                    "target_edge":"edge-resource",
                    "filter":"gold"
                }
            }
        }"#;
        let edge: EdgeSpec =
            serde_json::from_str(encoded).expect("state edge spec should deserialize");

        assert_eq!(edge.connection.kind, ConnectionKind::State);
        assert_eq!(edge.connection.state.role, StateConnectionRole::Modifier);
        assert_eq!(edge.connection.state.formula, "+1");
        assert_eq!(edge.connection.state.target, StateConnectionTarget::Formula);
        assert_eq!(edge.connection.state.target_connection, Some(EdgeId::fixture("edge-resource")));
        assert_eq!(edge.connection.state.resource_filter.as_deref(), Some("gold"));
    }

    #[test]
    fn edge_spec_round_trips_explicit_connection_semantics() {
        let edge = EdgeSpec::new(
            EdgeId::fixture("edge-state"),
            NodeId::fixture("source"),
            NodeId::fixture("sink"),
            TransferSpec::Remaining,
        )
        .with_connection(EdgeConnectionConfig {
            kind: ConnectionKind::State,
            resource: ResourceConnectionConfig { token_size: 1 },
            state: StateConnectionConfig {
                role: StateConnectionRole::Activator,
                formula: "*".to_string(),
                target: StateConnectionTarget::ResourceConnection,
                target_connection: Some(EdgeId::fixture("edge-resource")),
                resource_filter: None,
            },
        });

        let encoded = serde_json::to_string(&edge).expect("edge spec should serialize");
        let decoded: EdgeSpec =
            serde_json::from_str(&encoded).expect("edge spec should deserialize");

        assert_eq!(decoded, edge);
    }

    #[test]
    fn transfer_spec_expression_round_trips_formula_payload() {
        let transfer = TransferSpec::Expression { formula: "max(1, step + 1)".to_string() };

        let encoded = serde_json::to_string(&transfer).expect("transfer spec should serialize");
        let decoded: TransferSpec =
            serde_json::from_str(&encoded).expect("transfer spec should deserialize");

        assert_eq!(decoded, transfer);
    }

    #[test]
    fn scenario_spec_defaults_variable_runtime_config() {
        let scenario = ScenarioSpec::new(ScenarioId::fixture("scenario"));
        assert_eq!(scenario.variables.update_timing, VariableUpdateTiming::EveryStep);
        assert!(scenario.variables.sources.is_empty());
        assert_eq!(scenario.end_conditions, vec![EndConditionSpec::MaxSteps { steps: 1 }]);
    }

    #[test]
    fn scenario_end_condition_builders_replace_and_append_deterministically() {
        let replaced = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });
        assert_eq!(replaced.end_conditions, vec![EndConditionSpec::MaxSteps { steps: 3 }]);

        let explicit = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_end_conditions([
            EndConditionSpec::MaxSteps { steps: 4 },
            EndConditionSpec::MaxSteps { steps: 9 },
        ]);
        assert_eq!(
            explicit.end_conditions,
            vec![EndConditionSpec::MaxSteps { steps: 4 }, EndConditionSpec::MaxSteps { steps: 9 }]
        );

        let appended = replaced.push_end_condition(EndConditionSpec::NodeAtLeast {
            node_id: NodeId::fixture("sink"),
            value_scaled: 1,
        });
        assert_eq!(appended.end_conditions.len(), 2);
    }

    #[test]
    fn variable_runtime_config_round_trips_typed_sources() {
        let mut sources = BTreeMap::new();
        sources.insert("roll".to_string(), VariableSourceSpec::RandomInterval { min: 1, max: 6 });
        sources.insert(
            "table".to_string(),
            VariableSourceSpec::RandomMatrix { values: vec![vec![1.0], vec![2.0, 3.0]] },
        );
        let config =
            VariableRuntimeConfig { update_timing: VariableUpdateTiming::RunStart, sources };

        let encoded =
            serde_json::to_string(&config).expect("variable runtime config should serialize");
        let decoded: VariableRuntimeConfig =
            serde_json::from_str(&encoded).expect("variable runtime config should deserialize");

        assert_eq!(decoded, config);
    }

    #[test]
    fn scenario_builder_keeps_deterministic_ordering() {
        let scenario = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(NodeId::fixture("z-node"), NodeKind::Process))
            .with_node(NodeSpec::new(NodeId::fixture("a-node"), NodeKind::Source))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-z"),
                NodeId::fixture("z-node"),
                NodeId::fixture("a-node"),
                TransferSpec::Remaining,
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                NodeId::fixture("a-node"),
                NodeId::fixture("z-node"),
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let node_ids: Vec<&str> = scenario.nodes.keys().map(NodeId::as_str).collect();
        assert_eq!(node_ids, vec!["a-node", "z-node"]);

        let edge_ids: Vec<&str> = scenario.edges.keys().map(EdgeId::as_str).collect();
        assert_eq!(edge_ids, vec!["edge-a", "edge-z"]);
    }

    #[test]
    fn scenario_source_sink_constructor_reduces_setup_boilerplate() {
        let scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 2.0 });
        assert_eq!(scenario.id.as_str(), "scenario-source-sink");

        let source = scenario.nodes.get(&NodeId::fixture("source")).expect("source node");
        assert_eq!(source.kind, NodeKind::Source);
        assert_eq!(source.initial_value, 1.0);

        let sink = scenario.nodes.get(&NodeId::fixture("sink")).expect("sink node");
        assert_eq!(sink.kind, NodeKind::Sink);

        let edge = scenario.edges.get(&EdgeId::fixture("edge-source-sink")).expect("edge");
        assert_eq!(edge.transfer, TransferSpec::Fixed { amount: 2.0 });
        assert!(scenario.tracked_metrics.contains(&MetricKey::fixture("sink")));
    }

    #[test]
    fn scenario_linear_pipeline_constructor_builds_valid_chain() {
        let scenario = ScenarioSpec::linear_pipeline(4);
        assert_eq!(scenario.id.as_str(), "scenario-linear-pipeline");
        assert_eq!(scenario.nodes.len(), 4);
        assert_eq!(scenario.edges.len(), 3);
        assert!(scenario.nodes.contains_key(&NodeId::fixture("source")));
        assert!(scenario.nodes.contains_key(&NodeId::fixture("stage-1")));
        assert!(scenario.nodes.contains_key(&NodeId::fixture("stage-2")));
        assert!(scenario.nodes.contains_key(&NodeId::fixture("sink")));
        assert!(scenario.edges.contains_key(&EdgeId::fixture("edge-0")));
        assert!(scenario.edges.contains_key(&EdgeId::fixture("edge-1")));
        assert!(scenario.edges.contains_key(&EdgeId::fixture("edge-2")));
        assert!(scenario.tracked_metrics.contains(&MetricKey::fixture("sink")));
    }

    #[test]
    fn scenario_convenience_constructors_are_compile_and_run_ready() {
        for scenario in [
            ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 }),
            ScenarioSpec::linear_pipeline(3),
            ScenarioSpec::linear_pipeline(1),
        ] {
            let compiled =
                Simulator::compile(scenario).expect("constructor scenario should compile");
            let run = Simulator::run(&compiled, &RunConfig::for_seed(42))
                .expect("constructor scenario should run");
            assert!(run.completed);
        }
    }

    #[test]
    fn capture_and_run_defaults_are_stable() {
        let capture = CaptureConfig::default();
        assert_eq!(capture.every_n_steps, 1);
        assert!(capture.include_step_zero);
        assert!(capture.include_final_state);
        assert!(capture.capture_nodes.is_empty());
        assert!(capture.capture_metrics.is_empty());

        let run = RunConfig::for_seed(99);
        assert_eq!(run.seed, 99);
        assert_eq!(run.max_steps, 100);
        assert_eq!(run.capture, CaptureConfig::default());

        assert_eq!(ConfidenceLevel::default(), ConfidenceLevel::P95);
    }

    #[test]
    fn run_config_builders_override_defaults() {
        let capture = CaptureConfig {
            every_n_steps: 5,
            include_step_zero: false,
            include_final_state: false,
            ..CaptureConfig::default()
        };
        let run = RunConfig::for_seed(77).with_max_steps(250).with_capture(capture.clone());

        assert_eq!(run.seed, 77);
        assert_eq!(run.max_steps, 250);
        assert_eq!(run.capture, capture);
    }

    #[test]
    fn batch_config_builders_update_execution_and_run_template() {
        let capture = CaptureConfig {
            every_n_steps: 3,
            include_step_zero: false,
            include_final_state: true,
            ..CaptureConfig::default()
        };
        let batch = BatchConfig::for_runs(12)
            .with_execution_mode(ExecutionMode::SingleThread)
            .with_base_seed(991)
            .with_run_template(BatchRunTemplate::default())
            .with_max_steps(40)
            .with_capture(capture.clone());

        assert_eq!(batch.runs, 12);
        assert_eq!(batch.base_seed, 991);
        assert_eq!(batch.execution_mode, ExecutionMode::SingleThread);
        assert_eq!(batch.run_template.max_steps, 40);
        assert_eq!(batch.run_template.capture, capture);
    }

    #[test]
    fn series_and_batch_helpers_keep_stable_order() {
        let table = SeriesTable::new(MetricKey::fixture("throughput"))
            .with_point(SeriesPoint::new(8, 2.0))
            .with_point(SeriesPoint::new(2, 1.0))
            .with_point(SeriesPoint::new(5, 1.5));
        let steps: Vec<u64> = table.points.iter().map(|p| p.step).collect();
        assert_eq!(steps, vec![2, 5, 8]);

        let report =
            BatchReport::new(ScenarioId::fixture("scenario"), 2, ExecutionMode::SingleThread)
                .push_run(BatchRunSummary {
                    run_index: 9,
                    seed: 9,
                    completed: true,
                    steps_executed: 20,
                    final_metrics: BTreeMap::new(),
                    manifest: None,
                })
                .push_run(BatchRunSummary {
                    run_index: 1,
                    seed: 1,
                    completed: true,
                    steps_executed: 20,
                    final_metrics: BTreeMap::new(),
                    manifest: None,
                });

        let run_indexes: Vec<u64> = report.runs.iter().map(|run| run.run_index).collect();
        assert_eq!(run_indexes, vec![1, 9]);
        assert_eq!(report.completed_runs, 2);
    }

    #[test]
    fn manifest_refs_use_sorted_artifact_keys() {
        let manifest = ManifestRef::new(ScenarioId::fixture("scenario"))
            .with_artifact("zeta", ArtifactRef::new(ArtifactKind::Series, "series/zeta.json"))
            .with_artifact("alpha", ArtifactRef::new(ArtifactKind::Summary, "summary/alpha.json"));

        let keys: Vec<&str> = manifest.artifacts.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["alpha", "zeta"]);
        assert_eq!(manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
    }

    #[test]
    fn prediction_artifact_kind_serializes_snake_case() {
        let encoded = serde_json::to_string(&ArtifactKind::Prediction)
            .expect("prediction artifact kind should serialize");
        assert_eq!(encoded, "\"prediction\"");
    }

    #[test]
    fn history_and_replay_artifact_kinds_serialize_snake_case() {
        let history = serde_json::to_string(&ArtifactKind::HistoryIndex)
            .expect("history artifact kind should serialize");
        let replay = serde_json::to_string(&ArtifactKind::ReplayIndex)
            .expect("replay artifact kind should serialize");
        assert_eq!(history, "\"history_index\"");
        assert_eq!(replay, "\"replay_index\"");
    }

    #[test]
    fn prediction_summary_report_keeps_deterministic_metric_order() {
        let report = PredictionSummaryReport::new(
            ScenarioId::fixture("scenario"),
            BTreeMap::from([
                (
                    MetricKey::fixture("beta"),
                    PredictionMetricIndicators {
                        n: 2,
                        mean: 2.0,
                        variance: 1.0,
                        std_dev: 1.0,
                        min: 1.0,
                        max: 3.0,
                        median: 2.0,
                        p90: 2.8,
                        p95: 2.9,
                        p99: 2.98,
                        confidence_lower_95: 0.0,
                        confidence_upper_95: 4.0,
                        confidence_margin_95: 2.0,
                        confidence_lower_selected: 0.0,
                        confidence_upper_selected: 4.0,
                        confidence_margin_selected: 2.0,
                        reliability_score: 0.5,
                        convergence_delta: 1.0,
                        convergence_ratio: 0.5,
                    },
                ),
                (
                    MetricKey::fixture("alpha"),
                    PredictionMetricIndicators {
                        n: 2,
                        mean: 10.0,
                        variance: 0.0,
                        std_dev: 0.0,
                        min: 10.0,
                        max: 10.0,
                        median: 10.0,
                        p90: 10.0,
                        p95: 10.0,
                        p99: 10.0,
                        confidence_lower_95: 10.0,
                        confidence_upper_95: 10.0,
                        confidence_margin_95: 0.0,
                        confidence_lower_selected: 10.0,
                        confidence_upper_selected: 10.0,
                        confidence_margin_selected: 0.0,
                        reliability_score: 1.0,
                        convergence_delta: 0.0,
                        convergence_ratio: 0.0,
                    },
                ),
            ]),
        );

        let encoded = serde_json::to_string(&report).expect("prediction report should serialize");
        let decoded: PredictionSummaryReport =
            serde_json::from_str(&encoded).expect("prediction report should deserialize");
        assert_eq!(decoded.selected_confidence_level, ConfidenceLevel::P95);
        let keys: Vec<&str> = decoded.metrics.keys().map(MetricKey::as_str).collect();
        assert_eq!(keys, vec!["alpha", "beta"]);
    }

    #[test]
    fn history_index_report_round_trips_typed_diagnostics() {
        let mut report = HistoryIndexReport::new(ScenarioId::fixture("scenario"), "events.jsonl");
        report.entries = vec![
            HistoryIndexEntry {
                event_index: 0,
                run_id: "run-a".to_string(),
                step: 0,
                phase: "step_start".to_string(),
                ordinal: 0,
                event: "step_start".to_string(),
                severity: None,
                code: None,
            },
            HistoryIndexEntry {
                event_index: 1,
                run_id: "run-a".to_string(),
                step: 0,
                phase: "violation".to_string(),
                ordinal: 1,
                event: "violation".to_string(),
                severity: Some(DiagnosticSeverity::Warning),
                code: Some("RULE-001".to_string()),
            },
        ];

        let encoded = serde_json::to_string(&report).expect("history report should serialize");
        let decoded: HistoryIndexReport =
            serde_json::from_str(&encoded).expect("history report should deserialize");
        assert_eq!(decoded, report);
    }

    #[test]
    fn replay_index_report_keeps_sorted_runs() {
        let mut report = ReplayIndexReport::new(ScenarioId::fixture("scenario"), "events.jsonl", 3);
        report.runs.insert(
            "run-z".to_string(),
            ReplayRunIndex {
                first_event_index: 2,
                last_event_index: 3,
                first_step: 1,
                last_step: 1,
            },
        );
        report.runs.insert(
            "run-a".to_string(),
            ReplayRunIndex {
                first_event_index: 0,
                last_event_index: 1,
                first_step: 0,
                last_step: 1,
            },
        );

        let keys: Vec<&str> = report.runs.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["run-a", "run-z"]);
    }

    #[test]
    fn manifest_ref_upgrade_compat_adds_v2_version_and_sections() {
        let legacy = ManifestRef {
            schema_version: ARTIFACT_SCHEMA_VERSION_V1,
            setup_hash: "".to_string(),
            seed_strategy: "".to_string(),
            crate_version: "".to_string(),
            generated_at_unix_seconds: 0,
            scenario_id: ScenarioId::fixture("scenario"),
            artifacts: BTreeMap::from([
                ("events".to_string(), ArtifactRef::new(ArtifactKind::EventLog, "events.jsonl")),
                (
                    "history".to_string(),
                    ArtifactRef::new(ArtifactKind::HistoryIndex, "history.json"),
                ),
                (
                    "prediction".to_string(),
                    ArtifactRef::new(ArtifactKind::Prediction, "prediction.json"),
                ),
            ]),
            sections: ArtifactSchemaSections::default(),
        };

        let upgraded = legacy.upgrade_compat();
        assert_eq!(upgraded.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
        assert_eq!(upgraded.setup_hash, "unknown");
        assert_eq!(upgraded.seed_strategy, "unknown");
        assert_eq!(upgraded.crate_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            upgraded.sections.accuracy,
            Some(ArtifactSectionRef::new("prediction", ArtifactKind::Prediction))
        );
        assert_eq!(
            upgraded.sections.debug,
            Some(ArtifactSectionRef::new("events", ArtifactKind::EventLog))
        );
        assert_eq!(
            upgraded.sections.history,
            Some(ArtifactSectionRef::new("history", ArtifactKind::HistoryIndex))
        );
    }

    #[test]
    fn manifest_ref_deserializes_v1_without_schema_fields() {
        let raw = r#"{
            "scenario_id":"scenario-v1",
            "artifacts":{
                "summary":{"kind":"summary","path":"summary.csv","content_type":"text/csv"}
            }
        }"#;
        let decoded: ManifestRef =
            serde_json::from_str(raw).expect("legacy v1 manifest should deserialize");
        assert_eq!(decoded.schema_version, ARTIFACT_SCHEMA_VERSION_V1);
        assert_eq!(decoded.setup_hash, "unknown");
        assert_eq!(decoded.seed_strategy, "unknown");
        assert_eq!(decoded.crate_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(decoded.generated_at_unix_seconds, 0);
        assert!(decoded.sections.is_empty());
        assert!(decoded.artifacts.contains_key("summary"));
    }
}
