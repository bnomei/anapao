use serde::{Deserialize, Serialize};

use crate::error::AssertionError;
use crate::types::{BatchReport, MetricKey, RunReport, SeriesPoint};

const MAX_FAILURE_EVIDENCE_REFS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Selects which value window an expectation inspects.
pub enum MetricSelector {
    #[default]
    Final,
    Step(u64),
}

/// Typed expectation language for run and batch verification.
///
/// Create expectation sets up front, then evaluate them with
/// [`evaluate_run_expectations`] or [`evaluate_batch_expectations`], or use
/// [`crate::Simulator::run_with_assertions`]/[`crate::Simulator::run_batch_with_assertions`]
/// for integrated execution plus assertion checkpoints in the event stream.
///
/// # Example
/// ```rust
/// use anapao::assertions::{Expectation, MetricSelector};
/// use anapao::types::MetricKey;
///
/// let metric = MetricKey::fixture("sink");
/// let expectations = vec![
///     Expectation::Equals {
///         metric: metric.clone(),
///         selector: MetricSelector::Final,
///         expected: 3.0,
///     },
///     Expectation::Approx {
///         metric: metric.clone(),
///         selector: MetricSelector::Final,
///         expected: 3.0,
///         abs_tol: 0.0001,
///         rel_tol: 0.0,
///     },
///     Expectation::Between {
///         metric: metric.clone(),
///         selector: MetricSelector::Final,
///         min: 0.0,
///         max: 10.0,
///     },
///     Expectation::MonotonicNonDecreasing {
///         metric: metric.clone(),
///     },
///     Expectation::ProbabilityBand {
///         metric,
///         min: 2.0,
///         max: 4.0,
///         probability_min: 0.95,
///         probability_max: 1.0,
///     },
/// ];
///
/// assert_eq!(expectations.len(), 5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expectation {
    Equals {
        metric: MetricKey,
        selector: MetricSelector,
        expected: f64,
    },
    Approx {
        metric: MetricKey,
        selector: MetricSelector,
        expected: f64,
        abs_tol: f64,
        rel_tol: f64,
    },
    Between {
        metric: MetricKey,
        selector: MetricSelector,
        min: f64,
        max: f64,
    },
    MonotonicNonDecreasing {
        metric: MetricKey,
    },
    ProbabilityBand {
        metric: MetricKey,
        min: f64,
        max: f64,
        probability_min: f64,
        probability_max: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub metric: MetricKey,
    pub context: String,
}

