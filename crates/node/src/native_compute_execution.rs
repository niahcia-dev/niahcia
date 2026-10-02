use crate::address::AddressNetwork;
use crate::native_compute_payloads::{derive_compute_channel_id_v1, ComputeChannelOpenPayloadV1};
use crate::native_state_v2::{ComputeChannelStateV1, ComputeChannelStatusV1, NativeStateV2};
use crate::native_transaction_v2::{NativeActionV2, SignedNativeTransactionV2};
use crate::work::Hash32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedComputeChannelOpenV1 {
    pub transaction_id: Hash32,
    pub channel_id: Hash32,
    pub funding_account: [u8; 20],
    pub expected_nonce: u64,
    pub authorized_amount: u128,
    pub channel_state: ComputeChannelStateV1,
}

pub fn plan_compute_channel_open_v1(
    state: &NativeStateV2,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    current_height: u64,
) -> Result<ValidatedComputeChannelOpenV1, String> {
    transaction.verify_signature(network)?;

    if transaction.body.action != NativeActionV2::ComputeChannelOpen {
        return Err("native transaction V2 action is not ComputeChannelOpen".into());
    }

    let payload = ComputeChannelOpenPayloadV1::from_canonical_bytes(&transaction.body.data)?;

    if transaction.body.value != payload.authorized_amount {
        return Err(format!(
            "ComputeChannelOpen transaction value mismatch: expected {}, found {}",
            payload.authorized_amount, transaction.body.value
        ));
    }

    if current_height >= payload.expiry_height {
        return Err("ComputeChannelOpen expiry must be after the current block height".into());
    }

    let sender = transaction.authenticated_sender(network)?;
    let funding_account = sender.payload;
    let account = state.accounts().account(funding_account);

    if account.nonce != transaction.body.nonce {
        return Err(format!(
            "native account nonce mismatch: expected {}, found {}",
            account.nonce, transaction.body.nonce
        ));
    }

    if account.balance < payload.authorized_amount {
        return Err(
            "insufficient native account balance for ComputeChannelOpen authorization".into(),
        );
    }

    let transaction_id = transaction.tx_id()?;
    let channel_id = derive_compute_channel_id_v1(transaction_id);

    if state.channel(channel_id).is_some() {
        return Err("ComputeChannelOpen channel_id already exists".into());
    }

    let channel_state = ComputeChannelStateV1 {
        channel_id,
        funding_account,
        worker_id: payload.worker_id,
        operator_id: payload.operator_id,
        channel_public_key: payload.channel_public_key,
        worker_payment_account: payload.worker_payment_account,
        authorized_amount: payload.authorized_amount,
        settled_amount: 0,
        opened_height: current_height,
        expiry_height: payload.expiry_height,
        claim_deadline_height: payload.claim_deadline_height,
        refund_available_height: payload.refund_available_height,
        service_scope_commitment: payload.service_scope_commitment,
        model_scope_commitment: payload.model_scope_commitment,
        execution_profile_scope_commitment: payload.execution_profile_scope_commitment,
        settlement_policy: payload.settlement_policy,
        state: ComputeChannelStatusV1::Open,
    };
    channel_state.validate()?;

    Ok(ValidatedComputeChannelOpenV1 {
        transaction_id,
        channel_id,
        funding_account,
        expected_nonce: transaction.body.nonce,
        authorized_amount: payload.authorized_amount,
        channel_state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_execution::{AccountStateV1, NativeStateV1};
    use crate::native_state_v2::ComputeChannelSettlementPolicyV1;
    use crate::native_transaction::{DEVNET_CHAIN_ID, DEVNET_NETWORK_ID};
    use crate::native_transaction_v2::NativeTransactionBodyV2;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn open_payload(expiry_height: u64) -> ComputeChannelOpenPayloadV1 {
        let channel_key = SigningKey::from_slice(&[0x22; 32]).unwrap();
        let encoded = channel_key.verifying_key().to_encoded_point(false);
        let channel_public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();

        ComputeChannelOpenPayloadV1 {
            worker_id: [0x33; 32],
            operator_id: [0x44; 32],
            channel_public_key,
            worker_payment_account: [0x55; 20],
            authorized_amount: 1_000,
            expiry_height,
            claim_deadline_height: expiry_height + 10,
            refund_available_height: expiry_height + 11,
            service_scope_commitment: [0x66; 32],
            model_scope_commitment: [0x77; 32],
            execution_profile_scope_commitment: [0x88; 32],
            settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
        }
    }

    fn signed_open(
        signing_key: &SigningKey,
        nonce: u64,
        value: u128,
        expiry_height: u64,
    ) -> SignedNativeTransactionV2 {
        let payload = open_payload(expiry_height);
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

        let mut transaction = SignedNativeTransactionV2 {
            body: NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce,
                action: NativeActionV2::ComputeChannelOpen,
                target_payload: Vec::new(),
                value,
                gas_limit: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                data: payload.canonical_bytes().unwrap(),
            },
            public_key,
            signature: vec![0; 64],
        };

        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        transaction
    }

    fn state_for(
        transaction: &SignedNativeTransactionV2,
        balance: u128,
        nonce: u64,
    ) -> NativeStateV2 {
        let sender = transaction
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let mut accounts = NativeStateV1::default();
        accounts.set_account(sender, AccountStateV1 { balance, nonce });
        NativeStateV2::from_v1(accounts)
    }

    #[test]
    fn open_plan_derives_exact_channel_without_mutating_state() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 3, 1_000, 100);
        let state = state_for(&transaction, 5_000, 3);
        let before = state.clone();

        let plan =
            plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50).unwrap();

        assert_eq!(state, before);
        assert_eq!(plan.expected_nonce, 3);
        assert_eq!(plan.authorized_amount, 1_000);
        assert_eq!(plan.channel_state.channel_id, plan.channel_id);
        assert_eq!(plan.channel_state.funding_account, plan.funding_account);
        assert_eq!(plan.channel_state.opened_height, 50);
        assert_eq!(plan.channel_state.expiry_height, 100);
        assert_eq!(plan.channel_state.settled_amount, 0);
        assert_eq!(plan.channel_state.state, ComputeChannelStatusV1::Open);
        assert_eq!(
            plan.channel_id,
            derive_compute_channel_id_v1(transaction.tx_id().unwrap())
        );
    }

    #[test]
    fn open_plan_rejects_wrong_action() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let mut transaction = signed_open(&signing_key, 0, 1_000, 100);
        transaction.body.action = NativeActionV2::ComputeChannelRefund;
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        let state = state_for(&transaction, 5_000, 0);

        let error = plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50)
            .unwrap_err();
        assert!(error.contains("not ComputeChannelOpen"));
    }

    #[test]
    fn open_plan_rejects_wrong_network_or_signature() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 0, 1_000, 100);
        let state = state_for(&transaction, 5_000, 0);

        let result =
            plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Mainnet, 50);
        assert!(result.is_err());

        let mut tampered = transaction;
        tampered.signature[0] ^= 1;
        let result = plan_compute_channel_open_v1(&state, &tampered, AddressNetwork::Devnet, 50);
        assert!(result.is_err());
    }

    #[test]
    fn open_plan_rejects_value_mismatch() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 0, 999, 100);
        let state = state_for(&transaction, 5_000, 0);

        let error = plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50)
            .unwrap_err();
        assert!(error.contains("value mismatch"));
    }

    #[test]
    fn open_plan_rejects_expired_height_window() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 0, 1_000, 50);
        let state = state_for(&transaction, 5_000, 0);

        let error = plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50)
            .unwrap_err();
        assert!(error.contains("expiry"));
    }

    #[test]
    fn open_plan_rejects_nonce_mismatch() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 4, 1_000, 100);
        let state = state_for(&transaction, 5_000, 3);

        let error = plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50)
            .unwrap_err();
        assert!(error.contains("nonce mismatch"));
    }

    #[test]
    fn open_plan_rejects_insufficient_authorized_balance() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 0, 1_000, 100);
        let state = state_for(&transaction, 999, 0);

        let error = plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50)
            .unwrap_err();
        assert!(error.contains("insufficient"));
    }

    #[test]
    fn open_plan_rejects_duplicate_channel_id() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 0, 1_000, 100);
        let mut state = state_for(&transaction, 5_000, 0);

        let first =
            plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50).unwrap();
        state.set_channel(first.channel_state).unwrap();

        let error = plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50)
            .unwrap_err();
        assert!(error.contains("already exists"));
    }

    #[test]
    fn open_plan_rejects_noncanonical_payload() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let mut transaction = signed_open(&signing_key, 0, 1_000, 100);
        transaction.body.data.push(0);
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        let state = state_for(&transaction, 5_000, 0);

        let result = plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50);
        assert!(result.is_err());
    }
}
