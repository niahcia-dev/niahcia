use crate::address::AddressNetwork;
use crate::native_compute_execution::{
    execute_inactive_compute_transaction_v2, InactiveComputeExecutionResultV1,
};
use crate::native_execution::{AccountId, NativeExecutionContextV1};
use crate::native_state_v2::NativeStateV2;
use crate::native_transaction_v2::{NativeActionV2, SignedNativeTransactionV2};

pub const NATIVE_COMPUTE_CHANNEL_OPEN_GAS_V1: u64 = 3_000;
pub const NATIVE_COMPUTE_CHANNEL_SETTLE_GAS_V1: u64 = 5_000;
pub const NATIVE_COMPUTE_CHANNEL_REFUND_GAS_V1: u64 = 2_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeComputeFeeOutcomeV1 {
    pub fee_payer: AccountId,
    pub gas_used: u64,
    pub max_execution_charge: u128,
    pub priority_fee_per_gas: u128,
    pub effective_fee_per_gas: u128,
    pub base_fee_burned: u128,
    pub producer_priority_fee: u128,
    pub actual_fee: u128,
    pub unused_fee_reserve: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveComputeFeeExecutionResultV1 {
    pub execution: InactiveComputeExecutionResultV1,
    pub fee: NativeComputeFeeOutcomeV1,
}

pub fn compute_intrinsic_gas_v1(action: NativeActionV2) -> Result<u64, String> {
    match action {
        NativeActionV2::ComputeChannelOpen => Ok(NATIVE_COMPUTE_CHANNEL_OPEN_GAS_V1),
        NativeActionV2::ComputeChannelSettle => Ok(NATIVE_COMPUTE_CHANNEL_SETTLE_GAS_V1),
        NativeActionV2::ComputeChannelRefund => Ok(NATIVE_COMPUTE_CHANNEL_REFUND_GAS_V1),
        _ => Err("native transaction V2 action has no ComputeChannel intrinsic gas".into()),
    }
}

pub fn quote_compute_fee_v1(
    state: &NativeStateV2,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    context: NativeExecutionContextV1,
) -> Result<NativeComputeFeeOutcomeV1, String> {
    transaction.body.validate(network)?;
    let gas_used = compute_intrinsic_gas_v1(transaction.body.action)?;

    if transaction.body.gas_limit < gas_used {
        return Err(format!(
            "compute transaction gas_limit must be at least {gas_used}, found {}",
            transaction.body.gas_limit
        ));
    }

    if transaction.body.max_fee_per_gas < context.base_fee_per_gas {
        return Err(format!(
            "compute transaction max_fee_per_gas {} is below base_fee_per_gas {}",
            transaction.body.max_fee_per_gas, context.base_fee_per_gas
        ));
    }

    let fee_payer = transaction.authenticated_sender(network)?.payload;
    let max_execution_charge = transaction
        .body
        .max_fee_per_gas
        .checked_mul(transaction.body.gas_limit as u128)
        .ok_or_else(|| "compute transaction maximum execution charge overflow".to_string())?;
    let required_balance = transaction
        .body
        .value
        .checked_add(max_execution_charge)
        .ok_or_else(|| "compute transaction required balance overflow".to_string())?;

    let available = state.accounts().account(fee_payer).balance;
    if available < required_balance {
        return Err(format!(
            "insufficient native account balance for compute value plus maximum fee reserve: required {required_balance}, available {available}"
        ));
    }

    let priority_headroom = transaction
        .body
        .max_fee_per_gas
        .checked_sub(context.base_fee_per_gas)
        .ok_or_else(|| "compute priority fee headroom underflow".to_string())?;
    let priority_fee_per_gas = transaction
        .body
        .max_priority_fee_per_gas
        .min(priority_headroom);
    let effective_fee_per_gas = context
        .base_fee_per_gas
        .checked_add(priority_fee_per_gas)
        .ok_or_else(|| "compute effective fee overflow".to_string())?;
    let base_fee_burned = context
        .base_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| "compute base fee burn overflow".to_string())?;
    let producer_priority_fee = priority_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| "compute producer priority fee overflow".to_string())?;
    let actual_fee = base_fee_burned
        .checked_add(producer_priority_fee)
        .ok_or_else(|| "compute actual fee overflow".to_string())?;
    let unused_fee_reserve = max_execution_charge
        .checked_sub(actual_fee)
        .ok_or_else(|| "compute actual fee exceeds maximum reserve".to_string())?;

    Ok(NativeComputeFeeOutcomeV1 {
        fee_payer,
        gas_used,
        max_execution_charge,
        priority_fee_per_gas,
        effective_fee_per_gas,
        base_fee_burned,
        producer_priority_fee,
        actual_fee,
        unused_fee_reserve,
    })
}