impl EvidenceRef {
    fn new(metric: MetricKey, context: impl Into<String>) -> Self {
        Self { metric, context: context.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionResult {
    pub expectation: Expectation,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Aggregated assertion outcome summary.
///
/// This report is deterministic for deterministic inputs and includes
/// per-expectation evidence references to aid failure triage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AssertionReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<AssertionResult>,
}

impl AssertionReport {
    fn from_results(results: Vec<AssertionResult>) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|result| result.passed).count();
        let failed = total.saturating_sub(passed);
        Self { total, passed, failed, results }
    }

    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

pub fn evaluate_run_expectations(
    run_report: &RunReport,
    expectations: &[Expectation],
) -> Result<AssertionReport, AssertionError> {
    for expectation in expectations {
        validate_expectation(expectation)?;
    }

    let results = expectations
        .iter()
        .map(|expectation| evaluate_run_expectation(run_report, expectation))
        .collect::<Vec<_>>();
    Ok(AssertionReport::from_results(results))
}

pub fn evaluate_batch_expectations(
    batch_report: &BatchReport,
    expectations: &[Expectation],
) -> Result<AssertionReport, AssertionError> {
    for expectation in expectations {
        validate_expectation(expectation)?;
    }

    let results = expectations
        .iter()
        .map(|expectation| evaluate_batch_expectation(batch_report, expectation))
        .collect::<Vec<_>>();
    Ok(AssertionReport::from_results(results))
}

#[derive(Debug)]
struct ScalarObservation {
    value: Option<f64>,
    actual: String,
    evidence_refs: Vec<EvidenceRef>,
}

fn evaluate_run_expectation(run_report: &RunReport, expectation: &Expectation) -> AssertionResult {
    match expectation {
        Expectation::Equals { metric, selector, expected } => {
            let observed = observe_run_scalar(run_report, metric, selector);
            scalar_result(
                expectation,
                observed,
                format!("== {}", format_f64(*expected)),
                |actual| actual == *expected,
            )
        }
        Expectation::Approx { metric, selector, expected, abs_tol, rel_tol } => {
            let observed = observe_run_scalar(run_report, metric, selector);
            scalar_result(
                expectation,
                observed,
                format!(
                    "approx {} (abs_tol={}, rel_tol={})",
                    format_f64(*expected),
                    format_f64(*abs_tol),
                    format_f64(*rel_tol)
                ),
                |actual| approx_with_tolerance(actual, *expected, *abs_tol, *rel_tol),
            )
        }
        Expectation::Between { metric, selector, min, max } => {
            let observed = observe_run_scalar(run_report, metric, selector);
            scalar_result(
                expectation,
                observed,
                format!("[{}, {}]", format_f64(*min), format_f64(*max)),
                |actual| (*min..=*max).contains(&actual),
            )
        }
        Expectation::MonotonicNonDecreasing { metric } => {
            evaluate_run_monotonic_non_decreasing(run_report, expectation, metric)
        }
        Expectation::ProbabilityBand { metric, min, max, probability_min, probability_max } => {
            evaluate_run_probability_band(
                run_report,
                expectation,
                metric,
                *min,
                *max,
                *probability_min,
                *probability_max,
            )
        }
    }
}

fn evaluate_batch_expectation(
    batch_report: &BatchReport,
    expectation: &Expectation,
) -> AssertionResult {
    match expectation {
        Expectation::Equals { metric, selector, expected } => {
            let observed = observe_batch_scalar(batch_report, metric, selector);
            scalar_result(
                expectation,
                observed,
                format!("== {}", format_f64(*expected)),
                |actual| actual == *expected,
            )
        }
        Expectation::Approx { metric, selector, expected, abs_tol, rel_tol } => {
            let observed = observe_batch_scalar(batch_report, metric, selector);
            scalar_result(
                expectation,
                observed,
                format!(
                    "approx {} (abs_tol={}, rel_tol={})",
                    format_f64(*expected),
                    format_f64(*abs_tol),
                    format_f64(*rel_tol)
                ),
                |actual| approx_with_tolerance(actual, *expected, *abs_tol, *rel_tol),
            )
        }
        Expectation::Between { metric, selector, min, max } => {
            let observed = observe_batch_scalar(batch_report, metric, selector);
            scalar_result(
                expectation,
                observed,
                format!("[{}, {}]", format_f64(*min), format_f64(*max)),
                |actual| (*min..=*max).contains(&actual),
            )
        }
        Expectation::MonotonicNonDecreasing { metric } => {
            evaluate_batch_monotonic_non_decreasing(batch_report, expectation, metric)
        }
        Expectation::ProbabilityBand { metric, min, max, probability_min, probability_max } => {
            evaluate_batch_probability_band(
                batch_report,
                expectation,
                metric,
                *min,
                *max,
                *probability_min,
                *probability_max,
            )
        }
    }
}

fn scalar_result(
    expectation: &Expectation,
    observed: ScalarObservation,
    expected: String,
    predicate: impl FnOnce(f64) -> bool,
) -> AssertionResult {
    let (passed, actual) = match observed.value {
        Some(value) => (predicate(value), format_f64(value)),
        None => (false, observed.actual),
    };

    AssertionResult {
        expectation: expectation.clone(),
        passed,
        expected,
        actual,
        evidence_refs: observed.evidence_refs,
    }
}

fn observe_run_scalar(
    run_report: &RunReport,
    metric: &MetricKey,
    selector: &MetricSelector,
) -> ScalarObservation {
    match selector {
        MetricSelector::Final => {
            let context = "run.final_metrics";
            let value = run_report.final_metrics.get(metric).copied();
            scalar_observation(metric, context, value)
        }
        MetricSelector::Step(step) => {
            let context = format!("run.series.step={step}");
            let value = run_report
                .series
                .get(metric)
                .and_then(|table| table.points.iter().find(|point| point.step == *step))
                .map(|point| point.value);
            scalar_observation(metric, context, value)
        }
    }
}

fn observe_batch_scalar(
    batch_report: &BatchReport,
    metric: &MetricKey,
    selector: &MetricSelector,
) -> ScalarObservation {
    match selector {
        MetricSelector::Final => {
            let context = "batch.aggregate_series.final";
            let value = batch_report
                .aggregate_series
                .get(metric)
                .and_then(|table| table.points.last())
                .map(|point| point.value);
            scalar_observation(metric, context, value)
        }
        MetricSelector::Step(step) => {
            let context = format!("batch.aggregate_series.step={step}");
            let value = batch_report
                .aggregate_series
                .get(metric)
                .and_then(|table| table.points.iter().find(|point| point.step == *step))
                .map(|point| point.value);
            scalar_observation(metric, context, value)
        }
    }
}

fn scalar_observation(
    metric: &MetricKey,
    context: impl Into<String>,
    value: Option<f64>,
) -> ScalarObservation {
    let context = context.into();
    let actual =
        value.map_or_else(|| format!("missing metric `{metric}` at `{context}`"), format_f64);

    ScalarObservation {
        value,
        actual,
        evidence_refs: vec![EvidenceRef::new(metric.clone(), context)],
    }
}

fn evaluate_run_monotonic_non_decreasing(
    run_report: &RunReport,
    expectation: &Expectation,
    metric: &MetricKey,
) -> AssertionResult {
    evaluate_monotonic_non_decreasing(
        expectation,
        metric,
        "run.series",
        run_report.series.get(metric).map(|table| table.points.as_slice()),
    )
}

fn evaluate_batch_monotonic_non_decreasing(
    batch_report: &BatchReport,
    expectation: &Expectation,
    metric: &MetricKey,
) -> AssertionResult {
    evaluate_monotonic_non_decreasing(
        expectation,
        metric,
        "batch.aggregate_series",
        batch_report.aggregate_series.get(metric).map(|table| table.points.as_slice()),
    )
}

fn evaluate_monotonic_non_decreasing(
    expectation: &Expectation,
    metric: &MetricKey,
    context_prefix: &str,
    points: Option<&[SeriesPoint]>,
) -> AssertionResult {
    let mut evidence_refs = vec![EvidenceRef::new(metric.clone(), context_prefix.to_string())];
    let expected = "non-decreasing series".to_string();

    let Some(points) = points else {
        return AssertionResult {
            expectation: expectation.clone(),
            passed: false,
            expected,
            actual: format!("missing series for metric `{metric}`"),
            evidence_refs,
        };
    };

    if let Some((left, right)) = points.windows(2).find_map(|window| {
        if window[0].value > window[1].value {
            Some((&window[0], &window[1]))
        } else {
            None
        }
    }) {
        evidence_refs
            .push(EvidenceRef::new(metric.clone(), format!("{context_prefix}.step={}", left.step)));
        evidence_refs.push(EvidenceRef::new(
            metric.clone(),
            format!("{context_prefix}.step={}", right.step),
        ));
        return AssertionResult {
            expectation: expectation.clone(),
            passed: false,
            expected,
            actual: format!(
                "decreased from {} at step {} to {} at step {}",
                format_f64(left.value),
                left.step,
                format_f64(right.value),
                right.step
            ),
            evidence_refs,
        };
    }

    AssertionResult {
        expectation: expectation.clone(),
        passed: true,
        expected,
        actual: format!("series is non-decreasing across {} points", points.len()),
        evidence_refs,
    }
}

fn evaluate_run_probability_band(
    run_report: &RunReport,
    expectation: &Expectation,
    metric: &MetricKey,
    min: f64,
    max: f64,
    probability_min: f64,
    probability_max: f64,
) -> AssertionResult {
    let context = "run.series";
    let points = run_report.series.get(metric).map(|table| table.points.as_slice());
    evaluate_probability_band_over_points(
        expectation,
        metric,
        context,
        points,
        min,
        max,
        probability_min,
        probability_max,
    )
}

fn evaluate_batch_probability_band(
    batch_report: &BatchReport,
    expectation: &Expectation,
    metric: &MetricKey,
    min: f64,
    max: f64,
    probability_min: f64,
    probability_max: f64,
) -> AssertionResult {
    let mut evidence_refs =
        vec![EvidenceRef::new(metric.clone(), "batch.runs.final_metrics".to_string())];
    let expected = format!(
        "p in [{}, {}] for values in [{}, {}]",
        format_f64(probability_min),
        format_f64(probability_max),
        format_f64(min),
        format_f64(max)
    );

    if batch_report.runs.is_empty() {
        return AssertionResult {
            expectation: expectation.clone(),
            passed: false,
            expected,
            actual: "no runs available".to_string(),
            evidence_refs,
        };
    }

    let mut in_band = 0usize;
    let mut total = 0usize;
    for run in &batch_report.runs {
        total += 1;
        match run.final_metrics.get(metric).copied() {
            Some(value) if (min..=max).contains(&value) => in_band += 1,
            Some(_) | None => {
                if evidence_refs.len() < MAX_FAILURE_EVIDENCE_REFS + 1 {
                    evidence_refs.push(EvidenceRef::new(
                        metric.clone(),
                        format!("batch.runs[{}].final_metrics", run.run_index),
                    ));
                }
            }
        }
    }

    let probability = in_band as f64 / total as f64;
    let passed = (probability_min..=probability_max).contains(&probability);
    let actual = format!(
        "p={} ({}/{}) for values in [{}, {}]",
        format_f64(probability),
        in_band,
        total,
        format_f64(min),
        format_f64(max)
    );

    AssertionResult { expectation: expectation.clone(), passed, expected, actual, evidence_refs }
}

fn evaluate_probability_band_over_points(
    expectation: &Expectation,
    metric: &MetricKey,
    context_prefix: &str,
    points: Option<&[SeriesPoint]>,
    min: f64,
    max: f64,
    probability_min: f64,
    probability_max: f64,
) -> AssertionResult {
    let mut evidence_refs = vec![EvidenceRef::new(metric.clone(), context_prefix.to_string())];
    let expected = format!(
        "p in [{}, {}] for values in [{}, {}]",
        format_f64(probability_min),
        format_f64(probability_max),
        format_f64(min),
        format_f64(max)
    );

    let Some(points) = points else {
        return AssertionResult {
            expectation: expectation.clone(),
            passed: false,
            expected,
            actual: format!("missing series for metric `{metric}`"),
            evidence_refs,
        };
    };

    if points.is_empty() {
        return AssertionResult {
            expectation: expectation.clone(),
            passed: false,
            expected,
            actual: "series has no points".to_string(),
            evidence_refs,
        };
    }

    let mut in_band = 0usize;
    for point in points {
        if (min..=max).contains(&point.value) {
            in_band += 1;
        } else if evidence_refs.len() < MAX_FAILURE_EVIDENCE_REFS + 1 {
            evidence_refs.push(EvidenceRef::new(
                metric.clone(),
                format!("{context_prefix}.step={}", point.step),
            ));
        }
    }

    let probability = in_band as f64 / points.len() as f64;
    let passed = (probability_min..=probability_max).contains(&probability);
    let actual = format!(
        "p={} ({}/{}) for values in [{}, {}]",
        format_f64(probability),
        in_band,
        points.len(),
        format_f64(min),
        format_f64(max)
    );

    AssertionResult { expectation: expectation.clone(), passed, expected, actual, evidence_refs }
}

fn validate_expectation(expectation: &Expectation) -> Result<(), AssertionError> {
    match expectation {
        Expectation::Equals { expected, .. } => validate_finite("equals.expected", *expected),
        Expectation::Approx { expected, abs_tol, rel_tol, .. } => {
            validate_finite("approx.expected", *expected)?;
            validate_non_negative_finite("approx.abs_tol", *abs_tol)?;
            validate_non_negative_finite("approx.rel_tol", *rel_tol)
        }
        Expectation::Between { min, max, .. } => {
            validate_finite("between.min", *min)?;
            validate_finite("between.max", *max)?;
            if min > max {
                return Err(invalid_expectation(
                    "between.range",
                    "min <= max",
                    format!("min={} > max={}", format_f64(*min), format_f64(*max)),
                ));
            }
            Ok(())
        }
        Expectation::MonotonicNonDecreasing { .. } => Ok(()),
        Expectation::ProbabilityBand { min, max, probability_min, probability_max, .. } => {
            validate_finite("probability_band.min", *min)?;
            validate_finite("probability_band.max", *max)?;
            if min > max {
                return Err(invalid_expectation(
                    "probability_band.value_range",
                    "min <= max",
                    format!("min={} > max={}", format_f64(*min), format_f64(*max)),
                ));
            }

            validate_probability("probability_band.probability_min", *probability_min)?;
            validate_probability("probability_band.probability_max", *probability_max)?;
            if probability_min > probability_max {
                return Err(invalid_expectation(
                    "probability_band.probability_range",
                    "probability_min <= probability_max",
                    format!(
                        "probability_min={} > probability_max={}",
                        format_f64(*probability_min),
                        format_f64(*probability_max)
                    ),
                ));
            }
            Ok(())
        }
    }
}

fn validate_finite(subject: &str, value: f64) -> Result<(), AssertionError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid_expectation(subject, "finite numeric value", format_f64(value)))
    }
}

