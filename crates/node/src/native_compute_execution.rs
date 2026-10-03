use crate::address::AddressNetwork;
use crate::compute_usage_receipt_v1::ComputeUsageReceiptV1;
use crate::native_compute_payloads::{
    derive_compute_channel_id_v1, ComputeChannelOpenPayloadV1, ComputeChannelRefundPayloadV1,
    ComputeChannelSettlePayloadV1,
};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedComputeChannelSettleV1 {
    pub transaction_id: Hash32,
    pub channel_id: Hash32,
    pub submitter_account: [u8; 20],
    pub expected_nonce: u64,
    pub receipt_id: Hash32,
    pub receipt_sequence: u64,
    pub cumulative_spent: u128,
    pub worker_payment: u128,
    pub funding_refund: u128,
    pub channel_state_before: ComputeChannelStateV1,
    pub channel_state_after: ComputeChannelStateV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedComputeChannelRefundV1 {
    pub transaction_id: Hash32,
    pub channel_id: Hash32,
    pub funding_account: [u8; 20],
    pub expected_nonce: u64,
    pub refund_amount: u128,
    pub channel_state_before: ComputeChannelStateV1,
    pub channel_state_after: ComputeChannelStateV1,
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

pub fn plan_compute_channel_settle_v1(
    state: &NativeStateV2,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    current_height: u64,
) -> Result<ValidatedComputeChannelSettleV1, String> {
    transaction.verify_signature(network)?;

    if transaction.body.action != NativeActionV2::ComputeChannelSettle {
        return Err("native transaction V2 action is not ComputeChannelSettle".into());
    }

    if transaction.body.value != 0 {
        return Err("ComputeChannelSettle transaction value must be zero".into());
    }

    let payload = ComputeChannelSettlePayloadV1::from_canonical_bytes(&transaction.body.data)?;
    let channel = state
        .channel(payload.channel_id)
        .cloned()
        .ok_or_else(|| "ComputeChannelSettle channel does not exist".to_string())?;

    if channel.state != ComputeChannelStatusV1::Open {
        return Err("ComputeChannelSettle channel is not OPEN".into());
    }

    let submitter = transaction.authenticated_sender(network)?.payload;
    if submitter != channel.worker_payment_account {
        return Err(
            "ComputeChannelSettle sender is not the committed worker_payment_account".into(),
        );
    }

    let account = state.accounts().account(submitter);
    if account.nonce != transaction.body.nonce {
        return Err(format!(
            "native account nonce mismatch: expected {}, found {}",
            account.nonce, transaction.body.nonce
        ));
    }

    if current_height > channel.claim_deadline_height {
        return Err("ComputeChannelSettle is after the channel claim deadline".into());
    }

    let receipt = ComputeUsageReceiptV1::from_canonical_bytes(&payload.final_usage_receipt)?;

    if current_height > receipt.expires_at {
        return Err("ComputeChannelSettle receipt is expired".into());
    }

    if receipt.worker_id != channel.worker_id {
        return Err("ComputeChannelSettle receipt worker_id mismatch".into());
    }

    if receipt.operator_id != channel.operator_id {
        return Err("ComputeChannelSettle receipt operator_id mismatch".into());
    }

    receipt.verify_signature(network, &channel.channel_public_key)?;

    if receipt.cumulative_spent > channel.authorized_amount {
        return Err("ComputeChannelSettle cumulative spend exceeds channel authorization".into());
    }

    if receipt.cumulative_spent < channel.settled_amount {
        return Err("ComputeChannelSettle cumulative spend is below accepted channel state".into());
    }

    let funding_refund = channel
        .authorized_amount
        .checked_sub(receipt.cumulative_spent)
        .ok_or_else(|| "ComputeChannelSettle refund arithmetic underflow".to_string())?;

    let channel_state_before = channel.clone();
    let mut channel_state_after = channel;
    channel_state_after.settled_amount = receipt.cumulative_spent;
    channel_state_after.state = ComputeChannelStatusV1::Settled;
    channel_state_after.validate()?;

    Ok(ValidatedComputeChannelSettleV1 {
        transaction_id: transaction.tx_id()?,
        channel_id: payload.channel_id,
        submitter_account: submitter,
        expected_nonce: transaction.body.nonce,
        receipt_id: receipt.receipt_id,
        receipt_sequence: receipt.sequence,
        cumulative_spent: receipt.cumulative_spent,
        worker_payment: receipt.cumulative_spent,
        funding_refund,
        channel_state_before,
        channel_state_after,
    })
}

pub fn plan_compute_channel_refund_v1(
    state: &NativeStateV2,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    current_height: u64,
) -> Result<ValidatedComputeChannelRefundV1, String> {
    transaction.verify_signature(network)?;

    if transaction.body.action != NativeActionV2::ComputeChannelRefund {
        return Err("native transaction V2 action is not ComputeChannelRefund".into());
    }

    if transaction.body.value != 0 {
        return Err("ComputeChannelRefund transaction value must be zero".into());
    }

    let payload = ComputeChannelRefundPayloadV1::from_canonical_bytes(&transaction.body.data)?;
    let channel = state
        .channel(payload.channel_id)
        .cloned()
        .ok_or_else(|| "ComputeChannelRefund channel does not exist".to_string())?;

    if channel.state != ComputeChannelStatusV1::Open {
        return Err("ComputeChannelRefund channel is not OPEN".into());
    }

    let funding_account = transaction.authenticated_sender(network)?.payload;
    if funding_account != channel.funding_account {
        return Err("ComputeChannelRefund sender is not the committed funding_account".into());
    }

    let account = state.accounts().account(funding_account);
    if account.nonce != transaction.body.nonce {
        return Err(format!(
            "native account nonce mismatch: expected {}, found {}",
            account.nonce, transaction.body.nonce
        ));
    }

    if current_height < channel.refund_available_height {
        return Err("ComputeChannelRefund is before the refund available height".into());
    }

    let refund_amount = channel
        .authorized_amount
        .checked_sub(channel.settled_amount)
        .ok_or_else(|| "ComputeChannelRefund arithmetic underflow".to_string())?;

    let channel_state_before = channel.clone();
    let mut channel_state_after = channel;
    channel_state_after.state = ComputeChannelStatusV1::Refunded;
    channel_state_after.validate()?;

    Ok(ValidatedComputeChannelRefundV1 {
        transaction_id: transaction.tx_id()?,
        channel_id: payload.channel_id,
        funding_account,
        expected_nonce: transaction.body.nonce,
        refund_amount,
        channel_state_before,
        channel_state_after,
    })
}

pub fn apply_compute_channel_open_v1(
    state: &mut NativeStateV2,
    plan: &ValidatedComputeChannelOpenV1,
) -> Result<(), String> {
    plan.channel_state.validate()?;
    if plan.channel_state.channel_id != plan.channel_id
        || plan.channel_state.funding_account != plan.funding_account
        || plan.channel_state.authorized_amount != plan.authorized_amount
        || plan.channel_state.state != ComputeChannelStatusV1::Open
        || plan.channel_state.settled_amount != 0
    {
        return Err("ComputeChannelOpen validated plan is internally inconsistent".into());
    }

    let mut next = state.clone();
    if next.channel(plan.channel_id).is_some() {
        return Err("ComputeChannelOpen channel_id already exists during apply".into());
    }

    next.accounts_mut()
        .debit(plan.funding_account, plan.authorized_amount)?;
    next.accounts_mut()
        .consume_nonce(plan.funding_account, plan.expected_nonce)?;
    next.set_channel(plan.channel_state.clone())?;

    *state = next;
    Ok(())
}

pub fn apply_compute_channel_settle_v1(
    state: &mut NativeStateV2,
    plan: &ValidatedComputeChannelSettleV1,
) -> Result<(), String> {
    plan.channel_state_before.validate()?;
    plan.channel_state_after.validate()?;

    if plan.channel_state_before.channel_id != plan.channel_id
        || plan.submitter_account != plan.channel_state_before.worker_payment_account
        || plan.channel_state_before.state != ComputeChannelStatusV1::Open
        || plan.cumulative_spent != plan.worker_payment
    {
        return Err("ComputeChannelSettle validated plan is internally inconsistent".into());
    }

    let mut expected_after = plan.channel_state_before.clone();
    expected_after.settled_amount = plan.worker_payment;
    expected_after.state = ComputeChannelStatusV1::Settled;
    if plan.channel_state_after != expected_after {
        return Err("ComputeChannelSettle validated plan changes unexpected channel fields".into());
    }

    let total = plan
        .worker_payment
        .checked_add(plan.funding_refund)
        .ok_or_else(|| "ComputeChannelSettle settlement arithmetic overflow".to_string())?;
    if total != plan.channel_state_before.authorized_amount {
        return Err("ComputeChannelSettle validated plan does not conserve channel value".into());
    }

    let mut next = state.clone();
    let current = next
        .channel(plan.channel_id)
        .ok_or_else(|| "ComputeChannelSettle channel disappeared before apply".to_string())?;
    if current != &plan.channel_state_before {
        return Err("ComputeChannelSettle validated plan is stale".into());
    }

    next.accounts_mut().credit(
        plan.channel_state_before.worker_payment_account,
        plan.worker_payment,
    )?;
    next.accounts_mut().credit(
        plan.channel_state_before.funding_account,
        plan.funding_refund,
    )?;
    next.accounts_mut()
        .consume_nonce(plan.submitter_account, plan.expected_nonce)?;
    next.set_channel(plan.channel_state_after.clone())?;

    *state = next;
    Ok(())
}

pub fn apply_compute_channel_refund_v1(
    state: &mut NativeStateV2,
    plan: &ValidatedComputeChannelRefundV1,
) -> Result<(), String> {
    plan.channel_state_before.validate()?;
    plan.channel_state_after.validate()?;

    if plan.channel_state_before.channel_id != plan.channel_id
        || plan.channel_state_before.funding_account != plan.funding_account
        || plan.channel_state_before.state != ComputeChannelStatusV1::Open
    {
        return Err("ComputeChannelRefund validated plan is internally inconsistent".into());
    }

    let mut expected_after = plan.channel_state_before.clone();
    expected_after.state = ComputeChannelStatusV1::Refunded;
    if plan.channel_state_after != expected_after {
        return Err("ComputeChannelRefund validated plan changes unexpected channel fields".into());
    }

    let expected_refund = plan
        .channel_state_before
        .authorized_amount
        .checked_sub(plan.channel_state_before.settled_amount)
        .ok_or_else(|| "ComputeChannelRefund arithmetic underflow".to_string())?;
    if expected_refund != plan.refund_amount {
        return Err("ComputeChannelRefund validated plan refund amount mismatch".into());
    }

    let mut next = state.clone();
    let current = next
        .channel(plan.channel_id)
        .ok_or_else(|| "ComputeChannelRefund channel disappeared before apply".to_string())?;
    if current != &plan.channel_state_before {
        return Err("ComputeChannelRefund validated plan is stale".into());
    }

    next.accounts_mut()
        .credit(plan.funding_account, plan.refund_amount)?;
    next.accounts_mut()
        .consume_nonce(plan.funding_account, plan.expected_nonce)?;
    next.set_channel(plan.channel_state_after.clone())?;

    *state = next;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppliedComputeTransitionV1 {
    Open(ValidatedComputeChannelOpenV1),
    Settle(ValidatedComputeChannelSettleV1),
    Refund(ValidatedComputeChannelRefundV1),
}

impl AppliedComputeTransitionV1 {
    pub fn transaction_id(&self) -> Hash32 {
        match self {
            Self::Open(plan) => plan.transaction_id,
            Self::Settle(plan) => plan.transaction_id,
            Self::Refund(plan) => plan.transaction_id,
        }
    }

    pub fn channel_id(&self) -> Hash32 {
        match self {
            Self::Open(plan) => plan.channel_id,
            Self::Settle(plan) => plan.channel_id,
            Self::Refund(plan) => plan.channel_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveComputeExecutionResultV1 {
    pub transition: AppliedComputeTransitionV1,
    pub state_root_before: Hash32,
    pub state_root_after: Hash32,
}

pub fn execute_inactive_compute_transaction_v2(
    state: &mut NativeStateV2,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    current_height: u64,
) -> Result<InactiveComputeExecutionResultV1, String> {
    let state_root_before = state.state_root()?;
    let mut next = state.clone();

    let transition = match transaction.body.action {
        NativeActionV2::ComputeChannelOpen => {
            let plan = plan_compute_channel_open_v1(&next, transaction, network, current_height)?;
            apply_compute_channel_open_v1(&mut next, &plan)?;
            AppliedComputeTransitionV1::Open(plan)
        }
        NativeActionV2::ComputeChannelSettle => {
            let plan = plan_compute_channel_settle_v1(&next, transaction, network, current_height)?;
            apply_compute_channel_settle_v1(&mut next, &plan)?;
            AppliedComputeTransitionV1::Settle(plan)
        }
        NativeActionV2::ComputeChannelRefund => {
            let plan = plan_compute_channel_refund_v1(&next, transaction, network, current_height)?;
            apply_compute_channel_refund_v1(&mut next, &plan)?;
            AppliedComputeTransitionV1::Refund(plan)
        }
        _ => {
            return Err(
                "native transaction V2 action is not an inactive ComputeChannel action".into(),
            )
        }
    };

    let state_root_after = next.state_root()?;
    *state = next;

    Ok(InactiveComputeExecutionResultV1 {
        transition,
        state_root_before,
        state_root_after,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveComputeBatchExecutionResultV1 {
    pub transactions: Vec<InactiveComputeExecutionResultV1>,
    pub state_root_before: Hash32,
    pub state_root_after: Hash32,
}

pub fn execute_inactive_compute_batch_v2(
    state: &mut NativeStateV2,
    transactions: &[SignedNativeTransactionV2],
    network: AddressNetwork,
    current_height: u64,
) -> Result<InactiveComputeBatchExecutionResultV1, String> {
    let state_root_before = state.state_root()?;
    let mut next = state.clone();
    let mut results = Vec::with_capacity(transactions.len());

    for transaction in transactions {
        results.push(execute_inactive_compute_transaction_v2(
            &mut next,
            transaction,
            network,
            current_height,
        )?);
    }

    let state_root_after = next.state_root()?;
    *state = next;

    Ok(InactiveComputeBatchExecutionResultV1 {
        transactions: results,
        state_root_before,
        state_root_after,
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

    fn resign_v2(transaction: &mut SignedNativeTransactionV2, signing_key: &SigningKey) {
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
    }

    fn settle_fixture() -> (
        NativeStateV2,
        SignedNativeTransactionV2,
        SigningKey,
        SigningKey,
    ) {
        let worker_key = SigningKey::from_slice(&[0x31; 32]).unwrap();
        let worker_public_key = worker_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        let worker_payment_account =
            crate::address::NiahciaAddressV1::account_from_uncompressed_public_key(
                AddressNetwork::Devnet,
                &worker_public_key,
            )
            .unwrap()
            .payload;

        let channel_key = SigningKey::from_slice(&[0x41; 32]).unwrap();
        let channel_encoded = channel_key.verifying_key().to_encoded_point(false);
        let channel_public_key: [u8; 65] = channel_encoded.as_bytes().try_into().unwrap();

        let channel_id = [0x91; 32];
        let worker_id = [0x92; 32];
        let operator_id = [0x93; 32];

        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            worker_payment_account,
            AccountStateV1 {
                balance: 100,
                nonce: 2,
            },
        );

        let mut state = NativeStateV2::from_v1(accounts);
        state
            .set_channel(ComputeChannelStateV1 {
                channel_id,
                funding_account: [0x94; 20],
                worker_id,
                operator_id,
                channel_public_key,
                worker_payment_account,
                authorized_amount: 1_000,
                settled_amount: 0,
                opened_height: 10,
                expiry_height: 100,
                claim_deadline_height: 120,
                refund_available_height: 121,
                service_scope_commitment: [0x95; 32],
                model_scope_commitment: [0x96; 32],
                execution_profile_scope_commitment: [0x97; 32],
                settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
                state: ComputeChannelStatusV1::Open,
            })
            .unwrap();

        let mut receipt = ComputeUsageReceiptV1 {
            receipt_id: [0xa1; 32],
            channel_id,
            authorization_id: [0xa2; 32],
            worker_id,
            operator_id,
            sequence: 3,
            previous_receipt_id: [0xa3; 32],
            cumulative_spent: 370,
            job_id: [0xa4; 32],
            result_commitment_id: [0xa5; 32],
            price_offer_id: [0xa6; 32],
            job_charge: 25,
            metering_evidence_hash: [0xa7; 32],
            expires_at: 115,
            channel_signature: vec![0; 64],
        };
        let digest = receipt.signing_digest(AddressNetwork::Devnet).unwrap();
        let signature: Signature = channel_key.sign_prehash(&digest).unwrap();
        receipt.channel_signature = signature.to_bytes().to_vec();

        let settle_payload = ComputeChannelSettlePayloadV1 {
            channel_id,
            final_usage_receipt: receipt.canonical_bytes().unwrap(),
        };

        let mut transaction = SignedNativeTransactionV2 {
            body: NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 2,
                action: NativeActionV2::ComputeChannelSettle,
                target_payload: Vec::new(),
                value: 0,
                gas_limit: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                data: settle_payload.canonical_bytes().unwrap(),
            },
            public_key: worker_public_key,
            signature: vec![0; 64],
        };
        resign_v2(&mut transaction, &worker_key);

        (state, transaction, worker_key, channel_key)
    }

    fn refund_fixture() -> (NativeStateV2, SignedNativeTransactionV2, SigningKey) {
        let funding_key = SigningKey::from_slice(&[0x51; 32]).unwrap();
        let funding_public_key = funding_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        let funding_account =
            crate::address::NiahciaAddressV1::account_from_uncompressed_public_key(
                AddressNetwork::Devnet,
                &funding_public_key,
            )
            .unwrap()
            .payload;

        let channel_key = SigningKey::from_slice(&[0x52; 32]).unwrap();
        let channel_encoded = channel_key.verifying_key().to_encoded_point(false);
        let channel_public_key: [u8; 65] = channel_encoded.as_bytes().try_into().unwrap();

        let channel_id = [0xb1; 32];
        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            funding_account,
            AccountStateV1 {
                balance: 250,
                nonce: 4,
            },
        );

        let mut state = NativeStateV2::from_v1(accounts);
        state
            .set_channel(ComputeChannelStateV1 {
                channel_id,
                funding_account,
                worker_id: [0xb2; 32],
                operator_id: [0xb3; 32],
                channel_public_key,
                worker_payment_account: [0xb4; 20],
                authorized_amount: 1_000,
                settled_amount: 0,
                opened_height: 10,
                expiry_height: 100,
                claim_deadline_height: 120,
                refund_available_height: 121,
                service_scope_commitment: [0xb5; 32],
                model_scope_commitment: [0xb6; 32],
                execution_profile_scope_commitment: [0xb7; 32],
                settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
                state: ComputeChannelStatusV1::Open,
            })
            .unwrap();

        let payload = ComputeChannelRefundPayloadV1 { channel_id };
        let mut transaction = SignedNativeTransactionV2 {
            body: NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 4,
                action: NativeActionV2::ComputeChannelRefund,
                target_payload: Vec::new(),
                value: 0,
                gas_limit: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                data: payload.canonical_bytes().unwrap(),
            },
            public_key: funding_public_key,
            signature: vec![0; 64],
        };
        resign_v2(&mut transaction, &funding_key);

        (state, transaction, funding_key)
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
    fn settle_plan_derives_payment_and_refund_without_mutation() {
        let (state, transaction, _, _) = settle_fixture();
        let before = state.clone();

        let plan =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 110)
                .unwrap();

        assert_eq!(state, before);
        assert_eq!(plan.expected_nonce, 2);
        assert_eq!(plan.cumulative_spent, 370);
        assert_eq!(plan.worker_payment, 370);
        assert_eq!(plan.funding_refund, 630);
        assert_eq!(plan.channel_state_after.settled_amount, 370);
        assert_eq!(
            plan.channel_state_after.state,
            ComputeChannelStatusV1::Settled
        );
    }

    #[test]
    fn open_apply_locks_value_consumes_nonce_and_records_channel() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 3, 1_000, 100);
        let mut state = state_for(&transaction, 5_000, 3);
        let plan =
            plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50).unwrap();

        apply_compute_channel_open_v1(&mut state, &plan).unwrap();

        assert_eq!(
            state.accounts().account(plan.funding_account).balance,
            4_000
        );
        assert_eq!(state.accounts().account(plan.funding_account).nonce, 4);
        assert_eq!(state.channel(plan.channel_id), Some(&plan.channel_state));
    }

    #[test]
    fn open_apply_rolls_back_when_nonce_overflows() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, u64::MAX, 1_000, 100);
        let mut state = state_for(&transaction, 5_000, u64::MAX);
        let plan =
            plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50).unwrap();
        let before = state.clone();

        let error = apply_compute_channel_open_v1(&mut state, &plan).unwrap_err();
        assert!(error.contains("nonce overflow"));
        assert_eq!(state, before);
    }

    #[test]
    fn settle_apply_pays_worker_refunds_funder_and_closes_channel() {
        let (mut state, transaction, _, _) = settle_fixture();
        let plan =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 110)
                .unwrap();

        apply_compute_channel_settle_v1(&mut state, &plan).unwrap();

        assert_eq!(
            state.accounts().account(plan.submitter_account),
            AccountStateV1 {
                balance: 470,
                nonce: 3,
            }
        );
        assert_eq!(
            state
                .accounts()
                .account(plan.channel_state_before.funding_account)
                .balance,
            630
        );
        assert_eq!(
            state.channel(plan.channel_id),
            Some(&plan.channel_state_after)
        );
    }

    #[test]
    fn settle_apply_rolls_back_on_credit_overflow() {
        let (mut state, transaction, _, _) = settle_fixture();
        let plan =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 110)
                .unwrap();
        state.accounts_mut().set_account(
            plan.channel_state_before.funding_account,
            AccountStateV1 {
                balance: u128::MAX,
                nonce: 0,
            },
        );
        let before = state.clone();

        let error = apply_compute_channel_settle_v1(&mut state, &plan).unwrap_err();
        assert!(error.contains("balance overflow"));
        assert_eq!(state, before);
    }

    #[test]
    fn refund_apply_returns_locked_value_consumes_nonce_and_closes_channel() {
        let (mut state, transaction, _) = refund_fixture();
        let plan =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap();

        apply_compute_channel_refund_v1(&mut state, &plan).unwrap();

        assert_eq!(
            state.accounts().account(plan.funding_account),
            AccountStateV1 {
                balance: 1_250,
                nonce: 5,
            }
        );
        assert_eq!(
            state.channel(plan.channel_id),
            Some(&plan.channel_state_after)
        );
    }

    #[test]
    fn refund_apply_rolls_back_on_credit_overflow() {
        let (mut state, transaction, _) = refund_fixture();
        let plan =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap();
        state.accounts_mut().set_account(
            plan.funding_account,
            AccountStateV1 {
                balance: u128::MAX,
                nonce: plan.expected_nonce,
            },
        );
        let before = state.clone();

        let error = apply_compute_channel_refund_v1(&mut state, &plan).unwrap_err();
        assert!(error.contains("balance overflow"));
        assert_eq!(state, before);
    }

    #[test]
    fn open_apply_changes_state_root_deterministically() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 3, 1_000, 100);
        let mut state = state_for(&transaction, 5_000, 3);
        let before_root = state.state_root().unwrap();
        let plan =
            plan_compute_channel_open_v1(&state, &transaction, AddressNetwork::Devnet, 50).unwrap();

        apply_compute_channel_open_v1(&mut state, &plan).unwrap();
        let after_root = state.state_root().unwrap();

        assert_ne!(after_root, before_root);
        let restored =
            NativeStateV2::from_canonical_bytes(&state.canonical_bytes().unwrap()).unwrap();
        assert_eq!(restored.state_root().unwrap(), after_root);
        assert_eq!(restored, state);
    }

    #[test]
    fn settle_apply_changes_state_root_deterministically() {
        let (mut state, transaction, _, _) = settle_fixture();
        let before_root = state.state_root().unwrap();
        let plan =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 110)
                .unwrap();

        apply_compute_channel_settle_v1(&mut state, &plan).unwrap();
        let after_root = state.state_root().unwrap();

        assert_ne!(after_root, before_root);
        let restored =
            NativeStateV2::from_canonical_bytes(&state.canonical_bytes().unwrap()).unwrap();
        assert_eq!(restored.state_root().unwrap(), after_root);
        assert_eq!(restored, state);
    }

    #[test]
    fn refund_apply_changes_state_root_deterministically() {
        let (mut state, transaction, _) = refund_fixture();
        let before_root = state.state_root().unwrap();
        let plan =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap();

        apply_compute_channel_refund_v1(&mut state, &plan).unwrap();
        let after_root = state.state_root().unwrap();

        assert_ne!(after_root, before_root);
        let restored =
            NativeStateV2::from_canonical_bytes(&state.canonical_bytes().unwrap()).unwrap();
        assert_eq!(restored.state_root().unwrap(), after_root);
        assert_eq!(restored, state);
    }

    #[test]
    fn settle_apply_rejects_stale_and_duplicate_terminal_plan() {
        let (mut state, transaction, _, _) = settle_fixture();
        let plan =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 110)
                .unwrap();

        apply_compute_channel_settle_v1(&mut state, &plan).unwrap();
        let settled = state.clone();

        let error = apply_compute_channel_settle_v1(&mut state, &plan).unwrap_err();
        assert!(error.contains("stale"));
        assert_eq!(state, settled);

        let error =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 110)
                .unwrap_err();
        assert!(error.contains("not OPEN"));
    }

    #[test]
    fn refund_apply_rejects_stale_and_duplicate_terminal_plan() {
        let (mut state, transaction, _) = refund_fixture();
        let plan =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap();

        apply_compute_channel_refund_v1(&mut state, &plan).unwrap();
        let refunded = state.clone();

        let error = apply_compute_channel_refund_v1(&mut state, &plan).unwrap_err();
        assert!(error.contains("stale"));
        assert_eq!(state, refunded);

        let error =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap_err();
        assert!(error.contains("not OPEN"));
    }

    #[test]
    fn inactive_dispatcher_executes_open_and_reports_exact_roots() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 3, 1_000, 100);
        let mut state = state_for(&transaction, 5_000, 3);
        let before = state.clone();
        let expected_before_root = before.state_root().unwrap();

        let result = execute_inactive_compute_transaction_v2(
            &mut state,
            &transaction,
            AddressNetwork::Devnet,
            50,
        )
        .unwrap();

        assert_eq!(result.state_root_before, expected_before_root);
        assert_eq!(result.state_root_after, state.state_root().unwrap());
        assert_ne!(result.state_root_after, result.state_root_before);
        assert_eq!(
            result.transition.transaction_id(),
            transaction.tx_id().unwrap()
        );

        let channel_id = result.transition.channel_id();
        assert!(state.channel(channel_id).is_some());
        let sender = transaction
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        assert_eq!(
            state.accounts().account(sender),
            AccountStateV1 {
                balance: 4_000,
                nonce: 4,
            }
        );
    }

    #[test]
    fn inactive_dispatcher_executes_settle_and_refund_variants() {
        let (mut settle_state, settle_transaction, _, _) = settle_fixture();
        let settle_result = execute_inactive_compute_transaction_v2(
            &mut settle_state,
            &settle_transaction,
            AddressNetwork::Devnet,
            110,
        )
        .unwrap();
        assert!(matches!(
            &settle_result.transition,
            AppliedComputeTransitionV1::Settle(_)
        ));
        assert_eq!(
            settle_state
                .channel(settle_result.transition.channel_id())
                .unwrap()
                .state,
            ComputeChannelStatusV1::Settled
        );

        let (mut refund_state, refund_transaction, _) = refund_fixture();
        let refund_result = execute_inactive_compute_transaction_v2(
            &mut refund_state,
            &refund_transaction,
            AddressNetwork::Devnet,
            121,
        )
        .unwrap();
        assert!(matches!(
            &refund_result.transition,
            AppliedComputeTransitionV1::Refund(_)
        ));
        assert_eq!(
            refund_state
                .channel(refund_result.transition.channel_id())
                .unwrap()
                .state,
            ComputeChannelStatusV1::Refunded
        );
    }

    #[test]
    fn inactive_dispatcher_rejects_noncompute_v2_action_without_mutation() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let mut transaction = signed_open(&signing_key, 3, 1_000, 100);
        transaction.body.action = NativeActionV2::Transfer;
        transaction.body.data.clear();
        transaction.body.value = 0;
        resign_v2(&mut transaction, &signing_key);

        let mut state = state_for(&transaction, 5_000, 3);
        let before = state.clone();

        let error = execute_inactive_compute_transaction_v2(
            &mut state,
            &transaction,
            AddressNetwork::Devnet,
            50,
        )
        .unwrap_err();

        assert!(error.contains("not an inactive ComputeChannel action"));
        assert_eq!(state, before);
    }

    #[test]
    fn inactive_dispatcher_rolls_back_failed_compute_validation() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let mut transaction = signed_open(&signing_key, 3, 1_000, 100);
        let mut state = state_for(&transaction, 5_000, 3);
        let before = state.clone();

        transaction.signature[0] ^= 1;
        assert!(execute_inactive_compute_transaction_v2(
            &mut state,
            &transaction,
            AddressNetwork::Devnet,
            50,
        )
        .is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn inactive_batch_commits_all_transactions_atomically() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let first = signed_open(&signing_key, 3, 1_000, 100);
        let second = signed_open(&signing_key, 4, 1_000, 101);
        let mut state = state_for(&first, 5_000, 3);
        let before_root = state.state_root().unwrap();

        let result = execute_inactive_compute_batch_v2(
            &mut state,
            &[first.clone(), second.clone()],
            AddressNetwork::Devnet,
            50,
        )
        .unwrap();

        assert_eq!(result.transactions.len(), 2);
        assert_eq!(result.state_root_before, before_root);
        assert_eq!(
            result.transactions[0].state_root_before,
            result.state_root_before
        );
        assert_eq!(
            result.transactions[0].state_root_after,
            result.transactions[1].state_root_before
        );
        assert_eq!(
            result.transactions[1].state_root_after,
            result.state_root_after
        );
        assert_eq!(result.state_root_after, state.state_root().unwrap());

        let sender = first
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        assert_eq!(
            state.accounts().account(sender),
            AccountStateV1 {
                balance: 3_000,
                nonce: 5,
            }
        );
        assert!(state
            .channel(result.transactions[0].transition.channel_id())
            .is_some());
        assert!(state
            .channel(result.transactions[1].transition.channel_id())
            .is_some());
    }

    #[test]
    fn inactive_batch_rolls_back_earlier_transaction_when_later_one_fails() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let first = signed_open(&signing_key, 3, 1_000, 100);
        let second = signed_open(&signing_key, 5, 1_000, 101);
        let mut state = state_for(&first, 5_000, 3);
        let before = state.clone();

        let error = execute_inactive_compute_batch_v2(
            &mut state,
            &[first, second],
            AddressNetwork::Devnet,
            50,
        )
        .unwrap_err();

        assert!(error.contains("nonce mismatch"));
        assert_eq!(state, before);
    }

    #[test]
    fn inactive_empty_batch_is_a_deterministic_noop() {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let transaction = signed_open(&signing_key, 3, 1_000, 100);
        let mut state = state_for(&transaction, 5_000, 3);
        let before = state.clone();

        let result =
            execute_inactive_compute_batch_v2(&mut state, &[], AddressNetwork::Devnet, 50).unwrap();

        assert!(result.transactions.is_empty());
        assert_eq!(result.state_root_before, result.state_root_after);
        assert_eq!(state, before);
    }

    #[test]
    fn settle_plan_requires_worker_payment_account_sender() {
        let (state, mut transaction, _, _) = settle_fixture();
        let other_key = SigningKey::from_slice(&[0x32; 32]).unwrap();
        transaction.public_key = other_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        resign_v2(&mut transaction, &other_key);

        let error =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 110)
                .unwrap_err();
        assert!(error.contains("worker_payment_account"));
    }

    #[test]
    fn settle_plan_rejects_claim_deadline_and_receipt_expiry() {
        let (state, transaction, worker_key, _) = settle_fixture();

        let error =
            plan_compute_channel_settle_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap_err();
        assert!(error.contains("claim deadline"));

        let payload =
            ComputeChannelSettlePayloadV1::from_canonical_bytes(&transaction.body.data).unwrap();
        let mut receipt =
            ComputeUsageReceiptV1::from_canonical_bytes(&payload.final_usage_receipt).unwrap();
        receipt.expires_at = 109;

        let channel_key = SigningKey::from_slice(&[0x41; 32]).unwrap();
        let digest = receipt.signing_digest(AddressNetwork::Devnet).unwrap();
        let signature: Signature = channel_key.sign_prehash(&digest).unwrap();
        receipt.channel_signature = signature.to_bytes().to_vec();

        let mut expired_transaction = transaction;
        expired_transaction.body.data = ComputeChannelSettlePayloadV1 {
            channel_id: payload.channel_id,
            final_usage_receipt: receipt.canonical_bytes().unwrap(),
        }
        .canonical_bytes()
        .unwrap();
        resign_v2(&mut expired_transaction, &worker_key);

        let error = plan_compute_channel_settle_v1(
            &state,
            &expired_transaction,
            AddressNetwork::Devnet,
            110,
        )
        .unwrap_err();
        assert!(error.contains("receipt is expired"));
    }

    #[test]
    fn settle_plan_rejects_invalid_receipt_signature_and_identity() {
        let (state, transaction, worker_key, _) = settle_fixture();
        let payload =
            ComputeChannelSettlePayloadV1::from_canonical_bytes(&transaction.body.data).unwrap();
        let mut receipt =
            ComputeUsageReceiptV1::from_canonical_bytes(&payload.final_usage_receipt).unwrap();

        receipt.worker_id = [0xfe; 32];
        let channel_key = SigningKey::from_slice(&[0x41; 32]).unwrap();
        let digest = receipt.signing_digest(AddressNetwork::Devnet).unwrap();
        let signature: Signature = channel_key.sign_prehash(&digest).unwrap();
        receipt.channel_signature = signature.to_bytes().to_vec();

        let mut identity_transaction = transaction.clone();
        identity_transaction.body.data = ComputeChannelSettlePayloadV1 {
            channel_id: payload.channel_id,
            final_usage_receipt: receipt.canonical_bytes().unwrap(),
        }
        .canonical_bytes()
        .unwrap();
        resign_v2(&mut identity_transaction, &worker_key);

        let error = plan_compute_channel_settle_v1(
            &state,
            &identity_transaction,
            AddressNetwork::Devnet,
            110,
        )
        .unwrap_err();
        assert!(error.contains("worker_id mismatch"));

        let mut tampered_payload =
            ComputeChannelSettlePayloadV1::from_canonical_bytes(&transaction.body.data).unwrap();
        let mut tampered_receipt =
            ComputeUsageReceiptV1::from_canonical_bytes(&tampered_payload.final_usage_receipt)
                .unwrap();
        tampered_receipt.channel_signature[0] ^= 1;
        tampered_payload.final_usage_receipt = tampered_receipt.canonical_bytes().unwrap();

        let mut tampered_transaction = transaction;
        tampered_transaction.body.data = tampered_payload.canonical_bytes().unwrap();
        resign_v2(&mut tampered_transaction, &worker_key);

        assert!(plan_compute_channel_settle_v1(
            &state,
            &tampered_transaction,
            AddressNetwork::Devnet,
            110
        )
        .is_err());
    }

    #[test]
    fn settle_plan_rejects_overspend_and_nonzero_transaction_value() {
        let (state, transaction, worker_key, _) = settle_fixture();
        let payload =
            ComputeChannelSettlePayloadV1::from_canonical_bytes(&transaction.body.data).unwrap();
        let mut receipt =
            ComputeUsageReceiptV1::from_canonical_bytes(&payload.final_usage_receipt).unwrap();
        receipt.cumulative_spent = 1_001;

        let channel_key = SigningKey::from_slice(&[0x41; 32]).unwrap();
        let digest = receipt.signing_digest(AddressNetwork::Devnet).unwrap();
        let signature: Signature = channel_key.sign_prehash(&digest).unwrap();
        receipt.channel_signature = signature.to_bytes().to_vec();

        let mut overspend_transaction = transaction.clone();
        overspend_transaction.body.data = ComputeChannelSettlePayloadV1 {
            channel_id: payload.channel_id,
            final_usage_receipt: receipt.canonical_bytes().unwrap(),
        }
        .canonical_bytes()
        .unwrap();
        resign_v2(&mut overspend_transaction, &worker_key);

        let error = plan_compute_channel_settle_v1(
            &state,
            &overspend_transaction,
            AddressNetwork::Devnet,
            110,
        )
        .unwrap_err();
        assert!(error.contains("exceeds channel authorization"));

        let mut value_transaction = transaction;
        value_transaction.body.value = 1;
        resign_v2(&mut value_transaction, &worker_key);
        let error =
            plan_compute_channel_settle_v1(&state, &value_transaction, AddressNetwork::Devnet, 110)
                .unwrap_err();
        assert!(error.contains("value must be zero"));
    }

    #[test]
    fn refund_plan_derives_full_refund_without_mutating_state() {
        let (state, transaction, _) = refund_fixture();
        let before = state.clone();

        let plan =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap();

        assert_eq!(state, before);
        assert_eq!(plan.channel_id, [0xb1; 32]);
        assert_eq!(plan.expected_nonce, 4);
        assert_eq!(plan.refund_amount, 1_000);
        assert_eq!(plan.channel_state_after.settled_amount, 0);
        assert_eq!(
            plan.channel_state_after.state,
            ComputeChannelStatusV1::Refunded
        );
    }

    #[test]
    fn refund_plan_rejects_before_refund_height_and_nonzero_value() {
        let (state, transaction, funding_key) = refund_fixture();

        let error =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 120)
                .unwrap_err();
        assert!(error.contains("before the refund available height"));

        let mut value_transaction = transaction;
        value_transaction.body.value = 1;
        resign_v2(&mut value_transaction, &funding_key);
        let error =
            plan_compute_channel_refund_v1(&state, &value_transaction, AddressNetwork::Devnet, 121)
                .unwrap_err();
        assert!(error.contains("value must be zero"));
    }

    #[test]
    fn refund_plan_requires_funding_account_and_matching_nonce() {
        let (state, transaction, funding_key) = refund_fixture();

        let other_key = SigningKey::from_slice(&[0x53; 32]).unwrap();
        let mut wrong_sender = transaction.clone();
        wrong_sender.public_key = other_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        resign_v2(&mut wrong_sender, &other_key);
        let error =
            plan_compute_channel_refund_v1(&state, &wrong_sender, AddressNetwork::Devnet, 121)
                .unwrap_err();
        assert!(error.contains("funding_account"));

        let mut wrong_nonce = transaction;
        wrong_nonce.body.nonce = 5;
        resign_v2(&mut wrong_nonce, &funding_key);
        let error =
            plan_compute_channel_refund_v1(&state, &wrong_nonce, AddressNetwork::Devnet, 121)
                .unwrap_err();
        assert!(error.contains("nonce mismatch"));
    }

    #[test]
    fn refund_plan_rejects_closed_channel() {
        let (mut state, transaction, _) = refund_fixture();
        let mut channel = state.channel([0xb1; 32]).unwrap().clone();
        channel.state = ComputeChannelStatusV1::Settled;
        state.set_channel(channel).unwrap();

        let error =
            plan_compute_channel_refund_v1(&state, &transaction, AddressNetwork::Devnet, 121)
                .unwrap_err();
        assert!(error.contains("not OPEN"));
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
