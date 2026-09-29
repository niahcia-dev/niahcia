use crate::work::{keccak256, Hash32};
use num_bigint::BigUint;
use num_traits::{One, Zero};

pub const TARGET_BLOCK_INTERVAL: u64 = 30;
pub const RANDOMX_EPOCH_LENGTH: u64 = 2_048;
pub const RANDOMX_SEED_LAG: u64 = 64;
pub const MEDIAN_TIME_WINDOW: usize = 11;
pub const MAX_FUTURE_DRIFT: u64 = 90;

const RANDOMX_SEED_DOMAIN: &[u8] = b"NIAHCIA/RANDOMX-SEED/V1";

pub fn randomx_seed_height(height: u64) -> u64 {
    let epoch_start = (height / RANDOMX_EPOCH_LENGTH) * RANDOMX_EPOCH_LENGTH;
    epoch_start.saturating_sub(RANDOMX_SEED_LAG)
}

pub fn randomx_seed(seed_block_id: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(RANDOMX_SEED_DOMAIN.len() + 32);
    preimage.extend_from_slice(RANDOMX_SEED_DOMAIN);
    preimage.extend_from_slice(&seed_block_id);
    keccak256(&preimage)
}

pub fn pow_meets_target(pow_hash: Hash32, target: Hash32) -> bool {
    pow_hash <= target
}

pub const DIFFICULTY_WINDOW: usize = 60;
pub const MIN_SOLVE_TIME: u64 = 1;
pub const MAX_SOLVE_TIME: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifficultySample {
    pub target: Hash32,
    pub solve_time: u64,
}

pub fn next_target(
    previous_target: Hash32,
    samples: &[DifficultySample],
    pow_limit: Hash32,
) -> Hash32 {
    if samples.is_empty() {
        return previous_target;
    }

    let start = samples.len().saturating_sub(DIFFICULTY_WINDOW);
    let window = &samples[start..];

    let mut weighted = BigUint::zero();
    for sample in window {
        let bounded = sample.solve_time.clamp(MIN_SOLVE_TIME, MAX_SOLVE_TIME);
        weighted += hash32_to_biguint(sample.target) * BigUint::from(bounded);
    }

    let divisor = BigUint::from(TARGET_BLOCK_INTERVAL) * BigUint::from(window.len());
    let raw = weighted / divisor;

    let previous = hash32_to_biguint(previous_target);
    let harder = (&previous * BigUint::from(7_u8) + BigUint::from(7_u8)) / BigUint::from(8_u8);
    let easier = (&previous * BigUint::from(9_u8)) / BigUint::from(8_u8);
    let limit = hash32_to_biguint(pow_limit);

    let bounded = raw.max(harder).min(easier).min(limit);
    biguint_to_hash32(&bounded)
}

pub fn block_work(target: Hash32) -> BigUint {
    let max = (BigUint::one() << 256_usize) - BigUint::one();
    let denominator = hash32_to_biguint(target) + BigUint::one();
    (max / denominator) + BigUint::one()
}

fn hash32_to_biguint(value: Hash32) -> BigUint {
    BigUint::from_bytes_be(&value)
}

fn biguint_to_hash32(value: &BigUint) -> Hash32 {
    let bytes = value.to_bytes_be();
    assert!(bytes.len() <= 32, "256-bit target overflow");

    let mut out = [0_u8; 32];
    out[32 - bytes.len()..].copy_from_slice(&bytes);
    out
}

pub fn median_time_past(timestamps: &[u64]) -> Option<u64> {
    if timestamps.is_empty() {
        return None;
    }

    let start = timestamps.len().saturating_sub(MEDIAN_TIME_WINDOW);
    let mut window = timestamps[start..].to_vec();
    window.sort_unstable();
    Some(window[window.len() / 2])
}

