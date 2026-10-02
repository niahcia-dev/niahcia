use crate::address::AddressNetwork;
use crate::native_transaction::{
    native_transactions_root_v1, NativeActionV1, SignedNativeTransactionV1,
};
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

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        let count = u64::try_from(self.accounts.len())
            .map_err(|_| "native state account count exceeds u64".to_string())?;

        let mut out = Vec::with_capacity(1 + 8 + self.accounts.len().saturating_mul(44));
        out.push(1);
        out.extend_from_slice(&count.to_be_bytes());

        for (account, state) in &self.accounts {
            out.extend_from_slice(account);
            out.extend_from_slice(&state.balance.to_be_bytes());
            out.extend_from_slice(&state.nonce.to_be_bytes());
        }

        Ok(out)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        const HEADER_LEN: usize = 9;
        const ACCOUNT_LEN: usize = 44;

        if bytes.len() < HEADER_LEN {
            return Err("native state snapshot is truncated".into());
        }

        if bytes[0] != 1 {
            return Err(format!(
                "unsupported native state snapshot version: {}",
                bytes[0]
            ));
        }

        let count = u64::from_be_bytes(bytes[1..9].try_into().unwrap());
        let count = usize::try_from(count)
            .map_err(|_| "native state account count exceeds platform limits".to_string())?;
        let records_len = count
            .checked_mul(ACCOUNT_LEN)
            .ok_or_else(|| "native state snapshot length overflow".to_string())?;
        let expected_len = HEADER_LEN
            .checked_add(records_len)
            .ok_or_else(|| "native state snapshot length overflow".to_string())?;

        if bytes.len() != expected_len {
            return Err(format!(
                "native state snapshot length mismatch: expected {expected_len}, found {}",
                bytes.len()
            ));
        }

        let mut state = Self::default();
        let mut previous: Option<AccountId> = None;

        for index in 0..count {
            let start = HEADER_LEN + index * ACCOUNT_LEN;
            let account: AccountId = bytes[start..start + 20].try_into().unwrap();
            let balance = u128::from_be_bytes(bytes[start + 20..start + 36].try_into().unwrap());
            let nonce = u64::from_be_bytes(bytes[start + 36..start + 44].try_into().unwrap());
            let account_state = AccountStateV1 { balance, nonce };

            if account_state == AccountStateV1::default() {
                return Err("native state snapshot contains a default account".into());
            }

            if let Some(previous) = previous {
                if account <= previous {
                    return Err("native state snapshot accounts are not strictly ordered".into());
                }
            }

            state.accounts.insert(account, account_state);
            previous = Some(account);
        }

        Ok(state)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeExecutionContextV1 {
    pub base_fee_per_gas: u128,
    pub cpu_producer: AccountId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTransferOutcomeV1 {
    pub sender: AccountId,
    pub recipient: AccountId,
    pub value: u128,
    pub gas_used: u64,
    pub max_execution_charge: u128,
    pub priority_fee_per_gas: u128,
    pub effective_fee_per_gas: u128,
    pub base_fee_burned: u128,
    pub producer_priority_fee: u128,
    pub actual_fee: u128,
    pub unused_fee_reserve: u128,
    pub nonce_before: u64,
    pub nonce_after: u64,
}

const RECEIPT_DOMAIN: &[u8] = b"NIAHCIA/NATIVE-RECEIPT/V1";
const RECEIPTS_ROOT_DOMAIN: &[u8] = b"NIAHCIA/NATIVE-RECEIPTS-ROOT/V1";
const EMPTY_RECEIPTS_ROOT_DOMAIN: &[u8] = b"NIAHCIA/NATIVE-RECEIPTS-ROOT/V1/EMPTY";
const EXECUTION_ROOT_DOMAIN: &[u8] = b"NIAHCIA/NATIVE-EXECUTION-ROOT/V1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeReceiptV1 {
    pub transaction_id: Hash32,
    pub gas_used: u64,
    pub effective_fee_per_gas: u128,
    pub base_fee_burned: u128,
    pub producer_priority_fee: u128,
}

