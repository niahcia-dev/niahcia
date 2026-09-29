use crate::work::{keccak256, Hash32};

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
        median_time_past, pow_meets_target, randomx_seed, randomx_seed_height, validate_timestamp,
        RANDOMX_EPOCH_LENGTH, RANDOMX_SEED_LAG,
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

    #[test]
    fn timestamp_must_be_after_median_and_not_too_far_future() {
        let ancestors: Vec<u64> = (100..=110).collect();

        assert!(validate_timestamp(106, &ancestors, 110).is_ok());
        assert!(validate_timestamp(105, &ancestors, 110).is_err());
        assert!(validate_timestamp(201, &ancestors, 110).is_err());
    }
}
