use crate::address::AddressNetwork;
use crate::native_transaction::{NativeActionV1, SignedNativeTransactionV1};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTransferOutcomeV1 {
    pub sender: AccountId,
    pub recipient: AccountId,
    pub value: u128,
    pub max_execution_charge: u128,
    pub nonce_before: u64,
    pub nonce_after: u64,
}

pub fn execute_transfer_v1(
    state: &mut NativeStateV1,
    transaction: &SignedNativeTransactionV1,
    network: AddressNetwork,
) -> Result<NativeTransferOutcomeV1, String> {
    transaction.body.validate(network)?;

    if transaction.body.action != NativeActionV1::Transfer {
        return Err("native transfer executor requires Transfer action".to_string());
    }

    let sender_address = transaction.authenticated_sender(network)?;
    let sender = sender_address.payload;

    let recipient: AccountId = transaction
        .body
        .target_payload
        .as_slice()
        .try_into()
        .map_err(|_| "native transfer target must be exactly 20 bytes".to_string())?;

    let sender_before = state.account(sender);

    if sender_before.nonce != transaction.body.nonce {
        return Err(format!(
            "native account nonce mismatch: expected {}, found {}",
            sender_before.nonce, transaction.body.nonce
        ));
    }

    let max_execution_charge = transaction
        .body
        .max_fee_per_gas
        .checked_mul(transaction.body.gas_limit as u128)
        .ok_or_else(|| "native transaction maximum execution charge overflow".to_string())?;

    let required_balance = transaction
        .body
        .value
        .checked_add(max_execution_charge)
        .ok_or_else(|| "native transaction required balance overflow".to_string())?;

    if sender_before.balance < required_balance {
        return Err(format!(
            "insufficient native account balance: required {}, available {}",
            required_balance, sender_before.balance
        ));
    }

    // Apply against a clone so every failure before commit leaves canonical
    // state completely unchanged.
    let mut next = state.clone();

    next.consume_nonce(sender, transaction.body.nonce)?;

    if sender != recipient {
        next.debit(sender, transaction.body.value)?;
        next.credit(recipient, transaction.body.value)?;
    }

    let nonce_after = next.account(sender).nonce;

    *state = next;

    Ok(NativeTransferOutcomeV1 {
        sender,
        recipient,
        value: transaction.body.value,
        max_execution_charge,
        nonce_before: sender_before.nonce,
        nonce_after,
    })
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
    use crate::native_transaction::{NativeTransactionBodyV1, DEVNET_CHAIN_ID, DEVNET_NETWORK_ID};
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn account(marker: u8) -> AccountId {
        [marker; 20]
    }

    fn signed_transfer_to(
        recipient: AccountId,
        nonce: u64,
        value: u128,
        gas_limit: u64,
        max_fee_per_gas: u128,
    ) -> SignedNativeTransactionV1 {
        let signing_key = SigningKey::from_slice(&[0x01; 32]).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

        let mut tx = SignedNativeTransactionV1 {
            body: NativeTransactionBodyV1 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce,
                action: NativeActionV1::Transfer,
                target_payload: recipient.to_vec(),
                value,
                gas_limit,
                max_fee_per_gas,
                data: Vec::new(),
            },
            public_key,
            signature: vec![0; 64],
        };

        let digest = tx.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        tx.signature = signature.to_bytes().to_vec();
        tx
    }

    fn transaction_sender(tx: &SignedNativeTransactionV1) -> AccountId {
        tx.authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload
    }

    #[test]
    fn transfer_execution_moves_value_and_consumes_nonce() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 0, 100, 10, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 1_000).unwrap();

        let outcome = execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet).unwrap();

        assert_eq!(state.account(sender).balance, 900);
        assert_eq!(state.account(sender).nonce, 1);
        assert_eq!(state.account(recipient).balance, 100);

        assert_eq!(outcome.sender, sender);
        assert_eq!(outcome.recipient, recipient);
        assert_eq!(outcome.value, 100);
        assert_eq!(outcome.max_execution_charge, 20);
        assert_eq!(outcome.nonce_before, 0);
        assert_eq!(outcome.nonce_after, 1);
    }

    #[test]
    fn insufficient_maximum_reserve_does_not_mutate_state() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 0, 100, 10, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 119).unwrap();
        let before = state.clone();

        assert!(execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet).is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn maximum_execution_charge_overflow_does_not_mutate_state() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 0, 1, u64::MAX, u128::MAX);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, u128::MAX).unwrap();
        let before = state.clone();

        assert!(execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet).is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn required_balance_overflow_does_not_mutate_state() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 0, u128::MAX, 1, 1);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, u128::MAX).unwrap();
        let before = state.clone();

        assert!(execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet).is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn wrong_nonce_does_not_mutate_state() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 1, 100, 10, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 1_000).unwrap();
        let before = state.clone();

        assert!(execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet).is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn self_transfer_only_consumes_nonce() {
        let seed = signed_transfer_to(account(2), 0, 0, 10, 2);
        let sender = transaction_sender(&seed);
        let tx = signed_transfer_to(sender, 0, 100, 10, 2);

        let mut state = NativeStateV1::default();
        state.credit(sender, 1_000).unwrap();

        execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet).unwrap();

        assert_eq!(state.account(sender).balance, 1_000);
        assert_eq!(state.account(sender).nonce, 1);
    }

    #[test]
    fn invalid_signature_does_not_mutate_state() {
        let recipient = account(2);
        let mut tx = signed_transfer_to(recipient, 0, 100, 10, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 1_000).unwrap();
        let before = state.clone();

        tx.signature[0] ^= 1;

        assert!(execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet).is_err());
        assert_eq!(state, before);
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
