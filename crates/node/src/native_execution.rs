use crate::work::{keccak256, Hash32};
use std::collections::BTreeMap;

const ACCOUNT_DOMAIN: &[u8] = b"NIAHCIA/ACCOUNT-STATE/V1";
const STATE_DOMAIN: &[u8] = b"NIAHCIA/STATE-ROOT/V1";
const EMPTY_STATE_DOMAIN: &[u8] = b"NIAHCIA/STATE-ROOT/V1/EMPTY";

pub type AccountId = [u8; 20];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccountStateV1 {
    pub balance: u128,
    pub nonce: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeStateV1 {
    accounts: BTreeMap<AccountId, AccountStateV1>,
}

impl NativeStateV1 {
    pub fn account(&self, account: AccountId) -> AccountStateV1 {
        self.accounts.get(&account).copied().unwrap_or_default()
    }

    pub fn set_account(&mut self, account: AccountId, state: AccountStateV1) {
        if state == AccountStateV1::default() {
            self.accounts.remove(&account);
        } else {
            self.accounts.insert(account, state);
        }
    }

    pub fn credit(&mut self, account: AccountId, amount: u128) -> Result<(), String> {
        let mut state = self.account(account);
        state.balance = state
            .balance
            .checked_add(amount)
            .ok_or_else(|| "native account balance overflow".to_string())?;
        self.set_account(account, state);
        Ok(())
    }

    pub fn debit(&mut self, account: AccountId, amount: u128) -> Result<(), String> {
        let mut state = self.account(account);
        state.balance = state
            .balance
            .checked_sub(amount)
            .ok_or_else(|| "insufficient native account balance".to_string())?;
        self.set_account(account, state);
        Ok(())
    }

    pub fn consume_nonce(&mut self, account: AccountId, expected_nonce: u64) -> Result<(), String> {
        let mut state = self.account(account);
        if state.nonce != expected_nonce {
            return Err(format!(
                "native account nonce mismatch: expected {}, found {}",
                state.nonce, expected_nonce
            ));
        }

        state.nonce = state
            .nonce
            .checked_add(1)
            .ok_or_else(|| "native account nonce overflow".to_string())?;
        self.set_account(account, state);
        Ok(())
    }

    pub fn state_root(&self) -> Hash32 {
        if self.accounts.is_empty() {
            return keccak256(EMPTY_STATE_DOMAIN);
        }

        let mut preimage = Vec::with_capacity(STATE_DOMAIN.len() + self.accounts.len() * (20 + 32));
        preimage.extend_from_slice(STATE_DOMAIN);

        for (account, state) in &self.accounts {
            preimage.extend_from_slice(account);
            preimage.extend_from_slice(&account_hash(*account, *state));
        }

        keccak256(&preimage)
    }
}

fn account_hash(account: AccountId, state: AccountStateV1) -> Hash32 {
    let mut preimage = Vec::with_capacity(ACCOUNT_DOMAIN.len() + 20 + 16 + 8);

    preimage.extend_from_slice(ACCOUNT_DOMAIN);
    preimage.extend_from_slice(&account);
    preimage.extend_from_slice(&state.balance.to_be_bytes());
    preimage.extend_from_slice(&state.nonce.to_be_bytes());

    keccak256(&preimage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(marker: u8) -> AccountId {
        [marker; 20]
    }

    #[test]
    fn empty_state_root_is_stable() {
        let state = NativeStateV1::default();
        assert_eq!(
            state.state_root(),
            keccak256(b"NIAHCIA/STATE-ROOT/V1/EMPTY")
        );
    }

    #[test]
    fn credit_and_debit_are_checked() {
        let mut state = NativeStateV1::default();
        let alice = account(1);

        state.credit(alice, 100).unwrap();
        assert_eq!(state.account(alice).balance, 100);

        state.debit(alice, 40).unwrap();
        assert_eq!(state.account(alice).balance, 60);

        assert!(state.debit(alice, 61).is_err());
        assert_eq!(state.account(alice).balance, 60);
    }

    #[test]
    fn nonce_must_match_exactly() {
        let mut state = NativeStateV1::default();
        let alice = account(1);

        state.consume_nonce(alice, 0).unwrap();
        assert_eq!(state.account(alice).nonce, 1);

        assert!(state.consume_nonce(alice, 0).is_err());
        assert_eq!(state.account(alice).nonce, 1);

        state.consume_nonce(alice, 1).unwrap();
        assert_eq!(state.account(alice).nonce, 2);
    }

    #[test]
    fn state_root_is_independent_of_insertion_order() {
        let mut first = NativeStateV1::default();
        first.credit(account(1), 100).unwrap();
        first.credit(account(2), 200).unwrap();

        let mut second = NativeStateV1::default();
        second.credit(account(2), 200).unwrap();
        second.credit(account(1), 100).unwrap();

        assert_eq!(first.state_root(), second.state_root());
    }

    #[test]
    fn state_root_changes_with_balance_or_nonce() {
        let mut state = NativeStateV1::default();
        let initial = state.state_root();

        state.credit(account(1), 1).unwrap();
        let funded = state.state_root();
        assert_ne!(initial, funded);

        state.consume_nonce(account(1), 0).unwrap();
        assert_ne!(funded, state.state_root());
    }

    #[test]
    fn zero_account_state_is_canonical_absence() {
        let mut state = NativeStateV1::default();
        let before = state.state_root();

        state.set_account(account(1), AccountStateV1::default());

        assert_eq!(state.state_root(), before);
    }
}
