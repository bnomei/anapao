use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ConfidenceLevel, MetricKey, ScenarioId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Artifact type labels used in manifests.
pub enum ArtifactKind {
    Manifest,
    EventLog,
    VariableSeries,
    AssertionReport,
    HistoryIndex,
    ReplayIndex,
    Series,
    Summary,
    Prediction,
    Custom(String),
}

/// Legacy artifact schema version used by persisted compatibility readers.
pub const ARTIFACT_SCHEMA_VERSION_V1: u32 = 1;
/// Current artifact schema version for newly-written manifests.
pub const ARTIFACT_SCHEMA_VERSION_V2: u32 = 2;

fn default_artifact_schema_version() -> u32 {
    ARTIFACT_SCHEMA_VERSION_V1
}

fn default_manifest_setup_hash() -> String {
    "unknown".to_string()
}

fn default_manifest_seed_strategy() -> String {
    "unknown".to_string()
}

fn default_manifest_crate_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_manifest_generated_at_unix_seconds() -> u64 {
    0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Manifest entry referencing one concrete artifact file.
pub struct ArtifactRef {
    pub kind: ArtifactKind,
    pub path: String,
    pub content_type: Option<String>,
}

impl ArtifactRef {
    /// Creates an artifact reference from kind and relative path.
    pub fn new(kind: ArtifactKind, path: impl Into<String>) -> Self {
        Self { kind, path: path.into(), content_type: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Section pointer into a named artifact in the manifest.
pub struct ArtifactSectionRef {
    pub artifact_key: String,
    pub kind: ArtifactKind,
}

impl ArtifactSectionRef {
    /// Creates a section pointer.
    pub fn new(artifact_key: impl Into<String>, kind: ArtifactKind) -> Self {
        Self { artifact_key: artifact_key.into(), kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Optional manifest section index for downstream tooling.
pub struct ArtifactSchemaSections {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertions: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay: Option<ArtifactSectionRef>,
}

impl ArtifactSchemaSections {
    /// Returns true when no sections are currently linked.
    pub fn is_empty(&self) -> bool {
        self.accuracy.is_none()
            && self.assertions.is_none()
            && self.debug.is_none()
            && self.history.is_none()
            && self.replay.is_none()
    }

    /// Heuristically infers canonical section links from known artifact kinds.
    pub fn infer_from_artifacts(artifacts: &BTreeMap<String, ArtifactRef>) -> Self {
        let accuracy = find_section(artifacts, &[ArtifactKind::Prediction, ArtifactKind::Summary]);
        let assertions = find_section(artifacts, &[ArtifactKind::AssertionReport]);
        let debug = find_section(artifacts, &[ArtifactKind::EventLog]);
        let history = find_section(artifacts, &[ArtifactKind::HistoryIndex]);
        let replay = find_section(artifacts, &[ArtifactKind::ReplayIndex]);

        Self { accuracy, assertions, debug, history, replay }
    }
}

fn find_section(
    artifacts: &BTreeMap<String, ArtifactRef>,
    preferred_kinds: &[ArtifactKind],
) -> Option<ArtifactSectionRef> {
    for preferred_kind in preferred_kinds {
        for (key, artifact) in artifacts {
            if &artifact.kind == preferred_kind {
                return Some(ArtifactSectionRef::new(key.clone(), artifact.kind.clone()));
            }
        }
    }

    None
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Prediction diagnostics computed from batch metric samples.
pub struct PredictionMetricIndicators {
    pub n: usize,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub confidence_lower_95: f64,
    pub confidence_upper_95: f64,
    pub confidence_margin_95: f64,
    pub confidence_lower_selected: f64,
    pub confidence_upper_selected: f64,
    pub confidence_margin_selected: f64,
    pub reliability_score: f64,
    pub convergence_delta: f64,
    pub convergence_ratio: f64,
}

fn default_prediction_selected_confidence_level() -> ConfidenceLevel {
    ConfidenceLevel::default()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Aggregated prediction diagnostics for one scenario.
pub struct PredictionSummaryReport {
    pub scenario_id: ScenarioId,
    #[serde(default = "default_prediction_selected_confidence_level")]
    pub selected_confidence_level: ConfidenceLevel,
    pub metrics: BTreeMap<MetricKey, PredictionMetricIndicators>,
}

impl PredictionSummaryReport {
    /// Creates a prediction summary report.
    pub fn new(
        scenario_id: ScenarioId,
        metrics: BTreeMap<MetricKey, PredictionMetricIndicators>,
    ) -> Self {
        Self::new_with_confidence_level(
            scenario_id,
            default_prediction_selected_confidence_level(),
            metrics,
        )
    }

    /// Creates a prediction summary report for an explicit confidence level.
    pub fn new_with_confidence_level(
        scenario_id: ScenarioId,
        selected_confidence_level: ConfidenceLevel,
        metrics: BTreeMap<MetricKey, PredictionMetricIndicators>,
    ) -> Self {
        Self { scenario_id, selected_confidence_level, metrics }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Diagnostic severity levels used by violation/debug artifacts.
pub enum DiagnosticSeverity {
    Debug,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Flattened event index entry optimized for history browsing.
pub struct HistoryIndexEntry {
    pub event_index: u64,
    pub run_id: String,
    pub step: u64,
    pub phase: String,
    pub ordinal: u64,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<DiagnosticSeverity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Indexed event history for one scenario source.
pub struct HistoryIndexReport {
    pub scenario_id: ScenarioId,
    pub source: String,
    pub entries: Vec<HistoryIndexEntry>,
}

impl HistoryIndexReport {
    /// Creates an empty history index report.
    pub fn new(scenario_id: ScenarioId, source: impl Into<String>) -> Self {
        Self { scenario_id, source: source.into(), entries: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Replay span metadata for a single run id.
pub struct ReplayRunIndex {
    pub first_event_index: u64,
    pub last_event_index: u64,
    pub first_step: u64,
    pub last_step: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Replay index grouped by run id.
pub struct ReplayIndexReport {
    pub scenario_id: ScenarioId,
    pub source: String,
    pub event_count: u64,
    pub runs: BTreeMap<String, ReplayRunIndex>,
}

impl ReplayIndexReport {
    /// Creates an empty replay index with known event count.
    pub fn new(scenario_id: ScenarioId, source: impl Into<String>, event_count: u64) -> Self {
        Self { scenario_id, source: source.into(), event_count, runs: BTreeMap::new() }
    }
}

/// Stable manifest for run/batch artifacts.
///
/// `ManifestRef` is intended as the machine-readable index for CI or downstream tooling.
/// It binds scenario identity, seed strategy metadata, and typed artifact references.
///
/// # Example
/// ```rust
/// use anapao::types::{ArtifactKind, ArtifactRef, ManifestRef, ScenarioId};
///
/// let manifest = ManifestRef::new(ScenarioId::fixture("scenario-doc"))
///     .with_seed_strategy("single(seed=42)")
///     .with_generated_at_unix_seconds(42)
///     .with_artifact(
///         "events",
///         ArtifactRef::new(ArtifactKind::EventLog, "events.jsonl"),
///     )
///     .with_inferred_sections();
///
/// assert!(manifest.artifacts.contains_key("events"));
/// assert_eq!(manifest.seed_strategy, "single(seed=42)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestRef {
    #[serde(default = "default_artifact_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_manifest_setup_hash")]
    pub setup_hash: String,
    #[serde(default = "default_manifest_seed_strategy")]
    pub seed_strategy: String,
    #[serde(default = "default_manifest_crate_version")]
    pub crate_version: String,
    #[serde(default = "default_manifest_generated_at_unix_seconds")]
    pub generated_at_unix_seconds: u64,
    pub scenario_id: ScenarioId,
    pub artifacts: BTreeMap<String, ArtifactRef>,
    #[serde(default, skip_serializing_if = "ArtifactSchemaSections::is_empty")]
    pub sections: ArtifactSchemaSections,
}

impl ManifestRef {
    /// Creates a manifest with current schema defaults.
    pub fn new(scenario_id: ScenarioId) -> Self {
        Self {
            schema_version: ARTIFACT_SCHEMA_VERSION_V2,
            setup_hash: default_manifest_setup_hash(),
            seed_strategy: default_manifest_seed_strategy(),
            crate_version: default_manifest_crate_version(),
            generated_at_unix_seconds: default_manifest_generated_at_unix_seconds(),
            scenario_id,
            artifacts: BTreeMap::new(),
            sections: ArtifactSchemaSections::default(),
        }
    }

    /// Adds or replaces a named artifact reference.
    pub fn with_artifact(mut self, key: impl Into<String>, artifact: ArtifactRef) -> Self {
        self.artifacts.insert(key.into(), artifact);
        self
    }

    /// Replaces the optional section mapping.
    pub fn with_sections(mut self, sections: ArtifactSchemaSections) -> Self {
        self.sections = sections;
        self
    }

    /// Infers and sets sections from current artifact map.
    pub fn with_inferred_sections(mut self) -> Self {
        self.sections = ArtifactSchemaSections::infer_from_artifacts(&self.artifacts);
        self
    }

    /// Sets setup-hash metadata.
    pub fn with_setup_hash(mut self, setup_hash: impl Into<String>) -> Self {
        self.setup_hash = setup_hash.into();
        self
    }

    /// Sets seed-strategy metadata.
    pub fn with_seed_strategy(mut self, seed_strategy: impl Into<String>) -> Self {
        self.seed_strategy = seed_strategy.into();
        self
    }

    /// Sets generation timestamp metadata.
    pub fn with_generated_at_unix_seconds(mut self, generated_at_unix_seconds: u64) -> Self {
        self.generated_at_unix_seconds = generated_at_unix_seconds;
        self
    }

    /// Upgrades older manifest payloads to current compatibility defaults.
    pub fn upgrade_compat(mut self) -> Self {
        if self.schema_version < ARTIFACT_SCHEMA_VERSION_V2 {
            self.schema_version = ARTIFACT_SCHEMA_VERSION_V2;
        }
        if self.setup_hash.is_empty() {
            self.setup_hash = default_manifest_setup_hash();
        }
        if self.seed_strategy.is_empty() {
            self.seed_strategy = default_manifest_seed_strategy();
        }
        if self.crate_version.is_empty() {
            self.crate_version = default_manifest_crate_version();
        }
        if self.sections.is_empty() {
            self.sections = ArtifactSchemaSections::infer_from_artifacts(&self.artifacts);
        }
        self
    }
}
