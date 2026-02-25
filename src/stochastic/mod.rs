//! Stochastic sampling helpers used by variable and gate runtime logic.

use rand::distr::Distribution;
use rand::RngExt;
use rand_distr::weighted::WeightedAliasIndex;
use rand_distr::Bernoulli;

use crate::error::SetupError;
use crate::rng::BaseRng;

#[derive(Debug, Clone, PartialEq)]
/// Declarative stochastic primitive used by expression/runtime features.
pub enum StochasticSpec {
    UniformInt { min: i64, max: i64 },
    Bernoulli { p: f64, success: f64, failure: f64 },
    Dice { faces: u32, rolls: u32 },
    WeightedDiscrete { outcomes: Vec<(f64, f64)> },
}

fn invalid_parameter(name: impl Into<String>, reason: impl Into<String>) -> SetupError {
    SetupError::InvalidParameter { name: name.into(), reason: reason.into() }
}

/// Validates a stochastic specification before sampling.
pub fn validate_spec(spec: &StochasticSpec) -> Result<(), SetupError> {
    match spec {
        StochasticSpec::UniformInt { min, max } => {
            if min > max {
                return Err(invalid_parameter(
                    "uniform_int",
                    "min must be less than or equal to max",
                ));
            }
        }
        StochasticSpec::Bernoulli { p, success, failure } => {
            if !p.is_finite() || *p < 0.0 || *p > 1.0 {
                return Err(invalid_parameter(
                    "bernoulli.p",
                    "p must be finite and within [0.0, 1.0]",
                ));
            }
            if !success.is_finite() {
                return Err(invalid_parameter(
                    "bernoulli.success",
                    "success outcome must be finite",
                ));
            }
            if !failure.is_finite() {
                return Err(invalid_parameter(
                    "bernoulli.failure",
                    "failure outcome must be finite",
                ));
            }
        }
        StochasticSpec::Dice { faces, rolls } => {
            if *faces == 0 {
                return Err(invalid_parameter("dice.faces", "faces must be at least 1"));
            }
            if *rolls == 0 {
                return Err(invalid_parameter("dice.rolls", "rolls must be at least 1"));
            }
        }
        StochasticSpec::WeightedDiscrete { outcomes } => {
            if outcomes.is_empty() {
                return Err(invalid_parameter(
                    "weighted_discrete.outcomes",
                    "outcomes must contain at least one (value, weight) pair",
                ));
            }

            for (index, (value, weight)) in outcomes.iter().enumerate() {
                if !value.is_finite() {
                    return Err(invalid_parameter(
                        format!("weighted_discrete.outcomes[{index}].value"),
                        "value must be finite",
                    ));
                }
                if !weight.is_finite() || *weight <= 0.0 {
                    return Err(invalid_parameter(
                        format!("weighted_discrete.outcomes[{index}].weight"),
                        "weight must be finite and greater than 0.0",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn sample_validated(spec: &StochasticSpec, rng: &mut BaseRng) -> Result<f64, SetupError> {
    match spec {
        StochasticSpec::UniformInt { min, max } => Ok(rng.random_range(*min..=*max) as f64),
        StochasticSpec::Bernoulli { p, success, failure } => {
            let distribution = Bernoulli::new(*p).map_err(|err| {
                invalid_parameter("bernoulli.p", format!("invalid Bernoulli parameter: {err}"))
            })?;
            let is_success = distribution.sample(rng);
            Ok(if is_success { *success } else { *failure })
        }
        StochasticSpec::Dice { faces, rolls } => {
            let mut total = 0.0;
            for _ in 0..*rolls {
                total += rng.random_range(1_u32..=*faces) as f64;
            }
            Ok(total)
        }
        StochasticSpec::WeightedDiscrete { outcomes } => {
            if outcomes.len() == 1 {
                return Ok(outcomes[0].0);
            }

            let weights: Vec<f64> = outcomes.iter().map(|(_, weight)| *weight).collect();
            let distribution = WeightedAliasIndex::new(weights).map_err(|err| {
                invalid_parameter(
                    "weighted_discrete.outcomes",
                    format!("invalid weighted discrete distribution: {err}"),
                )
            })?;
            let index = distribution.sample(rng);
            Ok(outcomes[index].0)
        }
    }
}

/// Samples one integer value from an inclusive interval.
pub fn sample_closed_interval(min: i64, max: i64, rng: &mut BaseRng) -> Result<f64, SetupError> {
    if min > max {
        return Err(invalid_parameter("closed_interval", "min must be less than or equal to max"));
    }
    Ok(rng.random_range(min..=max) as f64)
}

/// Samples one value uniformly from a non-empty finite list.
pub fn sample_from_list(values: &[f64], rng: &mut BaseRng) -> Result<f64, SetupError> {
    if values.is_empty() {
        return Err(invalid_parameter(
            "random_list.values",
            "values must contain at least one element",
        ));
    }

    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(invalid_parameter(
                format!("random_list.values[{index}]"),
                "value must be finite",
            ));
        }
    }

    if values.len() == 1 {
        return Ok(values[0]);
    }

    let index = rng.random_range(0_usize..=values.len() - 1);
    Ok(values[index])
}

/// Samples one value uniformly from a non-empty matrix of finite values.
pub fn sample_from_matrix(values: &[Vec<f64>], rng: &mut BaseRng) -> Result<f64, SetupError> {
    if values.is_empty() {
        return Err(invalid_parameter(
            "random_matrix.values",
            "matrix must contain at least one row",
        ));
    }

    for (row_index, row) in values.iter().enumerate() {
        if row.is_empty() {
            return Err(invalid_parameter(
                format!("random_matrix.values[{row_index}]"),
                "rows must contain at least one element",
            ));
        }
        for (col_index, value) in row.iter().enumerate() {
            if !value.is_finite() {
                return Err(invalid_parameter(
                    format!("random_matrix.values[{row_index}][{col_index}]"),
                    "value must be finite",
                ));
            }
        }
    }

    if values.len() == 1 && values[0].len() == 1 {
        return Ok(values[0][0]);
    }

    let row_index = rng.random_range(0_usize..=values.len() - 1);
    let row = &values[row_index];
    let col_index = rng.random_range(0_usize..=row.len() - 1);
    Ok(row[col_index])
}

/// Samples one index according to positive weights.
pub fn sample_weighted_index(weights: &[f64], rng: &mut BaseRng) -> Result<usize, SetupError> {
    if weights.is_empty() {
        return Err(invalid_parameter(
            "weighted_index.weights",
            "weights must contain at least one element",
        ));
    }

    for (index, weight) in weights.iter().enumerate() {
        if !weight.is_finite() || *weight <= 0.0 {
            return Err(invalid_parameter(
                format!("weighted_index.weights[{index}]"),
                "weight must be finite and greater than 0.0",
            ));
        }
    }

    if weights.len() == 1 {
        return Ok(0);
    }

    let distribution = WeightedAliasIndex::new(weights.to_vec()).map_err(|err| {
        invalid_parameter(
            "weighted_index.weights",
            format!("invalid weighted index distribution: {err}"),
        )
    })?;
    Ok(distribution.sample(rng))
}

/// Samples a boolean outcome from a percent chance in `[0, +inf)`.
pub fn sample_chance_percent(percent: f64, rng: &mut BaseRng) -> Result<bool, SetupError> {
    if !percent.is_finite() || percent < 0.0 {
        return Err(invalid_parameter(
            "chance.percent",
            "percent must be finite and greater than or equal to 0.0",
        ));
    }

    if percent <= 0.0 {
        return Ok(false);
    }
    if percent >= 100.0 {
        return Ok(true);
    }

    let probability = percent / 100.0;
    let distribution = Bernoulli::new(probability).map_err(|err| {
        invalid_parameter("chance.percent", format!("invalid chance percent distribution: {err}"))
    })?;
    Ok(distribution.sample(rng))
}

/// Validates and samples one value from a stochastic spec.
pub fn sample(spec: &StochasticSpec, rng: &mut BaseRng) -> Result<f64, SetupError> {
    validate_spec(spec)?;
    sample_validated(spec, rng)
}

/// Validates and samples multiple values from a stochastic spec.
pub fn sample_many(
    spec: &StochasticSpec,
    rng: &mut BaseRng,
    count: usize,
) -> Result<Vec<f64>, SetupError> {
    validate_spec(spec)?;
    (0..count).map(|_| sample_validated(spec, rng)).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use rand::SeedableRng;

    use super::*;

    #[test]
    fn validate_uniform_int_rejects_reversed_bounds() {
        let spec = StochasticSpec::UniformInt { min: 8, max: 2 };
        assert!(matches!(
            validate_spec(&spec),
            Err(SetupError::InvalidParameter { name, .. }) if name == "uniform_int"
        ));
    }

    #[test]
    fn validate_bernoulli_rejects_out_of_range_probability() {
        let spec = StochasticSpec::Bernoulli { p: 1.2, success: 1.0, failure: 0.0 };
        assert!(matches!(
            validate_spec(&spec),
            Err(SetupError::InvalidParameter { name, .. }) if name == "bernoulli.p"
        ));
    }

    #[test]
    fn validate_bernoulli_rejects_non_finite_outcomes() {
        let non_finite_success =
            StochasticSpec::Bernoulli { p: 0.5, success: f64::NAN, failure: 0.0 };
        assert!(matches!(
            validate_spec(&non_finite_success),
            Err(SetupError::InvalidParameter { name, .. }) if name == "bernoulli.success"
        ));

        let non_finite_failure =
            StochasticSpec::Bernoulli { p: 0.5, success: 1.0, failure: f64::INFINITY };
        assert!(matches!(
            validate_spec(&non_finite_failure),
            Err(SetupError::InvalidParameter { name, .. }) if name == "bernoulli.failure"
        ));
    }

    #[test]
    fn validate_dice_rejects_zero_faces_or_rolls() {
        let zero_faces = StochasticSpec::Dice { faces: 0, rolls: 1 };
        assert!(validate_spec(&zero_faces).is_err());

        let zero_rolls = StochasticSpec::Dice { faces: 6, rolls: 0 };
        assert!(validate_spec(&zero_rolls).is_err());
    }

    #[test]
    fn validate_weighted_discrete_rejects_empty_outcomes() {
        let spec = StochasticSpec::WeightedDiscrete { outcomes: Vec::new() };
        assert!(matches!(
            validate_spec(&spec),
            Err(SetupError::InvalidParameter { name, .. })
                if name == "weighted_discrete.outcomes"
        ));
    }

    #[test]
    fn validate_weighted_discrete_rejects_non_positive_weights() {
        let spec = StochasticSpec::WeightedDiscrete { outcomes: vec![(1.0, 0.0), (2.0, 1.0)] };
        assert!(matches!(
            validate_spec(&spec),
            Err(SetupError::InvalidParameter { name, .. })
                if name == "weighted_discrete.outcomes[0].weight"
        ));
    }

    #[test]
    fn sample_uniform_int_is_reproducible_for_fixed_seed() {
        let spec = StochasticSpec::UniformInt { min: 3, max: 7 };
        let mut rng_a = BaseRng::seed_from_u64(101);
        let mut rng_b = BaseRng::seed_from_u64(101);

        let a = sample_many(&spec, &mut rng_a, 32).expect("valid spec");
        let b = sample_many(&spec, &mut rng_b, 32).expect("valid spec");

        assert_eq!(a, b);
    }

    #[test]
    fn sample_bernoulli_is_reproducible_for_fixed_seed() {
        let spec = StochasticSpec::Bernoulli { p: 0.35, success: 1.0, failure: -1.0 };
        let mut rng_a = BaseRng::seed_from_u64(202);
        let mut rng_b = BaseRng::seed_from_u64(202);

        let a = sample_many(&spec, &mut rng_a, 48).expect("valid spec");
        let b = sample_many(&spec, &mut rng_b, 48).expect("valid spec");

        assert_eq!(a, b);
    }

    #[test]
    fn sample_dice_is_reproducible_for_fixed_seed() {
        let spec = StochasticSpec::Dice { faces: 6, rolls: 3 };
        let mut rng_a = BaseRng::seed_from_u64(303);
        let mut rng_b = BaseRng::seed_from_u64(303);

        let a = sample_many(&spec, &mut rng_a, 24).expect("valid spec");
        let b = sample_many(&spec, &mut rng_b, 24).expect("valid spec");

        assert_eq!(a, b);
    }

    #[test]
    fn sample_weighted_discrete_is_reproducible_for_fixed_seed() {
        let spec =
            StochasticSpec::WeightedDiscrete { outcomes: vec![(1.0, 0.2), (2.0, 0.5), (3.0, 0.3)] };
        let mut rng_a = BaseRng::seed_from_u64(404);
        let mut rng_b = BaseRng::seed_from_u64(404);

        let a = sample_many(&spec, &mut rng_a, 40).expect("valid spec");
        let b = sample_many(&spec, &mut rng_b, 40).expect("valid spec");

        assert_eq!(a, b);
    }

    #[test]
    fn sample_many_matches_repeated_single_sample_calls() {
        let spec = StochasticSpec::Dice { faces: 12, rolls: 2 };
        let mut rng_many = BaseRng::seed_from_u64(505);
        let mut rng_single = BaseRng::seed_from_u64(505);

        let draws_many = sample_many(&spec, &mut rng_many, 16).expect("valid spec");
        let mut draws_single = Vec::with_capacity(16);
        for _ in 0..16 {
            draws_single.push(sample(&spec, &mut rng_single).expect("valid spec"));
        }

        assert_eq!(draws_many, draws_single);
    }

    #[test]
    fn sample_closed_interval_is_inclusive_and_reproducible() {
        let mut rng_a = BaseRng::seed_from_u64(606);
        let mut rng_b = BaseRng::seed_from_u64(606);

        let draws_a = (0..128)
            .map(|_| sample_closed_interval(2, 4, &mut rng_a).expect("valid closed interval"))
            .collect::<Vec<_>>();
        let draws_b = (0..128)
            .map(|_| sample_closed_interval(2, 4, &mut rng_b).expect("valid closed interval"))
            .collect::<Vec<_>>();

        assert_eq!(draws_a, draws_b);
        assert!(draws_a.iter().all(|value| *value >= 2.0 && *value <= 4.0));
        let observed = draws_a.into_iter().map(|value| value as i64).collect::<BTreeSet<_>>();
        assert!(observed.contains(&2));
        assert!(observed.contains(&4));
    }

    #[test]
    fn sample_closed_interval_rejects_reversed_bounds() {
        let mut rng = BaseRng::seed_from_u64(606);
        assert!(matches!(
            sample_closed_interval(5, 4, &mut rng),
            Err(SetupError::InvalidParameter { name, .. }) if name == "closed_interval"
        ));
    }

    #[test]
    fn sample_from_list_is_reproducible_for_fixed_seed() {
        let values = vec![10.0, 20.0, 30.0, 40.0];
        let mut rng_a = BaseRng::seed_from_u64(707);
        let mut rng_b = BaseRng::seed_from_u64(707);

        let draws_a = (0..64)
            .map(|_| sample_from_list(&values, &mut rng_a).expect("valid list"))
            .collect::<Vec<_>>();
        let draws_b = (0..64)
            .map(|_| sample_from_list(&values, &mut rng_b).expect("valid list"))
            .collect::<Vec<_>>();

        assert_eq!(draws_a, draws_b);
        assert!(draws_a.iter().all(|value| values.contains(value)));
    }

    #[test]
    fn sample_from_list_supports_singleton_and_rejects_non_finite_values() {
        let mut rng = BaseRng::seed_from_u64(717);
        let single = sample_from_list(&[42.0], &mut rng).expect("singleton list should work");
        assert_eq!(single, 42.0);

        assert!(matches!(
            sample_from_list(&[1.0, f64::NAN], &mut rng),
            Err(SetupError::InvalidParameter { name, .. }) if name == "random_list.values[1]"
        ));
    }

    #[test]
    fn sample_from_matrix_is_reproducible_for_fixed_seed() {
        let matrix = vec![vec![1.0, 2.0], vec![3.0], vec![4.0, 5.0, 6.0]];
        let mut rng_a = BaseRng::seed_from_u64(808);
        let mut rng_b = BaseRng::seed_from_u64(808);

        let draws_a = (0..64)
            .map(|_| sample_from_matrix(&matrix, &mut rng_a).expect("valid matrix"))
            .collect::<Vec<_>>();
        let draws_b = (0..64)
            .map(|_| sample_from_matrix(&matrix, &mut rng_b).expect("valid matrix"))
            .collect::<Vec<_>>();

        assert_eq!(draws_a, draws_b);
        assert!(draws_a.iter().all(|value| (1.0..=6.0).contains(value)));
    }

    #[test]
    fn sample_from_matrix_supports_singleton_and_rejects_invalid_shapes() {
        let mut rng = BaseRng::seed_from_u64(818);
        let single =
            sample_from_matrix(&[vec![7.0]], &mut rng).expect("singleton matrix should work");
        assert_eq!(single, 7.0);

        assert!(matches!(
            sample_from_matrix(&[Vec::new()], &mut rng),
            Err(SetupError::InvalidParameter { name, .. }) if name == "random_matrix.values[0]"
        ));
        assert!(matches!(
            sample_from_matrix(&[vec![1.0, f64::NEG_INFINITY]], &mut rng),
            Err(SetupError::InvalidParameter { name, .. }) if name == "random_matrix.values[0][1]"
        ));
    }

    #[test]
    fn sample_weighted_index_rejects_invalid_weights() {
        assert!(matches!(
            sample_weighted_index(&[], &mut BaseRng::seed_from_u64(1)),
            Err(SetupError::InvalidParameter { name, .. }) if name == "weighted_index.weights"
        ));

        assert!(matches!(
            sample_weighted_index(&[1.0, 0.0], &mut BaseRng::seed_from_u64(2)),
            Err(SetupError::InvalidParameter { name, .. })
                if name == "weighted_index.weights[1]"
        ));
    }

    #[test]
    fn sample_weighted_index_is_reproducible_for_fixed_seed() {
        let mut rng_a = BaseRng::seed_from_u64(909);
        let mut rng_b = BaseRng::seed_from_u64(909);

        let draws_a = (0..64)
            .map(|_| sample_weighted_index(&[1.0, 3.0, 6.0], &mut rng_a).expect("valid weights"))
            .collect::<Vec<_>>();
        let draws_b = (0..64)
            .map(|_| sample_weighted_index(&[1.0, 3.0, 6.0], &mut rng_b).expect("valid weights"))
            .collect::<Vec<_>>();

        assert_eq!(draws_a, draws_b);
        assert!(draws_a.iter().all(|draw| *draw <= 2));
    }

    #[test]
    fn sample_weighted_index_and_discrete_singleton_short_circuit() {
        let mut rng = BaseRng::seed_from_u64(919);
        let index = sample_weighted_index(&[2.0], &mut rng).expect("singleton index should work");
        assert_eq!(index, 0);

        let value =
            sample(&StochasticSpec::WeightedDiscrete { outcomes: vec![(9.0, 1.0)] }, &mut rng)
                .expect("singleton weighted discrete should work");
        assert_eq!(value, 9.0);
    }

    #[test]
    fn sample_chance_percent_is_reproducible_for_fixed_seed() {
        let mut rng_a = BaseRng::seed_from_u64(1001);
        let mut rng_b = BaseRng::seed_from_u64(1001);

        let draws_a = (0..96)
            .map(|_| sample_chance_percent(37.5, &mut rng_a).expect("valid chance"))
            .collect::<Vec<_>>();
        let draws_b = (0..96)
            .map(|_| sample_chance_percent(37.5, &mut rng_b).expect("valid chance"))
            .collect::<Vec<_>>();

        assert_eq!(draws_a, draws_b);
    }

    #[test]
    fn sample_chance_percent_handles_bounds_and_invalid_values() {
        let mut rng = BaseRng::seed_from_u64(1111);
        assert!(!sample_chance_percent(0.0, &mut rng).expect("0% chance is valid"));
        assert!(sample_chance_percent(150.0, &mut rng).expect(">100% chance is valid"));
        assert!(matches!(
            sample_chance_percent(-0.1, &mut rng),
            Err(SetupError::InvalidParameter { name, .. }) if name == "chance.percent"
        ));
    }
}
