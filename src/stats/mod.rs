//! Deterministic summary statistics utilities for simulation outputs.

use std::collections::BTreeMap;

use crate::types::{MetricKey, PredictionMetricIndicators};

#[derive(Debug, Clone, PartialEq)]
/// Streaming moments computed with Welford's online algorithm.
pub struct StreamingMomentsSummary {
    pub n: usize,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq)]
/// Summary statistics computed from a sample vector.
pub struct StatsSummary {
    pub n: usize,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

#[derive(Debug, Clone, Copy)]
struct WelfordAccumulator {
    n: usize,
    mean: f64,
    m2: f64,
    min: f64,
    max: f64,
}

impl WelfordAccumulator {
    fn new() -> Self {
        Self { n: 0, mean: 0.0, m2: 0.0, min: f64::INFINITY, max: f64::NEG_INFINITY }
    }

    fn push(&mut self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }

        if self.n == 0 {
            self.n = 1;
            self.mean = value;
            self.m2 = 0.0;
            self.min = value;
            self.max = value;
            return true;
        }

        self.n += 1;
        let n = self.n as f64;
        let delta = value - self.mean;
        self.mean += delta / n;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        true
    }

    fn finalize(self) -> Option<StreamingMomentsSummary> {
        if self.n == 0 {
            return None;
        }

        let variance = self.m2 / self.n as f64;
        Some(StreamingMomentsSummary {
            n: self.n,
            mean: self.mean,
            variance,
            std_dev: variance.sqrt(),
            min: self.min,
            max: self.max,
        })
    }
}

/// Computes streaming moments in one pass without retaining all values.
pub fn summarize_streaming<I>(values: I) -> Option<StreamingMomentsSummary>
where
    I: IntoIterator<Item = f64>,
{
    let mut accumulator = WelfordAccumulator::new();
    for value in values {
        if !accumulator.push(value) {
            return None;
        }
    }
    accumulator.finalize()
}

/// Computes descriptive statistics for finite values.
pub fn summarize(values: &[f64]) -> Option<StatsSummary> {
    let summary = summarize_streaming(values.iter().copied())?;

    let mut sorted_values = values.to_vec();
    sorted_values.sort_by(f64::total_cmp);

    Some(StatsSummary {
        n: summary.n,
        mean: summary.mean,
        variance: summary.variance,
        std_dev: summary.std_dev,
        min: summary.min,
        max: summary.max,
        p50: percentile_sorted(&sorted_values, 50.0)?,
        p90: percentile_sorted(&sorted_values, 90.0)?,
        p95: percentile_sorted(&sorted_values, 95.0)?,
        p99: percentile_sorted(&sorted_values, 99.0)?,
    })
}

/// Returns a percentile from a sorted finite slice using linear interpolation.
pub fn percentile_sorted(sorted_values: &[f64], percentile: f64) -> Option<f64> {
    if sorted_values.is_empty()
        || !percentile.is_finite()
        || !(0.0..=100.0).contains(&percentile)
        || sorted_values.iter().any(|value| !value.is_finite())
    {
        return None;
    }

    if sorted_values.windows(2).any(|window| window[0] > window[1]) {
        return None;
    }

    if sorted_values.len() == 1 {
        return Some(sorted_values[0]);
    }

    let rank = (percentile / 100.0) * (sorted_values.len() - 1) as f64;
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;

    if lower_index == upper_index {
        return Some(sorted_values[lower_index]);
    }

    let fraction = rank - lower_index as f64;
    let lower = sorted_values[lower_index];
    let upper = sorted_values[upper_index];
    Some(lower + (upper - lower) * fraction)
}

/// Computes summaries for each metric, dropping invalid metric samples.
pub fn summarize_by_metric(
    values_by_metric: BTreeMap<MetricKey, Vec<f64>>,
) -> BTreeMap<MetricKey, StatsSummary> {
    values_by_metric
        .into_iter()
        .filter_map(|(metric, values)| summarize(&values).map(|summary| (metric, summary)))
        .collect()
}

/// Computes a 95% confidence interval around the sample mean.
pub fn mean_confidence_interval_95(values: &[f64]) -> Option<(f64, f64)> {
    const Z_SCORE_95: f64 = 1.959_963_984_540_054;

    let summary = summarize_streaming(values.iter().copied())?;
    if summary.n == 1 {
        return Some((summary.mean, summary.mean));
    }

    let n = summary.n as f64;
    let sample_variance = summary.variance * n / (n - 1.0);
    let standard_error = sample_variance.sqrt() / n.sqrt();
    let margin = Z_SCORE_95 * standard_error;
    Some((summary.mean - margin, summary.mean + margin))
}

