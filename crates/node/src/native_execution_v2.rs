use crate::address::AddressNetwork;
use crate::native_execution::{AccountId, NativeExecutionContextV1};
use crate::native_state_v2::NativeStateV2;
use crate::native_transaction::NATIVE_TRANSFER_GAS_V1;
use crate::native_transaction_v2::{NativeActionV2, SignedNativeTransactionV2};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTransferOutcomeV2 {
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

/// Executes the schema-V2 Transfer action against NativeStateV2 while preserving
/// the already-defined NativeTransaction V1 transfer semantics.
///
/// This helper is deliberately not connected to the active mempool/P2P/mining
/// path. It exists so the successor V2 block/execution boundary can preserve
/// ordinary native transfers after NativeStateV2 activation without
/// reinterpreting ContractCall/ContractCreate or assigning compute gas values.
pub fn execute_transfer_v2(
    state: &mut NativeStateV2,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    context: NativeExecutionContextV1,
) -> Result<NativeTransferOutcomeV2, String> {
    transaction.body.validate(network)?;

    if transaction.body.action != NativeActionV2::Transfer {
        return Err("native transfer V2 executor requires Transfer action".to_string());
    }

    let sender = transaction.authenticated_sender(network)?.payload;
    let recipient: AccountId = transaction
        .body
        .target_payload
        .as_slice()
        .try_into()
        .map_err(|_| "native transfer V2 target must be exactly 20 bytes".to_string())?;

    let sender_before = state.accounts().account(sender);
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
        .ok_or_else(|| "native transaction V2 maximum execution charge overflow".to_string())?;

    let required_balance = transaction
        .body
        .value
        .checked_add(max_execution_charge)
        .ok_or_else(|| "native transaction V2 required balance overflow".to_string())?;

    if sender_before.balance < required_balance {
        return Err(format!(
            "insufficient native account balance: required {}, available {}",
            required_balance, sender_before.balance
        ));
    }

    if transaction.body.max_fee_per_gas < context.base_fee_per_gas {
        return Err(format!(
            "native transaction V2 max_fee_per_gas {} is below base_fee_per_gas {}",
            transaction.body.max_fee_per_gas, context.base_fee_per_gas
        ));
    }

    let priority_headroom = transaction
        .body
        .max_fee_per_gas
        .checked_sub(context.base_fee_per_gas)
        .ok_or_else(|| "native transaction V2 priority fee headroom underflow".to_string())?;

    let priority_fee_per_gas = transaction
        .body
        .max_priority_fee_per_gas
        .min(priority_headroom);

    let effective_fee_per_gas = context
        .base_fee_per_gas
        .checked_add(priority_fee_per_gas)
        .ok_or_else(|| "native transaction V2 effective fee overflow".to_string())?;

    let base_fee_burned = context
        .base_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| "native transaction V2 base fee burn overflow".to_string())?;

    let producer_priority_fee = priority_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| "native transaction V2 producer priority fee overflow".to_string())?;

    let actual_fee = base_fee_burned
        .checked_add(producer_priority_fee)
        .ok_or_else(|| "native transaction V2 actual fee overflow".to_string())?;

    let unused_fee_reserve = max_execution_charge
        .checked_sub(actual_fee)
        .ok_or_else(|| "native transaction V2 actual fee exceeds maximum reserve".to_string())?;

    let sender_actual_debit = transaction
        .body
        .value
        .checked_add(actual_fee)
        .ok_or_else(|| "native transaction V2 sender debit overflow".to_string())?;

    let mut next = state.clone();
    next.accounts_mut()
        .consume_nonce(sender, transaction.body.nonce)?;
    next.accounts_mut().debit(sender, sender_actual_debit)?;
    next.accounts_mut()
        .credit(recipient, transaction.body.value)?;
    next.accounts_mut()
        .credit(context.cpu_producer, producer_priority_fee)?;

    let nonce_after = next.accounts().account(sender).nonce;
    *state = next;

    Ok(NativeTransferOutcomeV2 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_execution::{execute_transfer_v1, AccountStateV1, NativeStateV1};
    use crate::native_transaction::{
        NativeActionV1, NativeTransactionBodyV1, SignedNativeTransactionV1, DEVNET_CHAIN_ID,
        DEVNET_NETWORK_ID,
    };
    use crate::native_transaction_v2::{NativeTransactionBodyV2, SignedNativeTransactionV2};
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn sign_v1(key: &SigningKey, body: NativeTransactionBodyV1) -> SignedNativeTransactionV1 {
        let mut transaction = SignedNativeTransactionV1 {
            body,
            public_key: key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            signature: vec![0; 64],
        };
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        transaction
    }

    fn sign_v2(key: &SigningKey, body: NativeTransactionBodyV2) -> SignedNativeTransactionV2 {
        let mut transaction = SignedNativeTransactionV2 {
            body,
            public_key: key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            signature: vec![0; 64],
        };
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        transaction
    }

    fn equivalent_transfers(
        key: &SigningKey,
        nonce: u64,
    ) -> (SignedNativeTransactionV1, SignedNativeTransactionV2) {
        let target = vec![0x44; 20];
        let value = 500;
        let gas_limit = NATIVE_TRANSFER_GAS_V1;
        let max_fee_per_gas = 10;
        let max_priority_fee_per_gas = 4;

        let v1 = sign_v1(
            key,
            NativeTransactionBodyV1 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce,
                action: NativeActionV1::Transfer,
                target_payload: target.clone(),
                value,
                gas_limit,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                data: Vec::new(),
            },
        );

        let v2 = sign_v2(
            key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce,
                action: NativeActionV2::Transfer,
                target_payload: target,
                value,
                gas_limit,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                data: Vec::new(),
            },
        );

        (v1, v2)
    }

    #[test]
    fn v2_transfer_preserves_v1_account_semantics_exactly() {
        let key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let sender = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x44; 20],
                value: 1,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 1,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        )
        .authenticated_sender(AddressNetwork::Devnet)
        .unwrap()
        .payload;

        let mut v1_state = NativeStateV1::default();
        v1_state.set_account(
            sender,
            AccountStateV1 {
                balance: 100_000,
                nonce: 0,
            },
        );
        let mut v2_state = NativeStateV2::from_v1(v1_state.clone());

        let (v1_transaction, v2_transaction) = equivalent_transfers(&key, 0);
        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: [0x99; 20],
        };

        let v1_outcome = execute_transfer_v1(
            &mut v1_state,
            &v1_transaction,
            AddressNetwork::Devnet,
            context,
        )
        .unwrap();
        let v2_outcome = execute_transfer_v2(
            &mut v2_state,
            &v2_transaction,
            AddressNetwork::Devnet,
            context,
        )
        .unwrap();

        assert_eq!(v2_state.accounts(), &v1_state);
        assert_eq!(v2_state.channel_count(), 0);
        assert_eq!(v2_outcome.sender, v1_outcome.sender);
        assert_eq!(v2_outcome.recipient, v1_outcome.recipient);
        assert_eq!(v2_outcome.value, v1_outcome.value);
        assert_eq!(v2_outcome.gas_used, v1_outcome.gas_used);
        assert_eq!(v2_outcome.actual_fee, v1_outcome.actual_fee);
        assert_eq!(
            v2_outcome.producer_priority_fee,
            v1_outcome.producer_priority_fee
        );
        assert_eq!(v2_outcome.nonce_after, v1_outcome.nonce_after);
    }

    #[test]
    fn v2_transfer_failure_is_atomic() {
        let key = SigningKey::from_slice(&[0x22; 32]).unwrap();
        let transaction = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x55; 20],
                value: 50_000,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 10,
                max_priority_fee_per_gas: 4,
                data: Vec::new(),
            },
        );
        let sender = transaction
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;

        let mut state = NativeStateV2::default();
        state.accounts_mut().set_account(
            sender,
            AccountStateV1 {
                balance: 1_000,
                nonce: 0,
            },
        );
        let before = state.clone();

        let error = execute_transfer_v2(
            &mut state,
            &transaction,
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 3,
                cpu_producer: [0x99; 20],
            },
        )
        .unwrap_err();

        assert!(error.contains("insufficient native account balance"));
        assert_eq!(state, before);
    }

    #[test]
    fn v2_transfer_executor_rejects_contract_action_without_mutation() {
        let key = SigningKey::from_slice(&[0x33; 32]).unwrap();
        let transaction = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::ContractCall,
                target_payload: vec![0x66; 20],
                value: 0,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 10,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        );

        let mut state = NativeStateV2::default();
        let before = state.clone();
        let error = execute_transfer_v2(
            &mut state,
            &transaction,
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 3,
                cpu_producer: [0x99; 20],
            },
        )
        .unwrap_err();

        assert!(error.contains("requires Transfer action"));
        assert_eq!(state, before);
    }
}