fn validate_non_negative_finite(subject: &str, value: f64) -> Result<(), AssertionError> {
    validate_finite(subject, value)?;
    if value < 0.0 {
        return Err(invalid_expectation(subject, "non-negative numeric value", format_f64(value)));
    }
    Ok(())
}

fn validate_probability(subject: &str, value: f64) -> Result<(), AssertionError> {
    validate_finite(subject, value)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(invalid_expectation(subject, "value in [0, 1]", format_f64(value)));
    }
    Ok(())
}

fn invalid_expectation(
    subject: impl Into<String>,
    expected: impl Into<String>,
    actual: impl Into<String>,
) -> AssertionError {
    AssertionError::ExpectationMismatch {
        subject: subject.into(),
        expected: expected.into(),
        actual: actual.into(),
    }
}

fn approx_with_tolerance(actual: f64, expected: f64, abs_tol: f64, rel_tol: f64) -> bool {
    let tolerance = abs_tol.max(expected.abs() * rel_tol);
    (actual - expected).abs() <= tolerance
}

fn format_f64(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() && value.is_sign_positive() {
        "inf".to_string()
    } else if value.is_infinite() {
        "-inf".to_string()
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::error::AssertionError;
    use crate::types::{
        BatchReport, BatchRunSummary, ExecutionMode, MetricKey, RunReport, ScenarioId, SeriesPoint,
        SeriesTable,
    };

    use super::{
        evaluate_batch_expectations, evaluate_run_expectations, Expectation, MetricSelector,
    };

    #[test]
    fn run_expectations_capture_pass_and_failure_evidence() {
        let metric = MetricKey::fixture("throughput");
        let run_report = fixture_run_report();
        let expectations = vec![
            Expectation::Equals {
                metric: metric.clone(),
                selector: MetricSelector::Final,
                expected: 11.0,
            },
            Expectation::Equals {
                metric: metric.clone(),
                selector: MetricSelector::Step(99),
                expected: 0.0,
            },
        ];

        let report = evaluate_run_expectations(&run_report, &expectations).expect("evaluation");

        assert_eq!(report.total, 2);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
        assert!(!report.is_success());

        let failed = report.results.iter().find(|result| !result.passed).expect("failed assertion");
        assert!(failed.evidence_refs.iter().any(|reference| reference.context.contains("step=99")));
    }

    #[test]
    fn run_approx_respects_tolerance() {
        let metric = MetricKey::fixture("throughput");
        let run_report = fixture_run_report();
        let expectations = vec![
            Expectation::Approx {
                metric: metric.clone(),
                selector: MetricSelector::Final,
                expected: 12.0,
                abs_tol: 1.0,
                rel_tol: 0.0,
            },
            Expectation::Approx {
                metric,
                selector: MetricSelector::Final,
                expected: 12.0,
                abs_tol: 0.5,
                rel_tol: 0.0,
            },
        ];

        let report = evaluate_run_expectations(&run_report, &expectations).expect("evaluation");

        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn run_monotonic_and_probability_band_capture_step_context() {
        let metric = MetricKey::fixture("load");
        let mut run_report = fixture_run_report();
        run_report.series.insert(
            metric.clone(),
            SeriesTable {
                metric: metric.clone(),
                points: vec![
                    SeriesPoint::new(0, 0.2),
                    SeriesPoint::new(1, 0.4),
                    SeriesPoint::new(2, 0.1),
                ],
            },
        );

        let expectations = vec![
            Expectation::MonotonicNonDecreasing { metric: metric.clone() },
            Expectation::ProbabilityBand {
                metric,
                min: 0.15,
                max: 1.0,
                probability_min: 0.9,
                probability_max: 1.0,
            },
        ];

        let report = evaluate_run_expectations(&run_report, &expectations).expect("evaluation");
        assert_eq!(report.failed, 2);

        let monotonic = report
            .results
            .iter()
            .find(|result| matches!(result.expectation, Expectation::MonotonicNonDecreasing { .. }))
            .expect("monotonic result");
        assert!(monotonic
            .evidence_refs
            .iter()
            .any(|reference| reference.context.contains("step=1")));

        let band = report
            .results
            .iter()
            .find(|result| matches!(result.expectation, Expectation::ProbabilityBand { .. }))
            .expect("band result");
        assert!(band.evidence_refs.iter().any(|reference| reference.context.contains("step=2")));
    }

    #[test]
    fn batch_probability_band_uses_per_run_final_metrics() {
        let throughput = MetricKey::fixture("throughput");
        let pass_rate = MetricKey::fixture("pass_rate");
        let batch_report = fixture_batch_report(&throughput, &pass_rate);

        let expectations = vec![
            Expectation::Between {
                metric: throughput.clone(),
                selector: MetricSelector::Final,
                min: 9.0,
                max: 11.0,
            },
            Expectation::ProbabilityBand {
                metric: pass_rate.clone(),
                min: 0.7,
                max: 1.0,
                probability_min: 0.75,
                probability_max: 1.0,
            },
            Expectation::ProbabilityBand {
                metric: pass_rate,
                min: 0.7,
                max: 1.0,
                probability_min: 0.8,
                probability_max: 1.0,
            },
        ];

        let report = evaluate_batch_expectations(&batch_report, &expectations).expect("evaluation");

        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 1);
        let failed = report.results.iter().find(|result| !result.passed).expect("failed result");
        assert!(failed.evidence_refs.iter().any(|reference| reference.context.contains("runs[3]")));
    }

    #[test]
    fn run_probability_band_reports_missing_and_empty_series() {
        let metric = MetricKey::fixture("missing");
        let expectation = Expectation::ProbabilityBand {
            metric: metric.clone(),
            min: 0.0,
            max: 1.0,
            probability_min: 0.5,
            probability_max: 1.0,
        };

        let missing_report =
            evaluate_run_expectations(&fixture_run_report(), &[expectation.clone()])
                .expect("evaluation should complete");
        assert_eq!(missing_report.failed, 1);
        assert_eq!(
            missing_report.results[0].actual,
            "missing series for metric `missing`".to_string()
        );

        let mut empty_series_run = fixture_run_report();
        empty_series_run.series.insert(
            metric,
            SeriesTable { metric: MetricKey::fixture("missing"), points: Vec::new() },
        );
        let empty_report =
            evaluate_run_expectations(&empty_series_run, &[expectation]).expect("evaluation");
        assert_eq!(empty_report.failed, 1);
        assert_eq!(empty_report.results[0].actual, "series has no points");
    }

    #[test]
    fn batch_monotonic_expectations_cover_success_and_missing_series() {
        let throughput = MetricKey::fixture("throughput");
        let pass_rate = MetricKey::fixture("pass_rate");
        let missing = MetricKey::fixture("missing");
        let batch_report = fixture_batch_report(&throughput, &pass_rate);
        let expectations = vec![
            Expectation::MonotonicNonDecreasing { metric: throughput },
            Expectation::MonotonicNonDecreasing { metric: missing },
        ];

        let report = evaluate_batch_expectations(&batch_report, &expectations).expect("evaluation");
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
        let failed = report.results.iter().find(|result| !result.passed).expect("failed result");
        assert!(failed.actual.contains("missing series for metric"));
        assert!(failed
            .evidence_refs
            .iter()
            .any(|reference| reference.context == "batch.aggregate_series"));
    }

    #[test]
    fn invalid_expectation_configuration_covers_additional_subjects() {
        let metric = MetricKey::fixture("throughput");
        let run_report = fixture_run_report();

        let cases = vec![
            (
                Expectation::Between {
                    metric: metric.clone(),
                    selector: MetricSelector::Final,
                    min: 2.0,
                    max: 1.0,
                },
                "between.range",
            ),
            (
                Expectation::ProbabilityBand {
                    metric: metric.clone(),
                    min: 0.0,
                    max: 1.0,
                    probability_min: 0.8,
                    probability_max: 0.7,
                },
                "probability_band.probability_range",
            ),
            (
                Expectation::ProbabilityBand {
                    metric: metric.clone(),
                    min: 2.0,
                    max: 1.0,
                    probability_min: 0.0,
                    probability_max: 1.0,
                },
                "probability_band.value_range",
            ),
            (
                Expectation::Approx {
                    metric,
                    selector: MetricSelector::Final,
                    expected: 10.0,
                    abs_tol: 0.1,
                    rel_tol: -0.1,
                },
                "approx.rel_tol",
            ),
        ];

        for (expectation, subject) in cases {
            let error =
                evaluate_run_expectations(&run_report, &[expectation]).expect_err("must fail");
            assert!(matches!(
                error,
                AssertionError::ExpectationMismatch { subject: actual, .. } if actual == subject
            ));
        }
    }

    #[test]
    fn invalid_expectation_configuration_returns_assertion_error() {
        let metric = MetricKey::fixture("throughput");
        let run_report = fixture_run_report();
        let expectations = vec![Expectation::Approx {
            metric,
            selector: MetricSelector::Final,
            expected: 10.0,
            abs_tol: -0.1,
            rel_tol: 0.0,
        }];

        let error = evaluate_run_expectations(&run_report, &expectations).expect_err("must fail");
        assert!(matches!(
            error,
            AssertionError::ExpectationMismatch { subject, .. } if subject == "approx.abs_tol"
        ));
    }

    fn fixture_run_report() -> RunReport {
        let throughput = MetricKey::fixture("throughput");
        let latency = MetricKey::fixture("latency");
        let mut run_report = RunReport::new(ScenarioId::fixture("scenario"), 7);
        run_report.steps_executed = 2;
        run_report.completed = true;
        run_report.final_metrics.insert(throughput.clone(), 11.0);
        run_report.final_metrics.insert(latency.clone(), 4.2);
        run_report.series.insert(
            throughput.clone(),
            SeriesTable {
                metric: throughput,
                points: vec![
                    SeriesPoint::new(0, 8.0),
                    SeriesPoint::new(1, 10.0),
                    SeriesPoint::new(2, 11.0),
                ],
            },
        );
        run_report.series.insert(
            latency.clone(),
            SeriesTable {
                metric: latency,
                points: vec![
                    SeriesPoint::new(0, 6.0),
                    SeriesPoint::new(1, 5.0),
                    SeriesPoint::new(2, 4.2),
                ],
            },
        );
        run_report
    }

    fn fixture_batch_report(throughput: &MetricKey, pass_rate: &MetricKey) -> BatchReport {
        let mut batch_report =
            BatchReport::new(ScenarioId::fixture("scenario"), 4, ExecutionMode::SingleThread);
        batch_report.completed_runs = 4;
        batch_report.aggregate_series.insert(
            throughput.clone(),
            SeriesTable {
                metric: throughput.clone(),
                points: vec![
                    SeriesPoint::new(0, 0.0),
                    SeriesPoint::new(1, 5.0),
                    SeriesPoint::new(2, 10.0),
                ],
            },
        );

        let run_values = [0.8, 0.75, 0.9, 0.6];
        batch_report.runs = run_values
            .into_iter()
            .enumerate()
            .map(|(run_index, value)| BatchRunSummary {
                run_index: run_index as u64,
                seed: run_index as u64 + 100,
                completed: true,
                steps_executed: 2,
                final_metrics: BTreeMap::from([(pass_rate.clone(), value)]),
                manifest: None,
            })
            .collect::<Vec<_>>();
        batch_report
    }
}