pub fn validate_timestamp(
    candidate_timestamp: u64,
    ancestor_timestamps: &[u64],
    adjusted_time: u64,
) -> Result<(), String> {
    if let Some(median) = median_time_past(ancestor_timestamps) {
        if candidate_timestamp <= median {
            return Err(format!(
                "timestamp {candidate_timestamp} must be greater than median time past {median}"
            ));
        }
    }

    let latest_allowed = adjusted_time.saturating_add(MAX_FUTURE_DRIFT);
    if candidate_timestamp > latest_allowed {
        return Err(format!(
            "timestamp {candidate_timestamp} exceeds maximum future time {latest_allowed}"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        block_work, median_time_past, next_target, pow_meets_target, randomx_seed,
        randomx_seed_height, validate_timestamp, DifficultySample, RANDOMX_EPOCH_LENGTH,
        RANDOMX_SEED_LAG,
    };

    #[test]
    fn seed_height_uses_genesis_during_first_epoch() {
        assert_eq!(randomx_seed_height(0), 0);
        assert_eq!(randomx_seed_height(RANDOMX_EPOCH_LENGTH - 1), 0);
    }

    #[test]
    fn seed_height_uses_lagged_prior_history_at_epoch_boundary() {
        assert_eq!(
            randomx_seed_height(RANDOMX_EPOCH_LENGTH),
            RANDOMX_EPOCH_LENGTH - RANDOMX_SEED_LAG
        );
    }

    #[test]
    fn seed_is_niahcia_domain_separated() {
        let first = randomx_seed([0x11; 32]);
        let second = randomx_seed([0x11; 32]);
        let changed = randomx_seed([0x12; 32]);

        assert_eq!(first, second);
        assert_ne!(first, changed);
    }

    #[test]
    fn target_comparison_is_unsigned_big_endian() {
        assert!(pow_meets_target([0x00; 32], [0xff; 32]));
        assert!(pow_meets_target([0x11; 32], [0x11; 32]));
        assert!(!pow_meets_target([0x12; 32], [0x11; 32]));
    }

    #[test]
    fn median_time_uses_last_eleven_ancestors() {
        let timestamps: Vec<u64> = (1..=20).collect();
        assert_eq!(median_time_past(&timestamps), Some(15));
    }

    fn target_u64(value: u64) -> [u8; 32] {
        let mut out = [0_u8; 32];
        out[24..].copy_from_slice(&value.to_be_bytes());
        out
    }

    #[test]
    fn difficulty_vector_steady() {
        let samples = vec![
            DifficultySample {
                target: target_u64(1000),
                solve_time: 30,
            };
            60
        ];

        assert_eq!(
            next_target(target_u64(1000), &samples, [0xff; 32]),
            target_u64(1000)
        );
    }

    #[test]
    fn difficulty_vector_fast_window_hits_harder_clamp() {
        let samples = vec![
            DifficultySample {
                target: target_u64(1000),
                solve_time: 15,
            };
            60
        ];

        assert_eq!(
            next_target(target_u64(1000), &samples, [0xff; 32]),
            target_u64(875)
        );
    }

    #[test]
    fn difficulty_vector_slow_window_hits_easier_clamp() {
        let samples = vec![
            DifficultySample {
                target: target_u64(1000),
                solve_time: 60,
            };
            60
        ];

        assert_eq!(
            next_target(target_u64(1000), &samples, [0xff; 32]),
            target_u64(1125)
        );
    }

    #[test]
    fn difficulty_vector_long_outlier_is_bounded() {
        let mut samples = vec![
            DifficultySample {
                target: target_u64(1000),
                solve_time: 30,
            };
            59
        ];
        samples.push(DifficultySample {
            target: target_u64(1000),
            solve_time: 3600,
        });

        assert_eq!(
            next_target(target_u64(1000), &samples, [0xff; 32]),
            target_u64(1125)
        );
    }

    #[test]
    fn block_work_matches_protocol_vectors() {
        assert_eq!(block_work([0xff; 32]).to_str_radix(10), "1");
        assert_eq!(
            block_work(target_u64(1000)).to_str_radix(10),
            "115676412824491703719851133874813094758511473192448115923534049957955174466"
        );
        assert_eq!(
            block_work(target_u64(1)).to_str_radix(10),
            "57896044618658097711785492504343953926634992332820282019728792003956564819968"
        );
    }

    #[test]
    fn timestamp_must_be_after_median_and_not_too_far_future() {
        let ancestors: Vec<u64> = (100..=110).collect();

        assert!(validate_timestamp(106, &ancestors, 110).is_ok());
        assert!(validate_timestamp(105, &ancestors, 110).is_err());
        assert!(validate_timestamp(201, &ancestors, 110).is_err());
    }
}