impl NativeReceiptV1 {
    pub fn commitment(&self) -> Hash32 {
        let mut preimage = Vec::with_capacity(RECEIPT_DOMAIN.len() + 88);
        preimage.extend_from_slice(RECEIPT_DOMAIN);
        preimage.extend_from_slice(&self.transaction_id);
        preimage.extend_from_slice(&self.gas_used.to_be_bytes());
        preimage.extend_from_slice(&self.effective_fee_per_gas.to_be_bytes());
        preimage.extend_from_slice(&self.base_fee_burned.to_be_bytes());
        preimage.extend_from_slice(&self.producer_priority_fee.to_be_bytes());
        keccak256(&preimage)
    }
}

pub fn native_receipts_root_v1(receipts: &[NativeReceiptV1]) -> Hash32 {
    if receipts.is_empty() {
        return keccak256(EMPTY_RECEIPTS_ROOT_DOMAIN);
    }

    let mut preimage = Vec::with_capacity(RECEIPTS_ROOT_DOMAIN.len() + receipts.len() * 32);
    preimage.extend_from_slice(RECEIPTS_ROOT_DOMAIN);

    for receipt in receipts {
        preimage.extend_from_slice(&receipt.commitment());
    }

    keccak256(&preimage)
}

pub fn native_execution_root_v1(
    transactions_root: Hash32,
    state_root: Hash32,
    receipts_root: Hash32,
    gas_used: u64,
    base_fee_burned: u128,
    producer_priority_fee: u128,
) -> Hash32 {
    let mut preimage = Vec::with_capacity(EXECUTION_ROOT_DOMAIN.len() + 136);
    preimage.extend_from_slice(EXECUTION_ROOT_DOMAIN);
    preimage.extend_from_slice(&transactions_root);
    preimage.extend_from_slice(&state_root);
    preimage.extend_from_slice(&receipts_root);
    preimage.extend_from_slice(&gas_used.to_be_bytes());
    preimage.extend_from_slice(&base_fee_burned.to_be_bytes());
    preimage.extend_from_slice(&producer_priority_fee.to_be_bytes());
    keccak256(&preimage)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeBlockExecutionResultV1 {
    pub transactions_root: Hash32,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub execution_root: Hash32,
    pub gas_used: u64,
    pub base_fee_burned: u128,
    pub producer_priority_fee: u128,
    pub receipts: Vec<NativeReceiptV1>,
}

impl NativeBlockExecutionResultV1 {
    pub fn monetary_effect_input(&self) -> (u128, u128) {
        (self.base_fee_burned, self.producer_priority_fee)
    }
}

pub fn execute_block_v1(
    state: &mut NativeStateV1,
    transactions: &[SignedNativeTransactionV1],
    network: AddressNetwork,
    context: NativeExecutionContextV1,
) -> Result<NativeBlockExecutionResultV1, String> {
    let transactions_root = native_transactions_root_v1(transactions)?;

    // Execute the complete ordered block against a clone. A failure in any
    // transaction or aggregate accounting step leaves canonical state
    // unchanged.
    let mut next = state.clone();
    let mut receipts = Vec::with_capacity(transactions.len());
    let mut gas_used = 0u64;
    let mut base_fee_burned = 0u128;
    let mut producer_priority_fee = 0u128;

    for transaction in transactions {
        let outcome = execute_transfer_v1(&mut next, transaction, network, context)?;

        gas_used = gas_used
            .checked_add(outcome.gas_used)
            .ok_or_else(|| "native block gas_used overflow".to_string())?;

        base_fee_burned = base_fee_burned
            .checked_add(outcome.base_fee_burned)
            .ok_or_else(|| "native block base fee burn overflow".to_string())?;

        producer_priority_fee = producer_priority_fee
            .checked_add(outcome.producer_priority_fee)
            .ok_or_else(|| "native block producer priority fee overflow".to_string())?;

        receipts.push(NativeReceiptV1 {
            transaction_id: transaction.tx_id()?,
            gas_used: outcome.gas_used,
            effective_fee_per_gas: outcome.effective_fee_per_gas,
            base_fee_burned: outcome.base_fee_burned,
            producer_priority_fee: outcome.producer_priority_fee,
        });
    }

    let state_root = next.state_root();
    let receipts_root = native_receipts_root_v1(&receipts);

    let execution_root = native_execution_root_v1(
        transactions_root,
        state_root,
        receipts_root,
        gas_used,
        base_fee_burned,
        producer_priority_fee,
    );

    let result = NativeBlockExecutionResultV1 {
        transactions_root,
        state_root,
        receipts_root,
        execution_root,
        gas_used,
        base_fee_burned,
        producer_priority_fee,
        receipts,
    };

    *state = next;

    Ok(result)
}

pub fn execute_transfer_v1(
    state: &mut NativeStateV1,
    transaction: &SignedNativeTransactionV1,
    network: AddressNetwork,
    context: NativeExecutionContextV1,
) -> Result<NativeTransferOutcomeV1, String> {
    use crate::native_transaction::NATIVE_TRANSFER_GAS_V1;

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

    let gas_used = NATIVE_TRANSFER_GAS_V1;

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

    if transaction.body.max_fee_per_gas < context.base_fee_per_gas {
        return Err(format!(
            "native transaction max_fee_per_gas {} is below base_fee_per_gas {}",
            transaction.body.max_fee_per_gas, context.base_fee_per_gas
        ));
    }

    let priority_headroom = transaction
        .body
        .max_fee_per_gas
        .checked_sub(context.base_fee_per_gas)
        .ok_or_else(|| "native transaction priority fee headroom underflow".to_string())?;

    let priority_fee_per_gas = transaction
        .body
        .max_priority_fee_per_gas
        .min(priority_headroom);

    let effective_fee_per_gas = context
        .base_fee_per_gas
        .checked_add(priority_fee_per_gas)
        .ok_or_else(|| "native transaction effective fee overflow".to_string())?;

    let base_fee_burned = context
        .base_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| "native transaction base fee burn overflow".to_string())?;

    let producer_priority_fee = priority_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| "native transaction producer priority fee overflow".to_string())?;

    let actual_fee = base_fee_burned
        .checked_add(producer_priority_fee)
        .ok_or_else(|| "native transaction actual fee overflow".to_string())?;

    let unused_fee_reserve = max_execution_charge
        .checked_sub(actual_fee)
        .ok_or_else(|| "native transaction actual fee exceeds maximum reserve".to_string())?;

    let sender_actual_debit = transaction
        .body
        .value
        .checked_add(actual_fee)
        .ok_or_else(|| "native transaction sender debit overflow".to_string())?;

    // Apply the complete transition against a clone. Any arithmetic,
    // balance, nonce, recipient, or producer-credit failure leaves canonical
    // state unchanged.
    let mut next = state.clone();

    next.consume_nonce(sender, transaction.body.nonce)?;
    next.debit(sender, sender_actual_debit)?;
    next.credit(recipient, transaction.body.value)?;
    next.credit(context.cpu_producer, producer_priority_fee)?;

    let nonce_after = next.account(sender).nonce;

    *state = next;

    Ok(NativeTransferOutcomeV1 {
        sender,
        recipient,
        value: transaction.body.value,
        gas_used,
        max_execution_charge,
        priority_fee_per_gas,
        effective_fee_per_gas,
        base_fee_burned,
        producer_priority_fee,
        actual_fee,
        unused_fee_reserve,
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

    #[test]
    fn native_state_snapshot_round_trip_preserves_state_and_root() {
        let mut state = NativeStateV1::default();
        state.set_account(
            [0x22; 20],
            AccountStateV1 {
                balance: 9_876_543_210,
                nonce: 7,
            },
        );
        state.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 123_456_789,
                nonce: 3,
            },
        );

        let encoded = state.canonical_bytes().unwrap();
        let decoded = NativeStateV1::from_canonical_bytes(&encoded).unwrap();

        assert_eq!(decoded, state);
        assert_eq!(decoded.state_root(), state.state_root());
        assert_eq!(encoded[0], 1);
        assert_eq!(u64::from_be_bytes(encoded[1..9].try_into().unwrap()), 2);
        assert_eq!(&encoded[9..29], &[0x11; 20]);
        assert_eq!(&encoded[53..73], &[0x22; 20]);
    }

    #[test]
    fn empty_native_state_snapshot_round_trips() {
        let state = NativeStateV1::default();
        let encoded = state.canonical_bytes().unwrap();

        assert_eq!(encoded, [vec![1], 0_u64.to_be_bytes().to_vec()].concat());
        assert_eq!(
            NativeStateV1::from_canonical_bytes(&encoded).unwrap(),
            state
        );
    }

    #[test]
    fn native_state_snapshot_rejects_wrong_version_and_length() {
        let err = NativeStateV1::from_canonical_bytes(&[]).unwrap_err();
        assert!(err.contains("truncated"));

        let mut wrong_version = vec![2];
        wrong_version.extend_from_slice(&0_u64.to_be_bytes());
        let err = NativeStateV1::from_canonical_bytes(&wrong_version).unwrap_err();
        assert!(err.contains("unsupported"));

        let mut wrong_length = vec![1];
        wrong_length.extend_from_slice(&1_u64.to_be_bytes());
        let err = NativeStateV1::from_canonical_bytes(&wrong_length).unwrap_err();
        assert!(err.contains("length mismatch"));
    }

    #[test]
    fn native_state_snapshot_rejects_default_and_unordered_accounts() {
        let mut default_account = vec![1];
        default_account.extend_from_slice(&1_u64.to_be_bytes());
        default_account.extend_from_slice(&[0x11; 20]);
        default_account.extend_from_slice(&0_u128.to_be_bytes());
        default_account.extend_from_slice(&0_u64.to_be_bytes());

        let err = NativeStateV1::from_canonical_bytes(&default_account).unwrap_err();
        assert!(err.contains("default account"));

        let mut unordered = vec![1];
        unordered.extend_from_slice(&2_u64.to_be_bytes());

        for account in [[0x22; 20], [0x11; 20]] {
            unordered.extend_from_slice(&account);
            unordered.extend_from_slice(&1_u128.to_be_bytes());
            unordered.extend_from_slice(&0_u64.to_be_bytes());
        }

        let err = NativeStateV1::from_canonical_bytes(&unordered).unwrap_err();
        assert!(err.contains("strictly ordered"));
    }

    use crate::native_transaction::{
        NativeTransactionBodyV1, DEVNET_CHAIN_ID, DEVNET_NETWORK_ID, NATIVE_TRANSFER_GAS_V1,
    };
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
                max_priority_fee_per_gas: 0,
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

    fn signed_fee_transfer_to(
        recipient: AccountId,
        nonce: u64,
        value: u128,
        gas_limit: u64,
        max_fee_per_gas: u128,
        max_priority_fee_per_gas: u128,
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
                max_priority_fee_per_gas,
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

    fn zero_fee_context() -> NativeExecutionContextV1 {
        NativeExecutionContextV1 {
            base_fee_per_gas: 0,
            cpu_producer: account(9),
        }
    }

    #[test]
    fn transfer_execution_moves_value_and_consumes_nonce() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 3_000).unwrap();

        let outcome =
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, zero_fee_context())
                .unwrap();

        assert_eq!(state.account(sender).balance, 2_900);
        assert_eq!(state.account(sender).nonce, 1);
        assert_eq!(state.account(recipient).balance, 100);

        assert_eq!(outcome.sender, sender);
        assert_eq!(outcome.recipient, recipient);
        assert_eq!(outcome.value, 100);
        assert_eq!(outcome.max_execution_charge, 2_000);
        assert_eq!(outcome.nonce_before, 0);
        assert_eq!(outcome.nonce_after, 1);
    }

    #[test]
    fn transfer_fee_accounting_is_exact() {
        let recipient = account(2);
        let producer = account(9);

        let tx = signed_fee_transfer_to(recipient, 0, 100, 2_000, 10, 4);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 25_000).unwrap();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: producer,
        };

        let outcome =
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, context).unwrap();

        assert_eq!(outcome.sender, sender);
        assert_eq!(outcome.recipient, recipient);
        assert_eq!(outcome.value, 100);

        assert_eq!(outcome.gas_used, 1_000);
        assert_eq!(outcome.max_execution_charge, 20_000);
        assert_eq!(outcome.priority_fee_per_gas, 4);
        assert_eq!(outcome.effective_fee_per_gas, 7);
        assert_eq!(outcome.base_fee_burned, 3_000);
        assert_eq!(outcome.producer_priority_fee, 4_000);
        assert_eq!(outcome.actual_fee, 7_000);
        assert_eq!(outcome.unused_fee_reserve, 13_000);

        assert_eq!(outcome.nonce_before, 0);
        assert_eq!(outcome.nonce_after, 1);

        assert_eq!(state.account(sender).balance, 17_900);
        assert_eq!(state.account(sender).nonce, 1);
        assert_eq!(state.account(recipient).balance, 100);
        assert_eq!(state.account(producer).balance, 4_000);
    }

    #[test]
    fn priority_fee_is_capped_by_max_fee_headroom() {
        let recipient = account(2);
        let producer = account(9);

        // max fee 10, base fee 7 leaves only 3 per gas of priority
        // headroom even though the sender authorizes a priority fee of 8.
        let tx = signed_fee_transfer_to(recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 10, 8);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 20_000).unwrap();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 7,
            cpu_producer: producer,
        };

        let outcome =
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, context).unwrap();

        assert_eq!(outcome.gas_used, 1_000);
        assert_eq!(outcome.priority_fee_per_gas, 3);
        assert_eq!(outcome.effective_fee_per_gas, 10);
        assert_eq!(outcome.base_fee_burned, 7_000);
        assert_eq!(outcome.producer_priority_fee, 3_000);
        assert_eq!(outcome.actual_fee, 10_000);
        assert_eq!(outcome.unused_fee_reserve, 0);

        assert_eq!(state.account(sender).balance, 9_900);
        assert_eq!(state.account(recipient).balance, 100);
        assert_eq!(state.account(producer).balance, 3_000);
    }

    #[test]
    fn max_fee_below_base_fee_does_not_mutate_state() {
        let recipient = account(2);
        let producer = account(9);

        let tx = signed_fee_transfer_to(recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 6, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 20_000).unwrap();
        let before = state.clone();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 7,
            cpu_producer: producer,
        };

        assert!(execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, context).is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn producer_may_be_sender_without_minting_priority_fee() {
        let recipient = account(2);

        let seed = signed_fee_transfer_to(recipient, 0, 0, NATIVE_TRANSFER_GAS_V1, 10, 4);
        let sender = transaction_sender(&seed);

        let tx = signed_fee_transfer_to(recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 10, 4);

        let mut state = NativeStateV1::default();
        state.credit(sender, 20_000).unwrap();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: sender,
        };

        let outcome =
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, context).unwrap();

        assert_eq!(outcome.base_fee_burned, 3_000);
        assert_eq!(outcome.producer_priority_fee, 4_000);
        assert_eq!(outcome.actual_fee, 7_000);

        // Sender pays value + full fee, then receives its own priority fee:
        // 20,000 - 100 - 7,000 + 4,000 = 16,900.
        assert_eq!(state.account(sender).balance, 16_900);
        assert_eq!(state.account(sender).nonce, 1);
        assert_eq!(state.account(recipient).balance, 100);
    }

    #[test]
    fn producer_may_be_recipient_without_double_crediting() {
        let recipient_and_producer = account(2);

        let tx = signed_fee_transfer_to(
            recipient_and_producer,
            0,
            100,
            NATIVE_TRANSFER_GAS_V1,
            10,
            4,
        );
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 20_000).unwrap();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: recipient_and_producer,
        };

        let outcome =
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, context).unwrap();

        assert_eq!(outcome.base_fee_burned, 3_000);
        assert_eq!(outcome.producer_priority_fee, 4_000);
        assert_eq!(outcome.actual_fee, 7_000);

        assert_eq!(state.account(sender).balance, 12_900);
        assert_eq!(state.account(sender).nonce, 1);

        // One value credit plus one priority-fee credit.
        assert_eq!(state.account(recipient_and_producer).balance, 4_100);
    }

    #[test]
    fn producer_credit_overflow_does_not_mutate_state() {
        let recipient = account(2);
        let producer = account(9);

        let tx = signed_fee_transfer_to(recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 10, 1);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 20_000).unwrap();
        state.credit(producer, u128::MAX).unwrap();
        let before = state.clone();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: producer,
        };

        assert!(execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, context).is_err());

        assert_eq!(state, before);
        assert_eq!(state.account(sender).nonce, 0);
        assert_eq!(state.account(sender).balance, 20_000);
        assert_eq!(state.account(recipient).balance, 0);
        assert_eq!(state.account(producer).balance, u128::MAX);
    }

    #[test]
    fn insufficient_maximum_reserve_does_not_mutate_state() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 119).unwrap();
        let before = state.clone();

        assert!(
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, zero_fee_context())
                .is_err()
        );
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

        assert!(
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, zero_fee_context())
                .is_err()
        );
        assert_eq!(state, before);
    }

    #[test]
    fn required_balance_overflow_does_not_mutate_state() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 0, u128::MAX, NATIVE_TRANSFER_GAS_V1, 1);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, u128::MAX).unwrap();
        let before = state.clone();

        assert!(
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, zero_fee_context())
                .is_err()
        );
        assert_eq!(state, before);
    }

    #[test]
    fn wrong_nonce_does_not_mutate_state() {
        let recipient = account(2);
        let tx = signed_transfer_to(recipient, 1, 100, NATIVE_TRANSFER_GAS_V1, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 1_000).unwrap();
        let before = state.clone();

        assert!(
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, zero_fee_context())
                .is_err()
        );
        assert_eq!(state, before);
    }

    #[test]
    fn self_transfer_only_consumes_nonce() {
        let seed = signed_transfer_to(account(2), 0, 0, NATIVE_TRANSFER_GAS_V1, 2);
        let sender = transaction_sender(&seed);
        let tx = signed_transfer_to(sender, 0, 100, NATIVE_TRANSFER_GAS_V1, 2);

        let mut state = NativeStateV1::default();
        state.credit(sender, 3_000).unwrap();

        execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, zero_fee_context()).unwrap();

        assert_eq!(state.account(sender).balance, 3_000);
        assert_eq!(state.account(sender).nonce, 1);
    }

    #[test]
    fn invalid_signature_does_not_mutate_state() {
        let recipient = account(2);
        let mut tx = signed_transfer_to(recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 2);
        let sender = transaction_sender(&tx);

        let mut state = NativeStateV1::default();
        state.credit(sender, 1_000).unwrap();
        let before = state.clone();

        tx.signature[0] ^= 1;

        assert!(
            execute_transfer_v1(&mut state, &tx, AddressNetwork::Devnet, zero_fee_context())
                .is_err()
        );
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
    fn empty_block_execution_is_deterministic() {
        let mut state = NativeStateV1::default();
        let before = state.clone();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: account(9),
        };

        let result = execute_block_v1(&mut state, &[], AddressNetwork::Devnet, context).unwrap();

        assert_eq!(state, before);
        assert_eq!(
            result.transactions_root,
            native_transactions_root_v1(&[]).unwrap()
        );
        assert_eq!(result.state_root, state.state_root());
        assert_eq!(result.receipts_root, native_receipts_root_v1(&[]));
        assert_eq!(result.gas_used, 0);
        assert_eq!(result.base_fee_burned, 0);
        assert_eq!(result.producer_priority_fee, 0);
        assert!(result.receipts.is_empty());

        assert_eq!(
            result.execution_root,
            native_execution_root_v1(
                result.transactions_root,
                result.state_root,
                result.receipts_root,
                0,
                0,
                0,
            )
        );
    }

    #[test]
    fn block_execution_applies_transactions_in_order() {
        let first_recipient = account(2);
        let second_recipient = account(3);
        let producer = account(9);

        let first = signed_fee_transfer_to(first_recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 10, 4);
        let sender = transaction_sender(&first);

        let second =
            signed_fee_transfer_to(second_recipient, 1, 200, NATIVE_TRANSFER_GAS_V1, 10, 4);

        let transactions = vec![first, second];

        let mut state = NativeStateV1::default();
        state.credit(sender, 30_000).unwrap();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: producer,
        };

        let result =
            execute_block_v1(&mut state, &transactions, AddressNetwork::Devnet, context).unwrap();

        assert_eq!(
            result.transactions_root,
            native_transactions_root_v1(&transactions).unwrap()
        );
        assert_eq!(result.state_root, state.state_root());
        assert_eq!(
            result.receipts_root,
            native_receipts_root_v1(&result.receipts)
        );
        assert_eq!(result.gas_used, 2_000);
        assert_eq!(result.base_fee_burned, 6_000);
        assert_eq!(result.producer_priority_fee, 8_000);
        assert_eq!(result.receipts.len(), 2);

        assert_eq!(state.account(sender).balance, 15_700);
        assert_eq!(state.account(sender).nonce, 2);
        assert_eq!(state.account(first_recipient).balance, 100);
        assert_eq!(state.account(second_recipient).balance, 200);
        assert_eq!(state.account(producer).balance, 8_000);

        assert_eq!(
            result.execution_root,
            native_execution_root_v1(
                result.transactions_root,
                result.state_root,
                result.receipts_root,
                result.gas_used,
                result.base_fee_burned,
                result.producer_priority_fee,
            )
        );
    }

    #[test]
    fn failed_transaction_rolls_back_entire_block() {
        let first_recipient = account(2);
        let second_recipient = account(3);
        let producer = account(9);

        let first = signed_fee_transfer_to(first_recipient, 0, 100, NATIVE_TRANSFER_GAS_V1, 10, 4);
        let sender = transaction_sender(&first);

        // Wrong nonce. The first transaction is valid, but the second must
        // invalidate the complete block transition.
        let second =
            signed_fee_transfer_to(second_recipient, 7, 200, NATIVE_TRANSFER_GAS_V1, 10, 4);

        let transactions = vec![first, second];

        let mut state = NativeStateV1::default();
        state.credit(sender, 30_000).unwrap();
        let before = state.clone();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: producer,
        };

        assert!(
            execute_block_v1(&mut state, &transactions, AddressNetwork::Devnet, context,).is_err()
        );

        assert_eq!(state, before);
        assert_eq!(state.account(sender).balance, 30_000);
        assert_eq!(state.account(sender).nonce, 0);
        assert_eq!(state.account(first_recipient).balance, 0);
        assert_eq!(state.account(second_recipient).balance, 0);
        assert_eq!(state.account(producer).balance, 0);
    }

    #[test]
    fn native_execution_v1_interoperability_vector() {
        let first = signed_fee_transfer_to(account(2), 0, 100, NATIVE_TRANSFER_GAS_V1, 10, 4);
        let sender = transaction_sender(&first);

        let second = signed_fee_transfer_to(account(3), 1, 200, NATIVE_TRANSFER_GAS_V1, 10, 4);

        let transactions = vec![first, second];

        let mut state = NativeStateV1::default();
        state.credit(sender, 30_000).unwrap();

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: account(9),
        };

        let result =
            execute_block_v1(&mut state, &transactions, AddressNetwork::Devnet, context).unwrap();

        assert_eq!(
            hex::encode(sender),
            "1a642f0e3c3af545e7acbd38b07251b3990914f1"
        );
        assert_eq!(
            hex::encode(result.receipts[0].transaction_id),
            "12824fa00e58c1e26bbd294669016b563a5ad033459324da9a6a521c5597d38c"
        );
        assert_eq!(
            hex::encode(result.receipts[1].transaction_id),
            "e491ae3db2b72ac0e9bcf2cdb1544ef0625dc6a5ef771fe45852e9d490eec305"
        );
        assert_eq!(
            hex::encode(result.receipts[0].commitment()),
            "6ede591e53c51bae6f306997e24a4f6a7343834d8de3734edb98c5dfe40b310e"
        );
        assert_eq!(
            hex::encode(result.receipts[1].commitment()),
            "58993e4a0fee536b051d032ec31b8e5c1ed11e8630a9615aeab3fcdc6e6ad483"
        );
        assert_eq!(
            hex::encode(result.transactions_root),
            "601dcd68243e382ab2697cb3732c307e232875549714f1a69a033f81c095e0ef"
        );
        assert_eq!(
            hex::encode(result.state_root),
            "dabce914c479e1996769e4741b7febf2390cb4d1d7b9502d9fea6e87beda5e2f"
        );
        assert_eq!(
            hex::encode(result.receipts_root),
            "9ea9d54bed0acf2a7c0a18a44b043f1e169163b3ba6d962d58e8cbbe10af1fea"
        );
        assert_eq!(
            hex::encode(result.execution_root),
            "9dad2c600bd3d554a69143c444eac49313956a3e8005d25dc381ab5f7eabb2a7"
        );
        assert_eq!(result.gas_used, 2_000);
        assert_eq!(result.base_fee_burned, 6_000);
        assert_eq!(result.producer_priority_fee, 8_000);
    }

    #[test]
    fn execution_root_commits_to_every_field() {
        let transactions_root = [0x11; 32];
        let state_root = [0x22; 32];
        let receipts_root = [0x33; 32];
        let gas_used = 2_000;
        let base_fee_burned = 6_000;
        let producer_priority_fee = 8_000;

        let expected = native_execution_root_v1(
            transactions_root,
            state_root,
            receipts_root,
            gas_used,
            base_fee_burned,
            producer_priority_fee,
        );

        let mut changed_transactions_root = transactions_root;
        changed_transactions_root[0] ^= 1;
        assert_ne!(
            native_execution_root_v1(
                changed_transactions_root,
                state_root,
                receipts_root,
                gas_used,
                base_fee_burned,
                producer_priority_fee,
            ),
            expected
        );

        let mut changed_state_root = state_root;
        changed_state_root[0] ^= 1;
        assert_ne!(
            native_execution_root_v1(
                transactions_root,
                changed_state_root,
                receipts_root,
                gas_used,
                base_fee_burned,
                producer_priority_fee,
            ),
            expected
        );

        let mut changed_receipts_root = receipts_root;
        changed_receipts_root[0] ^= 1;
        assert_ne!(
            native_execution_root_v1(
                transactions_root,
                state_root,
                changed_receipts_root,
                gas_used,
                base_fee_burned,
                producer_priority_fee,
            ),
            expected
        );

        assert_ne!(
            native_execution_root_v1(
                transactions_root,
                state_root,
                receipts_root,
                gas_used + 1,
                base_fee_burned,
                producer_priority_fee,
            ),
            expected
        );

        assert_ne!(
            native_execution_root_v1(
                transactions_root,
                state_root,
                receipts_root,
                gas_used,
                base_fee_burned + 1,
                producer_priority_fee,
            ),
            expected
        );

        assert_ne!(
            native_execution_root_v1(
                transactions_root,
                state_root,
                receipts_root,
                gas_used,
                base_fee_burned,
                producer_priority_fee + 1,
            ),
            expected
        );
    }

    #[test]
    fn empty_receipts_root_is_stable() {
        assert_eq!(
            native_receipts_root_v1(&[]),
            keccak256(EMPTY_RECEIPTS_ROOT_DOMAIN)
        );
    }

    #[test]
    fn receipt_commitment_changes_with_every_field() {
        let receipt = NativeReceiptV1 {
            transaction_id: [0x11; 32],
            gas_used: 1_000,
            effective_fee_per_gas: 7,
            base_fee_burned: 3_000,
            producer_priority_fee: 4_000,
        };

        let expected = receipt.commitment();

        let mut changed = receipt;
        changed.transaction_id[0] ^= 1;
        assert_ne!(changed.commitment(), expected);

        let mut changed = receipt;
        changed.gas_used += 1;
        assert_ne!(changed.commitment(), expected);

        let mut changed = receipt;
        changed.effective_fee_per_gas += 1;
        assert_ne!(changed.commitment(), expected);

        let mut changed = receipt;
        changed.base_fee_burned += 1;
        assert_ne!(changed.commitment(), expected);

        let mut changed = receipt;
        changed.producer_priority_fee += 1;
        assert_ne!(changed.commitment(), expected);
    }

    #[test]
    fn receipts_root_is_order_sensitive() {
        let first = NativeReceiptV1 {
            transaction_id: [0x11; 32],
            gas_used: 1_000,
            effective_fee_per_gas: 7,
            base_fee_burned: 3_000,
            producer_priority_fee: 4_000,
        };

        let second = NativeReceiptV1 {
            transaction_id: [0x22; 32],
            gas_used: 1_000,
            effective_fee_per_gas: 5,
            base_fee_burned: 3_000,
            producer_priority_fee: 2_000,
        };

        assert_ne!(
            native_receipts_root_v1(&[first, second]),
            native_receipts_root_v1(&[second, first])
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
