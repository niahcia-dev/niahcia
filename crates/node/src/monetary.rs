//! Deterministic NIAHCIA monetary-policy V1 arithmetic.
//!
//! The selected pre-production monetary-policy candidate is:
//! - 100,000,000 aniah = 1 NIAH
//! - 10 NIAH initial CPU-PoW subsidy
//! - smooth shift=22 decay
//! - 0.25 NIAH permanent CPU-PoW tail subsidy
//!
//! Consensus activation remains gated on fee-policy and canonical-state
//! integration tests.

pub const ANIAH_PER_NIAH: u128 = 100_000_000;
pub const MAIN_EMISSION_REFERENCE: u128 = 41_943_040u128 * ANIAH_PER_NIAH;
pub const EMISSION_SHIFT: u32 = 22;
pub const TAIL_SUBSIDY: u128 = ANIAH_PER_NIAH / 4;
pub const TAIL_TRANSITION_BLOCKS: u64 = 15_472_280;
pub const TAIL_TRANSITION_ISSUED: u128 = 4_089_446_397_817_847;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonetaryPolicyV1;

impl MonetaryPolicyV1 {
    /// CPU-PoW subsidy for the next canonical block, given cumulative CPU
    /// subsidy already issued before that block.
    pub const fn cpu_subsidy(issued_cpu_subsidy: u128) -> u128 {
        let accounted = if issued_cpu_subsidy > MAIN_EMISSION_REFERENCE {
            MAIN_EMISSION_REFERENCE
        } else {
            issued_cpu_subsidy
        };
        let decay = (MAIN_EMISSION_REFERENCE - accounted) >> EMISSION_SHIFT;
        if decay > TAIL_SUBSIDY {
            decay
        } else {
            TAIL_SUBSIDY
        }
    }

    /// Apply one canonical CPU-PoW block using checked arithmetic.
    pub fn issue_next(issued_cpu_subsidy: u128) -> Option<(u128, u128)> {
        let subsidy = Self::cpu_subsidy(issued_cpu_subsidy);
        issued_cpu_subsidy
            .checked_add(subsidy)
            .map(|next_total| (subsidy, next_total))
    }

    /// Exact reference implementation used for vector generation and tests.
    /// Returns cumulative CPU subsidy after `blocks` canonical blocks.
    pub fn cumulative_after(blocks: u64) -> Option<u128> {
        let mut issued = 0u128;
        let mut height = 0u64;
        while height < blocks {
            (_, issued) = Self::issue_next(issued)?;
            height += 1;
        }
        Some(issued)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_constants_are_exact() {
        assert_eq!(ANIAH_PER_NIAH, 100_000_000);
        assert_eq!(MAIN_EMISSION_REFERENCE, 4_194_304_000_000_000);
        assert_eq!(EMISSION_SHIFT, 22);
        assert_eq!(MonetaryPolicyV1::cpu_subsidy(0), 10 * ANIAH_PER_NIAH);
        assert_eq!(TAIL_SUBSIDY, 25_000_000);
    }

    #[test]
    fn canonical_early_vectors_match() {
        let vectors = [
            (0u64, 0u128, 1_000_000_000u128),
            (1, 1_000_000_000, 999_999_761),
            (2, 1_999_999_761, 999_999_523),
            (10, 9_999_989_267, 999_997_615),
            (100, 99_998_819_787, 999_976_158),
            (1_000, 999_880_918_866, 999_761_609),
        ];
        for (blocks, expected_issued, expected_next) in vectors {
            let issued = MonetaryPolicyV1::cumulative_after(blocks).unwrap();
            assert_eq!(issued, expected_issued, "issued mismatch at {blocks}");
            assert_eq!(
                MonetaryPolicyV1::cpu_subsidy(issued),
                expected_next,
                "subsidy mismatch at {blocks}"
            );
        }
    }

    #[test]
    fn canonical_time_vectors_match() {
        let vectors = [
            (1_051_920u64, 930_380_129_401_329u128, 778_180_091u128),
            (5_259_600, 2_997_396_786_764_314, 285_364_917),
            (10_519_200, 3_852_748_671_251_556, 81_433_136),
        ];
        for (blocks, expected_issued, expected_next) in vectors {
            let issued = MonetaryPolicyV1::cumulative_after(blocks).unwrap();
            assert_eq!(issued, expected_issued, "issued mismatch at {blocks}");
            assert_eq!(
                MonetaryPolicyV1::cpu_subsidy(issued),
                expected_next,
                "subsidy mismatch at {blocks}"
            );
        }
    }

    #[test]
    fn tail_transition_is_exact() {
        let issued = MonetaryPolicyV1::cumulative_after(TAIL_TRANSITION_BLOCKS).unwrap();
        assert_eq!(issued, TAIL_TRANSITION_ISSUED);
        assert_eq!(MonetaryPolicyV1::cpu_subsidy(issued), TAIL_SUBSIDY);

        let before = MonetaryPolicyV1::cumulative_after(TAIL_TRANSITION_BLOCKS - 1).unwrap();
        assert!(MonetaryPolicyV1::cpu_subsidy(before) > TAIL_SUBSIDY);
    }

    #[test]
    fn subsidy_never_falls_below_tail() {
        assert_eq!(
            MonetaryPolicyV1::cpu_subsidy(MAIN_EMISSION_REFERENCE),
            TAIL_SUBSIDY
        );
        assert_eq!(MonetaryPolicyV1::cpu_subsidy(u128::MAX), TAIL_SUBSIDY);
    }

    #[test]
    fn subsidy_is_monotonic_nonincreasing() {
        let mut issued = 0u128;
        let mut previous = u128::MAX;
        for _ in 0..100_000 {
            let (reward, next) = MonetaryPolicyV1::issue_next(issued).unwrap();
            assert!(reward <= previous);
            assert!(reward >= TAIL_SUBSIDY);
            previous = reward;
            issued = next;
        }
    }

    #[test]
    fn cumulative_matches_iterative_issue() {
        let blocks = 10_000u64;
        let from_helper = MonetaryPolicyV1::cumulative_after(blocks).unwrap();
        let mut issued = 0u128;
        for _ in 0..blocks {
            (_, issued) = MonetaryPolicyV1::issue_next(issued).unwrap();
        }
        assert_eq!(from_helper, issued);
    }

    #[test]
    fn checked_issue_rejects_overflow() {
        assert!(MonetaryPolicyV1::issue_next(u128::MAX).is_none());
    }
}