/// Derives prediction-oriented indicators for one metric sample.
pub fn prediction_indicators(values: &[f64]) -> Option<PredictionMetricIndicators> {
    let summary = summarize(values)?;
    let (confidence_lower_95, confidence_upper_95) = mean_confidence_interval_95(values)?;
    let confidence_margin_95 = (confidence_upper_95 - confidence_lower_95) / 2.0;

    let convergence_delta = leading_trailing_delta(values)?;
    let baseline = summary.mean.abs() + 1.0;
    let relative_margin = confidence_margin_95 / baseline;
    let relative_dispersion = summary.std_dev / baseline;
    let reliability_score = 1.0 / (1.0 + relative_margin + relative_dispersion);

    Some(PredictionMetricIndicators {
        n: summary.n,
        mean: summary.mean,
        variance: summary.variance,
        std_dev: summary.std_dev,
        min: summary.min,
        max: summary.max,
        median: summary.p50,
        p90: summary.p90,
        p95: summary.p95,
        p99: summary.p99,
        confidence_lower_95,
        confidence_upper_95,
        confidence_margin_95,
        reliability_score,
        convergence_delta,
        convergence_ratio: convergence_delta / baseline,
    })
}

/// Computes prediction indicators for each metric, dropping invalid samples.
pub fn prediction_indicators_by_metric(
    values_by_metric: BTreeMap<MetricKey, Vec<f64>>,
) -> BTreeMap<MetricKey, PredictionMetricIndicators> {
    values_by_metric
        .into_iter()
        .filter_map(|(metric, values)| {
            prediction_indicators(&values).map(|summary| (metric, summary))
        })
        .collect()
}