pub fn execute_inactive_compute_transaction_with_fee_v2(
    state: &mut NativeStateV2,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    current_height: u64,
    context: NativeExecutionContextV1,
) -> Result<InactiveComputeFeeExecutionResultV1, String> {
    let mut next = state.clone();
    let fee = quote_compute_fee_v1(&next, transaction, network, context)?;
    let mut execution =
        execute_inactive_compute_transaction_v2(&mut next, transaction, network, current_height)?;

    next.accounts_mut().debit(fee.fee_payer, fee.actual_fee)?;
    next.accounts_mut()
        .credit(context.cpu_producer, fee.producer_priority_fee)?;

    execution.state_root_after = next.state_root()?;
    *state = next;

    Ok(InactiveComputeFeeExecutionResultV1 { execution, fee })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_execution::{AccountStateV1, NativeStateV1};
    use crate::native_transaction::{DEVNET_CHAIN_ID, DEVNET_NETWORK_ID};
    use crate::native_transaction_v2::NativeTransactionBodyV2;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn sign(key: &SigningKey, body: NativeTransactionBodyV2) -> SignedNativeTransactionV2 {
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

    #[test]
    fn candidate_compute_intrinsic_gas_values_are_exact() {
        assert_eq!(
            compute_intrinsic_gas_v1(NativeActionV2::ComputeChannelOpen).unwrap(),
            3_000
        );
        assert_eq!(
            compute_intrinsic_gas_v1(NativeActionV2::ComputeChannelSettle).unwrap(),
            5_000
        );
        assert_eq!(
            compute_intrinsic_gas_v1(NativeActionV2::ComputeChannelRefund).unwrap(),
            2_000
        );
        assert!(compute_intrinsic_gas_v1(NativeActionV2::Transfer).is_err());
    }

    #[test]
    fn fee_quote_matches_transfer_fee_policy_shape() {
        let key = SigningKey::from_slice(&[0x31; 32]).unwrap();
        let transaction = sign(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::ComputeChannelRefund,
                target_payload: Vec::new(),
                value: 0,
                gas_limit: NATIVE_COMPUTE_CHANNEL_REFUND_GAS_V1,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 1,
                data: vec![0xa1, 0x01, 0x01],
            },
        );
        let payer = transaction
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            payer,
            AccountStateV1 {
                balance: 100_000,
                nonce: 0,
            },
        );
        let state = NativeStateV2::from_v1(accounts);

        let fee = quote_compute_fee_v1(
            &state,
            &transaction,
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 2,
                cpu_producer: [0x77; 20],
            },
        )
        .unwrap();

        assert_eq!(fee.gas_used, 2_000);
        assert_eq!(fee.max_execution_charge, 10_000);
        assert_eq!(fee.priority_fee_per_gas, 1);
        assert_eq!(fee.effective_fee_per_gas, 3);
        assert_eq!(fee.base_fee_burned, 4_000);
        assert_eq!(fee.producer_priority_fee, 2_000);
        assert_eq!(fee.actual_fee, 6_000);
        assert_eq!(fee.unused_fee_reserve, 4_000);
    }

    #[test]
    fn compute_gas_fee_vector_matches_json() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/native-compute-gas-v1.json"
        ))
        .unwrap();

        assert_eq!(
            vectors["intrinsic_gas"]["transfer_baseline"]
                .as_u64()
                .unwrap(),
            crate::native_transaction::NATIVE_TRANSFER_GAS_V1
        );
        assert_eq!(
            vectors["intrinsic_gas"]["compute_channel_open"]
                .as_u64()
                .unwrap(),
            NATIVE_COMPUTE_CHANNEL_OPEN_GAS_V1
        );
        assert_eq!(
            vectors["intrinsic_gas"]["compute_channel_settle"]
                .as_u64()
                .unwrap(),
            NATIVE_COMPUTE_CHANNEL_SETTLE_GAS_V1
        );
        assert_eq!(
            vectors["intrinsic_gas"]["compute_channel_refund"]
                .as_u64()
                .unwrap(),
            NATIVE_COMPUTE_CHANNEL_REFUND_GAS_V1
        );

        let sample = &vectors["fee_example"];
        let gas_used = sample["gas_used"].as_u64().unwrap();
        let gas_limit = sample["gas_limit"].as_u64().unwrap();
        let base_fee_per_gas: u128 = sample["base_fee_per_gas"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let max_fee_per_gas: u128 = sample["max_fee_per_gas"].as_str().unwrap().parse().unwrap();
        let max_priority_fee_per_gas: u128 = sample["max_priority_fee_per_gas"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();

        let priority = max_priority_fee_per_gas.min(max_fee_per_gas - base_fee_per_gas);
        let effective = base_fee_per_gas + priority;
        let max_reserve = max_fee_per_gas * gas_limit as u128;
        let burned = base_fee_per_gas * gas_used as u128;
        let producer = priority * gas_used as u128;
        let actual = burned + producer;
        let unused = max_reserve - actual;

        assert_eq!(
            effective.to_string(),
            sample["effective_fee_per_gas"].as_str().unwrap()
        );
        assert_eq!(
            max_reserve.to_string(),
            sample["max_execution_charge"].as_str().unwrap()
        );
        assert_eq!(
            burned.to_string(),
            sample["base_fee_burned"].as_str().unwrap()
        );
        assert_eq!(
            producer.to_string(),
            sample["producer_priority_fee"].as_str().unwrap()
        );
        assert_eq!(actual.to_string(), sample["actual_fee"].as_str().unwrap());
        assert_eq!(
            unused.to_string(),
            sample["unused_fee_reserve"].as_str().unwrap()
        );
    }

    #[test]
    fn fee_quote_rejects_intrinsic_gas_underflow() {
        let key = SigningKey::from_slice(&[0x32; 32]).unwrap();
        let transaction = sign(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::ComputeChannelOpen,
                target_payload: Vec::new(),
                value: 1_000,
                gas_limit: NATIVE_COMPUTE_CHANNEL_OPEN_GAS_V1 - 1,
                max_fee_per_gas: 2,
                max_priority_fee_per_gas: 0,
                data: vec![0xa1, 0x01, 0x01],
            },
        );
        let payer = transaction
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            payer,
            AccountStateV1 {
                balance: 100_000,
                nonce: 0,
            },
        );

        let error = quote_compute_fee_v1(
            &NativeStateV2::from_v1(accounts),
            &transaction,
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 1,
                cpu_producer: [0_u8; 20],
            },
        )
        .unwrap_err();
        assert!(error.contains("gas_limit must be at least 3000"));
    }
}
