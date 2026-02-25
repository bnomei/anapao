use rand::{distr::Distribution, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Base deterministic RNG for simulation runs.
pub type BaseRng = ChaCha8Rng;

const SPLITMIX64_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
const SPLITMIX64_MUL1: u64 = 0xBF58_476D_1CE4_E5B9;
const SPLITMIX64_MUL2: u64 = 0x94D0_49BB_1331_11EB;

/// SplitMix64-style mixer used for deterministic seed derivation.
pub fn splitmix64_mix(value: u64) -> u64 {
    let mut mixed = value.wrapping_add(SPLITMIX64_GAMMA);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(SPLITMIX64_MUL1);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(SPLITMIX64_MUL2);
    mixed ^ (mixed >> 31)
}

/// Derives a deterministic per-run seed from a master seed and run identifier.
pub fn derive_run_seed(master_seed: u64, run_id: u64) -> u64 {
    splitmix64_mix(master_seed ^ run_id)
}

/// Creates the base RNG from a u64 seed.
pub fn rng_from_seed(seed: u64) -> BaseRng {
    BaseRng::seed_from_u64(seed)
}

/// Creates a deterministic per-run RNG from the master seed and run id.
pub fn run_rng(master_seed: u64, run_id: u64) -> BaseRng {
    rng_from_seed(derive_run_seed(master_seed, run_id))
}

/// Runs a closure with a deterministic per-run RNG.
pub fn with_run_rng<T>(
    master_seed: u64,
    run_id: u64,
    draw_fn: impl FnOnce(&mut BaseRng) -> T,
) -> T {
    let mut rng = run_rng(master_seed, run_id);
    draw_fn(&mut rng)
}

/// Helpers for deterministic draw workflows used by engine and batch layers.
pub trait DeterministicDrawExt: Rng {
    /// Draws one sample from a distribution.
    fn draw<T, D>(&mut self, distribution: D) -> T
    where
        D: Distribution<T>,
    {
        distribution.sample(self)
    }

    /// Draws a fixed number of samples from a clonable distribution.
    fn draw_many<T, D>(&mut self, distribution: D, count: usize) -> Vec<T>
    where
        D: Distribution<T> + Clone,
    {
        (0..count).map(|_| distribution.clone().sample(self)).collect()
    }
}

impl<R: Rng + ?Sized> DeterministicDrawExt for R {}

/// Draws a single deterministic sample for a run.
pub fn draw_for_run<T, D>(master_seed: u64, run_id: u64, distribution: D) -> T
where
    D: Distribution<T>,
{
    with_run_rng(master_seed, run_id, |rng| rng.draw(distribution))
}

/// Draws multiple deterministic samples for a run.
pub fn draw_many_for_run<T, D>(
    master_seed: u64,
    run_id: u64,
    distribution: D,
    count: usize,
) -> Vec<T>
where
    D: Distribution<T> + Clone,
{
    with_run_rng(master_seed, run_id, |rng| rng.draw_many(distribution, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distr::Uniform;
    use rand::RngExt;
    use std::collections::HashSet;

    fn hamming_distance(left: u64, right: u64) -> u32 {
        (left ^ right).count_ones()
    }

    #[test]
    fn splitmix64_mix_matches_known_vectors() {
        assert_eq!(splitmix64_mix(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64_mix(1), 0x910A_2DEC_8902_5CC1);
        assert_eq!(splitmix64_mix(42), 0xBDD7_3226_2FEB_6E95);
        assert_eq!(splitmix64_mix(u64::MAX), 0xE4D9_7177_1B65_2C20);
    }

    #[test]
    fn derive_run_seed_is_reproducible() {
        let master_seed = 0xCAFE_BABE_F00D_u64;
        let run_id = 17_u64;

        let left = derive_run_seed(master_seed, run_id);
        let right = derive_run_seed(master_seed, run_id);

        assert_eq!(left, right);
    }

    #[test]
    fn derive_run_seed_is_unique_for_sampled_runs() {
        let master_seed = 0xDEAD_BEEF_u64;
        let mut seen = HashSet::with_capacity(10_000);

        for run_id in 0_u64..10_000_u64 {
            assert!(seen.insert(derive_run_seed(master_seed, run_id)));
        }
    }

    #[test]
    fn splitmix64_mix_neighboring_inputs_have_bit_diffusion() {
        let sample = 16_384_u64;
        let mut min_distance = u32::MAX;
        let mut total_distance = 0_u64;

        for value in 0_u64..sample {
            let distance = hamming_distance(splitmix64_mix(value), splitmix64_mix(value + 1));
            min_distance = min_distance.min(distance);
            total_distance = total_distance.saturating_add(distance as u64);
        }

        let average_distance = total_distance as f64 / sample as f64;
        assert!(
            min_distance >= 8,
            "splitmix64 diffusion floor regressed: min distance={min_distance}"
        );
        assert!(
            average_distance >= 28.0,
            "splitmix64 average diffusion regressed: avg={average_distance}"
        );
    }

    #[test]
    fn derive_run_seed_matches_splitmix_reference_formula() {
        let master_seeds =
            [0_u64, 1_u64, 0x0000_FFFF_0000_FFFF_u64, 0xCAFE_BABE_F00D_u64, u64::MAX];

        for master_seed in master_seeds {
            for run_id in 0_u64..2_048_u64 {
                assert_eq!(
                    derive_run_seed(master_seed, run_id),
                    splitmix64_mix(master_seed ^ run_id),
                );
            }
        }
    }

    #[test]
    fn rng_from_seed_is_reproducible() {
        let seed = 1337_u64;
        let mut left = rng_from_seed(seed);
        let mut right = rng_from_seed(seed);

        let draws_left: Vec<u64> = (0..32).map(|_| left.random()).collect();
        let draws_right: Vec<u64> = (0..32).map(|_| right.random()).collect();

        assert_eq!(draws_left, draws_right);
    }

    #[test]
    fn run_rng_matches_seeded_rng_for_run_seed() {
        let master_seed = 2026_u64;
        let run_id = 9_u64;
        let mut from_run = run_rng(master_seed, run_id);
        let mut from_seed = rng_from_seed(derive_run_seed(master_seed, run_id));

        for _ in 0..16 {
            assert_eq!(from_run.random::<u64>(), from_seed.random::<u64>());
        }
    }

    #[test]
    fn run_rng_matches_seeded_rng_for_many_run_ids() {
        let master_seed = 0x0B5E_ED_u64;
        let mut seen_prefixes = HashSet::with_capacity(2_048);

        for run_id in 0_u64..2_048_u64 {
            let mut from_run = run_rng(master_seed, run_id);
            let mut from_seed = rng_from_seed(derive_run_seed(master_seed, run_id));

            for _ in 0..8 {
                assert_eq!(from_run.random::<u64>(), from_seed.random::<u64>());
            }

            let prefix = draw_many_for_run(
                master_seed,
                run_id,
                Uniform::new(0_u64, u64::MAX).expect("uniform range is valid"),
                4,
            );
            assert!(
                seen_prefixes.insert(prefix),
                "derived stream collision for run_id={run_id} under stress sample"
            );
        }
    }

    #[test]
    fn deterministic_draw_helpers_are_reproducible() {
        let master_seed = 7_u64;
        let run_id = 21_u64;
        let distribution = Uniform::new(10_i32, 20_i32).expect("uniform range is valid");

        let run_draws_a = draw_many_for_run(master_seed, run_id, distribution, 24);
        let run_draws_b = draw_many_for_run(master_seed, run_id, distribution, 24);
        assert_eq!(run_draws_a, run_draws_b);

        let one_a = draw_for_run(master_seed, run_id, distribution);
        let one_b = with_run_rng(master_seed, run_id, |rng| rng.draw(distribution));
        assert_eq!(one_a, one_b);
    }

    #[test]
    fn draw_many_for_run_is_prefix_stable() {
        let master_seed = 11_u64;
        let run_id = 42_u64;
        let distribution = Uniform::new(1_i64, 100_i64).expect("uniform range is valid");

        let short = draw_many_for_run(master_seed, run_id, distribution, 16);
        let long = draw_many_for_run(master_seed, run_id, distribution, 64);

        assert_eq!(short, long[..16]);
    }
}