fn leading_trailing_delta(values: &[f64]) -> Option<f64> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return None;
    }

    if values.len() == 1 {
        return Some(0.0);
    }

    let window = (values.len() / 2).max(1);
    let leading = &values[..window];
    let trailing = &values[values.len() - window..];

    let leading_mean = leading.iter().sum::<f64>() / leading.len() as f64;
    let trailing_mean = trailing.iter().sum::<f64>() / trailing.len() as f64;
    Some((trailing_mean - leading_mean).abs())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        mean_confidence_interval_95, percentile_sorted, prediction_indicators,
        prediction_indicators_by_metric, summarize, summarize_by_metric, summarize_streaming,
    };
    use crate::types::MetricKey;

    const EPSILON: f64 = 1e-12;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= EPSILON, "expected {expected}, got {actual}");
    }

    #[test]
    fn summarize_returns_none_for_empty_input() {
        assert!(summarize(&[]).is_none());
    }

    #[test]
    fn summarize_streaming_returns_none_for_empty_or_non_finite_input() {
        assert!(summarize_streaming(std::iter::empty::<f64>()).is_none());
        assert!(summarize_streaming([1.0, f64::NAN].into_iter()).is_none());
    }

    #[test]
    fn percentile_sorted_uses_linear_interpolation() {
        let values = [1.0, 2.0, 3.0, 4.0];
        assert_close(percentile_sorted(&values, 0.0).expect("p0"), 1.0);
        assert_close(percentile_sorted(&values, 25.0).expect("p25"), 1.75);
        assert_close(percentile_sorted(&values, 50.0).expect("p50"), 2.5);
        assert_close(percentile_sorted(&values, 100.0).expect("p100"), 4.0);
    }

    #[test]
    fn summarize_computes_expected_values() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let summary = summarize(&values).expect("summary");

        assert_eq!(summary.n, 10);
        assert_close(summary.mean, 5.5);
        assert_close(summary.variance, 8.25);
        assert_close(summary.std_dev, 2.872_281_323_269_014_3);
        assert_close(summary.min, 1.0);
        assert_close(summary.max, 10.0);
        assert_close(summary.p50, 5.5);
        assert_close(summary.p90, 9.1);
        assert_close(summary.p95, 9.55);
        assert_close(summary.p99, 9.91);
    }

    #[test]
    fn summarize_streaming_matches_legacy_moments_with_deterministic_tolerance() {
        let values = (0..10_000)
            .map(|index| {
                let i = index as f64;
                (i * 0.37).sin() * 40.0 + (index % 17) as f64 - 8.0
            })
            .collect::<Vec<_>>();
        let streaming = summarize_streaming(values.iter().copied()).expect("streaming summary");

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values
            .iter()
            .map(|value| {
                let diff = *value - mean;
                diff * diff
            })
            .sum::<f64>()
            / values.len() as f64;
        let std_dev = variance.sqrt();
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        assert_eq!(streaming.n, values.len());
        assert!((streaming.mean - mean).abs() <= 1e-10, "mean drifted");
        assert!((streaming.variance - variance).abs() <= 1e-10, "variance drifted");
        assert!((streaming.std_dev - std_dev).abs() <= 1e-10, "std_dev drifted");
        assert_close(streaming.min, min);
        assert_close(streaming.max, max);
    }

    #[test]
    fn summarize_streaming_accepts_large_iterators_without_collecting() {
        let n = 50_000_usize;
        let summary = summarize_streaming((1..=n).map(|value| value as f64))
            .expect("streaming summary from iterator");

        let n_f64 = n as f64;
        let expected_mean = (n_f64 + 1.0) / 2.0;
        let expected_variance = (n_f64 * n_f64 - 1.0) / 12.0;

        assert_eq!(summary.n, n);
        assert!((summary.mean - expected_mean).abs() <= 1e-9, "mean drifted");
        assert!((summary.variance - expected_variance).abs() <= 1e-3, "variance drifted");
        assert!((summary.std_dev - expected_variance.sqrt()).abs() <= 1e-9, "std_dev drifted");
        assert_close(summary.min, 1.0);
        assert_close(summary.max, n_f64);
    }

    #[test]
    fn summarize_and_streaming_moments_agree() {
        let values = [3.0, 8.0, 1.0, 4.0, 7.0];
        let summary = summarize(&values).expect("summary");
        let streaming = summarize_streaming(values.into_iter()).expect("streaming");

        assert_eq!(summary.n, streaming.n);
        assert_close(summary.mean, streaming.mean);
        assert_close(summary.variance, streaming.variance);
        assert_close(summary.std_dev, streaming.std_dev);
        assert_close(summary.min, streaming.min);
        assert_close(summary.max, streaming.max);
    }

    #[test]
    fn summarize_by_metric_keeps_deterministic_order() {
        let alpha = MetricKey::fixture("alpha");
        let beta = MetricKey::fixture("beta");
        let gamma = MetricKey::fixture("gamma");

        let values_by_metric = BTreeMap::from([
            (beta.clone(), vec![2.0, 4.0, 6.0, 8.0]),
            (gamma, vec![]),
            (alpha.clone(), vec![10.0, 20.0, 30.0]),
        ]);

        let summaries = summarize_by_metric(values_by_metric);
        let keys = summaries.keys().cloned().collect::<Vec<_>>();

        assert_eq!(keys, vec![alpha.clone(), beta.clone()]);
        assert_eq!(summaries.len(), 2);

        let alpha_summary = summaries.get(&alpha).expect("alpha summary");
        assert_eq!(alpha_summary.n, 3);
        assert_close(alpha_summary.mean, 20.0);
        assert_close(alpha_summary.p90, 28.0);

        let beta_summary = summaries.get(&beta).expect("beta summary");
        assert_eq!(beta_summary.n, 4);
        assert_close(beta_summary.mean, 5.0);
        assert_close(beta_summary.p95, 7.7);
    }

    #[test]
    fn mean_confidence_interval_95_matches_expected_values() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];
        let (lower, upper) = mean_confidence_interval_95(&values).expect("ci95");
        assert_close(lower, 1.614_096_175_650_322);
        assert_close(upper, 4.385_903_824_349_678);
    }

    #[test]
    fn mean_confidence_interval_95_single_value_has_zero_width() {
        let (lower, upper) = mean_confidence_interval_95(&[42.0]).expect("ci95");
        assert_close(lower, 42.0);
        assert_close(upper, 42.0);
    }

    #[test]
    fn prediction_indicators_include_confidence_reliability_and_convergence() {
        let indicators = prediction_indicators(&[10.0, 12.0, 14.0, 16.0]).expect("indicators");

        assert_eq!(indicators.n, 4);
        assert_close(indicators.mean, 13.0);
        assert_close(indicators.variance, 5.0);
        assert_close(indicators.std_dev, 2.236_067_977_499_79);
        assert_close(indicators.min, 10.0);
        assert_close(indicators.max, 16.0);
        assert_close(indicators.median, 13.0);
        assert_close(indicators.p90, 15.4);
        assert_close(indicators.p95, 15.7);
        assert_close(indicators.p99, 15.94);
        assert_close(indicators.confidence_lower_95, 10.469_697_376_236_681);
        assert_close(indicators.confidence_upper_95, 15.530_302_623_763_319);
        assert_close(indicators.confidence_margin_95, 2.530_302_623_763_319);
        assert!(indicators.reliability_score > 0.0 && indicators.reliability_score <= 1.0);
        assert_close(indicators.convergence_delta, 4.0);
        assert_close(indicators.convergence_ratio, 0.285_714_285_714_285_7);
    }

    #[test]
    fn prediction_indicators_by_metric_keeps_sorted_keys_and_filters_invalid_metrics() {
        let alpha = MetricKey::fixture("alpha");
        let beta = MetricKey::fixture("beta");

        let values_by_metric = BTreeMap::from([
            (beta.clone(), vec![1.0, 2.0, 3.0, 4.0]),
            (MetricKey::fixture("gamma"), vec![1.0, f64::NAN]),
            (alpha.clone(), vec![10.0, 10.0, 10.0]),
        ]);

        let summaries = prediction_indicators_by_metric(values_by_metric);
        let keys = summaries.keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys, vec![alpha.clone(), beta.clone()]);

        let alpha_summary = summaries.get(&alpha).expect("alpha summary");
        assert_close(alpha_summary.mean, 10.0);
        assert_close(alpha_summary.convergence_delta, 0.0);

        let beta_summary = summaries.get(&beta).expect("beta summary");
        assert_close(beta_summary.mean, 2.5);
        assert_close(beta_summary.convergence_delta, 2.0);
    }
}
