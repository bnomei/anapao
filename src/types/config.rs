use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{MetricKey, NodeId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Execution strategy for batch runs.
pub enum ExecutionMode {
    #[default]
    SingleThread,
    Rayon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Supported confidence levels for prediction interval diagnostics.
pub enum ConfidenceLevel {
    P90,
    #[default]
    P95,
    P99,
}

impl ConfidenceLevel {
    /// Returns the two-sided normal-distribution Z-score for this confidence level.
    pub fn z_score(self) -> f64 {
        match self {
            Self::P90 => 1.644_853_626_951_472_2,
            Self::P95 => 1.959_963_984_540_054,
            Self::P99 => 2.575_829_303_548_900_4,
        }
    }
}

/// Capture policy for per-step node and metric snapshots.
///
/// Use [`CaptureConfig::default`] for debugging/analysis-friendly traces
/// or [`CaptureConfig::disabled`] for throughput-oriented runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub capture_nodes: BTreeSet<NodeId>,
    pub capture_metrics: BTreeSet<MetricKey>,
    pub every_n_steps: u64,
    pub include_step_zero: bool,
    pub include_final_state: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            capture_nodes: BTreeSet::new(),
            capture_metrics: BTreeSet::new(),
            every_n_steps: 1,
            include_step_zero: true,
            include_final_state: true,
        }
    }
}

impl CaptureConfig {
    /// Disables step-zero/final captures while preserving explicit selection sets.
    pub fn disabled() -> Self {
        Self { include_step_zero: false, include_final_state: false, ..Self::default() }
    }
}

/// Deterministic controls for one simulation run.
///
/// # Example
/// ```rust
/// use anapao::types::{CaptureConfig, RunConfig};
///
/// let run = RunConfig::for_seed(42).with_max_steps(250).with_capture(CaptureConfig {
///     every_n_steps: 5,
///     include_step_zero: false,
///     ..CaptureConfig::default()
/// });
///
/// assert_eq!(run.seed, 42);
/// assert_eq!(run.max_steps, 250);
/// assert_eq!(run.capture.every_n_steps, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunConfig {
    pub seed: u64,
    pub max_steps: u64,
    pub capture: CaptureConfig,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self { seed: 0, max_steps: 100, capture: CaptureConfig::default() }
    }
}

impl RunConfig {
    /// Creates a run config from a seed with default limits/capture policy.
    pub fn for_seed(seed: u64) -> Self {
        Self { seed, ..Self::default() }
    }

    /// Sets the run step limit.
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Replaces capture settings for the run.
    pub fn with_capture(mut self, capture: CaptureConfig) -> Self {
        self.capture = capture;
        self
    }
}

/// Seed-agnostic run template used by batch execution.
///
/// `BatchConfig.base_seed` and run index derive the actual seed for each run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchRunTemplate {
    pub max_steps: u64,
    pub capture: CaptureConfig,
}

impl Default for BatchRunTemplate {
    fn default() -> Self {
        Self { max_steps: 100, capture: CaptureConfig::default() }
    }
}

impl BatchRunTemplate {
    /// Sets the per-run step limit used by derived run configs.
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Replaces capture settings used by derived run configs.
    pub fn with_capture(mut self, capture: CaptureConfig) -> Self {
        self.capture = capture;
        self
    }

    /// Builds a concrete run config for one derived seed.
    pub fn to_run_config(&self, seed: u64) -> RunConfig {
        RunConfig { seed, max_steps: self.max_steps, capture: self.capture.clone() }
    }
}

/// Deterministic Monte Carlo controls for many runs.
///
/// The `base_seed` is used with run index derivation to produce stable per-run seeds.
/// `run_template` stores seed-agnostic controls shared by all runs.
///
/// # Example
/// ```rust
/// use anapao::types::{BatchConfig, BatchRunTemplate, CaptureConfig, ExecutionMode};
///
/// let batch = BatchConfig::for_runs(128)
///     .with_execution_mode(ExecutionMode::SingleThread)
///     .with_base_seed(999)
///     .with_run_template(BatchRunTemplate::default())
///     .with_max_steps(50)
///     .with_capture(CaptureConfig::disabled());
///
/// assert_eq!(batch.runs, 128);
/// assert_eq!(batch.execution_mode, ExecutionMode::SingleThread);
/// assert_eq!(batch.base_seed, 999);
/// assert_eq!(batch.run_template.max_steps, 50);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchConfig {
    pub runs: u64,
    pub base_seed: u64,
    pub execution_mode: ExecutionMode,
    pub run_template: BatchRunTemplate,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            runs: 1,
            base_seed: 0,
            execution_mode: ExecutionMode::default(),
            run_template: BatchRunTemplate::default(),
        }
    }
}

impl BatchConfig {
    /// Creates a batch config from a requested run count and default options.
    pub fn for_runs(runs: u64) -> Self {
        Self { runs, ..Self::default() }
    }

    /// Sets execution mode for the batch.
    pub fn with_execution_mode(mut self, execution_mode: ExecutionMode) -> Self {
        self.execution_mode = execution_mode;
        self
    }

    /// Sets the deterministic base seed used to derive per-run seeds.
    pub fn with_base_seed(mut self, base_seed: u64) -> Self {
        self.base_seed = base_seed;
        self
    }

    /// Replaces the default run template used for each batch run.
    pub fn with_run_template(mut self, run_template: BatchRunTemplate) -> Self {
        self.run_template = run_template;
        self
    }

    /// Sets max steps on the batch run template.
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.run_template.max_steps = max_steps;
        self
    }

    /// Replaces capture settings on the batch run template.
    pub fn with_capture(mut self, capture: CaptureConfig) -> Self {
        self.run_template.capture = capture;
        self
    }
}
