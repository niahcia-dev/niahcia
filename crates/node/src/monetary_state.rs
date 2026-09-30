//! Reorg-safe monetary accounting for NIAHCIA V1.
//!
//! This layer deliberately does not credit spendable miner balances yet. It
//! records deterministic canonical monetary effects so restart/replay/reorg
//! behavior can be tested before subsidy payout is activated.

use crate::monetary::MonetaryPolicyV1;
use crate::work::Hash32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonetaryTotalsV1 {
    pub gross_issued: u128,
    pub protocol_burned: u128,
}

impl MonetaryTotalsV1 {
    pub fn net_protocol_supply(self) -> Option<u128> {
        self.gross_issued.checked_sub(self.protocol_burned)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockMonetaryEffectV1 {
    pub block_id: Hash32,
    pub cpu_subsidy: u128,
    pub base_fee_burned: u128,
    pub priority_fee_to_miner: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MonetaryStateV1 {
    totals: MonetaryTotalsV1,
    canonical_effects: Vec<BlockMonetaryEffectV1>,
}

impl MonetaryStateV1 {
    pub fn totals(&self) -> MonetaryTotalsV1 {
        self.totals
    }

    pub fn canonical_effects(&self) -> &[BlockMonetaryEffectV1] {
        &self.canonical_effects
    }

    pub fn next_cpu_subsidy(&self) -> u128 {
        MonetaryPolicyV1::cpu_subsidy(self.totals.gross_issued)
    }

    /// Apply one canonical block exactly once.
    ///
    /// `base_fee_burned` and `priority_fee_to_miner` must already have been
    /// derived from independently validated execution receipts. Priority fees
    /// are tracked for auditability but do not change protocol supply.
    pub fn apply_block(
        &mut self,
        block_id: Hash32,
        base_fee_burned: u128,
        priority_fee_to_miner: u128,
    ) -> Result<BlockMonetaryEffectV1, String> {
        if self
            .canonical_effects
            .iter()
            .any(|effect| effect.block_id == block_id)
        {
            return Err("monetary effect for block is already canonical".into());
        }

        let subsidy = self.next_cpu_subsidy();
        let gross_issued = self
            .totals
            .gross_issued
            .checked_add(subsidy)
            .ok_or_else(|| "gross issuance overflow".to_string())?;
        let protocol_burned = self
            .totals
            .protocol_burned
            .checked_add(base_fee_burned)
            .ok_or_else(|| "protocol burn overflow".to_string())?;

        if protocol_burned > gross_issued {
            return Err("protocol burned supply would exceed gross issued supply".into());
        }

        let effect = BlockMonetaryEffectV1 {
            block_id,
            cpu_subsidy: subsidy,
            base_fee_burned,
            priority_fee_to_miner,
        };
        self.totals = MonetaryTotalsV1 {
            gross_issued,
            protocol_burned,
        };
        self.canonical_effects.push(effect);
        Ok(effect)
    }

    /// Detach the current canonical tip. Rollback is tip-only by construction,
    /// which makes reorg ordering explicit and prevents arbitrary historical
    /// subtraction.
    pub fn detach_tip(&mut self, expected_block_id: Hash32) -> Result<BlockMonetaryEffectV1, String> {
        let effect = self
            .canonical_effects
            .last()
            .copied()
            .ok_or_else(|| "cannot detach monetary state from an empty chain".to_string())?;
        if effect.block_id != expected_block_id {
            return Err("monetary rollback must detach the canonical tip".into());
        }

        let gross_issued = self
            .totals
            .gross_issued
            .checked_sub(effect.cpu_subsidy)
            .ok_or_else(|| "gross issuance rollback underflow".to_string())?;
        let protocol_burned = self
            .totals
            .protocol_burned
            .checked_sub(effect.base_fee_burned)
            .ok_or_else(|| "protocol burn rollback underflow".to_string())?;

        self.canonical_effects.pop();
        self.totals = MonetaryTotalsV1 {
            gross_issued,
            protocol_burned,
        };
        Ok(effect)
    }

    /// Rebuild from canonical effects, as a restart/replay reference path.
    /// Stored subsidy values are checked against the deterministic policy.
    pub fn replay(effects: &[BlockMonetaryEffectV1]) -> Result<Self, String> {
        let mut state = Self::default();
        for expected in effects {
            let actual = state.apply_block(
                expected.block_id,
                expected.base_fee_burned,
                expected.priority_fee_to_miner,
            )?;
            if actual.cpu_subsidy != expected.cpu_subsidy {
                return Err("persisted monetary subsidy disagrees with policy".into());
            }
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> Hash32 {
        [byte; 32]
    }

    #[test]
    fn apply_tracks_issuance_burn_and_priority_fee_without_minting_priority_fee() {
        let mut state = MonetaryStateV1::default();
        let effect = state.apply_block(id(1), 100, 50).unwrap();
        assert_eq!(effect.cpu_subsidy, 1_000_000_000);
        assert_eq!(state.totals().gross_issued, 1_000_000_000);
        assert_eq!(state.totals().protocol_burned, 100);
        assert_eq!(state.totals().net_protocol_supply(), Some(999_999_900));
        assert_eq!(effect.priority_fee_to_miner, 50);
    }

    #[test]
    fn duplicate_application_is_rejected() {
        let mut state = MonetaryStateV1::default();
        state.apply_block(id(1), 0, 0).unwrap();
        assert!(state.apply_block(id(1), 0, 0).is_err());
    }

    #[test]
    fn rollback_restores_exact_previous_totals() {
        let mut state = MonetaryStateV1::default();
        state.apply_block(id(1), 10, 3).unwrap();
        let before_second = state.totals();
        state.apply_block(id(2), 20, 7).unwrap();
        state.detach_tip(id(2)).unwrap();
        assert_eq!(state.totals(), before_second);
        assert_eq!(state.canonical_effects().len(), 1);
    }

    #[test]
    fn rollback_must_be_tip_ordered() {
        let mut state = MonetaryStateV1::default();
        state.apply_block(id(1), 0, 0).unwrap();
        state.apply_block(id(2), 0, 0).unwrap();
        assert!(state.detach_tip(id(1)).is_err());
    }

    #[test]
    fn reorg_replaces_detached_effects_once() {
        let mut state = MonetaryStateV1::default();
        state.apply_block(id(1), 10, 1).unwrap();
        state.apply_block(id(2), 20, 2).unwrap();
        state.detach_tip(id(2)).unwrap();
        state.apply_block(id(3), 7, 9).unwrap();
        assert_eq!(state.canonical_effects()[1].block_id, id(3));
        assert_eq!(state.totals().protocol_burned, 17);
    }

    #[test]
    fn replay_reconstructs_restart_state() {
        let mut original = MonetaryStateV1::default();
        original.apply_block(id(1), 10, 1).unwrap();
        original.apply_block(id(2), 20, 2).unwrap();
        original.apply_block(id(3), 30, 3).unwrap();
        let replayed = MonetaryStateV1::replay(original.canonical_effects()).unwrap();
        assert_eq!(replayed, original);
    }

    #[test]
    fn burn_cannot_exceed_issued_supply() {
        let mut state = MonetaryStateV1::default();
        assert!(state.apply_block(id(1), 1_000_000_001, 0).is_err());
        assert_eq!(state, MonetaryStateV1::default());
    }
}
