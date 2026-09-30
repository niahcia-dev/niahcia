//! Deterministic NIAHCIA monetary-policy candidate arithmetic.
//!
//! These constants implement `docs/emission-candidate-v1.md`. They remain a
//! candidate until the comparison/vector review promotes them to production.

pub const ANIAH_PER_NIAH: u128 = 100_000_000;
pub const MAIN_EMISSION_REFERENCE: u128 = 41_943_040u128 * ANIAH_PER_NIAH;
pub const EMISSION_SHIFT: u32 = 22;
pub const TAIL_SUBSIDY: u128 = ANIAH_PER_NIAH / 4;

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
    fn candidate_constants_are_exact() {
        assert_eq!(ANIAH_PER_NIAH, 100_000_000);
        assert_eq!(MonetaryPolicyV1::cpu_subsidy(0), 10 * ANIAH_PER_NIAH);
        assert_eq!(TAIL_SUBSIDY, 25_000_000);
    }

    #[test]
    fn subsidy_never_falls_below_tail() {
        assert_eq!(
            MonetaryPolicyV1::cpu_subsidy(MAIN_EMISSION_REFERENCE),
            TAIL_SUBSIDY
        );
        assert_eq!(
            MonetaryPolicyV1::cpu_subsidy(u128::MAX),
            TAIL_SUBSIDY
        );
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
