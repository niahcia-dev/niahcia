use crate::address::AddressNetwork;
use crate::native_block_body_v2::{NativeBlockBodyV2, VersionedSignedNativeTransaction};
use crate::native_compute_execution::{
    execute_inactive_compute_transaction_v2, InactiveComputeExecutionResultV1,
};
use crate::native_execution::{
    execute_transfer_v1, NativeExecutionContextV1, NativeTransferOutcomeV1,
};
use crate::native_execution_v2::{execute_transfer_v2, NativeTransferOutcomeV2};
use crate::native_state_v2::NativeStateV2;
use crate::native_transaction::NativeActionV1;
use crate::native_transaction_v2::NativeActionV2;
use crate::work::Hash32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InactiveVersionedTransactionTransitionV2 {
    V1Transfer {
        transaction_id: Hash32,
        state_root_before: Hash32,
        state_root_after: Hash32,
        outcome: NativeTransferOutcomeV1,
    },
    V2Transfer {
        transaction_id: Hash32,
        state_root_before: Hash32,
        state_root_after: Hash32,
        outcome: NativeTransferOutcomeV2,
    },
    Compute(Box<InactiveComputeExecutionResultV1>),
}

impl InactiveVersionedTransactionTransitionV2 {
    pub fn transaction_id(&self) -> Hash32 {
        match self {
            Self::V1Transfer { transaction_id, .. } | Self::V2Transfer { transaction_id, .. } => {
                *transaction_id
            }
            Self::Compute(result) => result.transition.transaction_id(),
        }
    }

    pub fn state_root_before(&self) -> Hash32 {
        match self {
            Self::V1Transfer {
                state_root_before, ..
            }
            | Self::V2Transfer {
                state_root_before, ..
            } => *state_root_before,
            Self::Compute(result) => result.state_root_before,
        }
    }

    pub fn state_root_after(&self) -> Hash32 {
        match self {
            Self::V1Transfer {
                state_root_after, ..
            }
            | Self::V2Transfer {
                state_root_after, ..
            } => *state_root_after,
            Self::Compute(result) => result.state_root_after,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveVersionedBlockTransitionV2 {
    pub transactions_root: Hash32,
    pub state_root_before: Hash32,
    pub state_root_after: Hash32,
    pub producer_priority_fee: u128,
    pub transactions: Vec<InactiveVersionedTransactionTransitionV2>,
}

/// Executes a NativeBlockBodyV2 against NativeStateV2 as an inactive transition
/// proof. This is not the final V2 execution commitment and is deliberately
/// disconnected from active mempool/P2P/mining.
///
/// V1/V2 Transfer retain the active V1 fee rules. Compute-channel actions are
/// accepted only with zero placeholder gas/fee fields until compute intrinsic
/// gas is explicitly specified. ContractCall/ContractCreate remain rejected
/// until the native smart-contract runtime is specified and activated.
pub fn execute_inactive_versioned_block_v2(
    state: &mut NativeStateV2,
    body: &NativeBlockBodyV2,
    network: AddressNetwork,
    current_height: u64,
    base_fee_per_gas: u128,
) -> Result<InactiveVersionedBlockTransitionV2, String> {
    let state_root_before = state.state_root()?;
    let mut next = state.clone();
    let mut transitions = Vec::with_capacity(body.transactions.len());
    let mut producer_priority_fee = 0u128;

    for transaction in body.decoded_transactions()? {
        let before = next.state_root()?;

        let transition = match transaction {
            VersionedSignedNativeTransaction::V1(transaction) => {
                if transaction.body.action != NativeActionV1::Transfer {
                    return Err(
                        "inactive V2 block executor does not yet execute V1 contract actions"
                            .into(),
                    );
                }

                let outcome = execute_transfer_v1(
                    next.accounts_mut(),
                    &transaction,
                    network,
                    NativeExecutionContextV1 {
                        base_fee_per_gas,
                        cpu_producer: body.producer_fee_recipient,
                    },
                )?;
                producer_priority_fee = producer_priority_fee
                    .checked_add(outcome.producer_priority_fee)
                    .ok_or_else(|| {
                        "inactive V2 block producer priority fee overflow".to_string()
                    })?;

                InactiveVersionedTransactionTransitionV2::V1Transfer {
                    transaction_id: transaction.tx_id()?,
                    state_root_before: before,
                    state_root_after: next.state_root()?,
                    outcome,
                }
            }
            VersionedSignedNativeTransaction::V2(transaction) => {
                match transaction.body.action {
                    NativeActionV2::Transfer => {
                        let outcome = execute_transfer_v2(
                            &mut next,
                            &transaction,
                            network,
                            NativeExecutionContextV1 {
                                base_fee_per_gas,
                                cpu_producer: body.producer_fee_recipient,
                            },
                        )?;
                        producer_priority_fee = producer_priority_fee
                            .checked_add(outcome.producer_priority_fee)
                            .ok_or_else(|| {
                                "inactive V2 block producer priority fee overflow".to_string()
                            })?;

                        InactiveVersionedTransactionTransitionV2::V2Transfer {
                            transaction_id: transaction.tx_id()?,
                            state_root_before: before,
                            state_root_after: next.state_root()?,
                            outcome,
                        }
                    }
                    NativeActionV2::ComputeChannelOpen
                    | NativeActionV2::ComputeChannelSettle
                    | NativeActionV2::ComputeChannelRefund => {
                        if transaction.body.gas_limit != 0
                            || transaction.body.max_fee_per_gas != 0
                            || transaction.body.max_priority_fee_per_gas != 0
                        {
                            return Err(
                                "inactive compute execution requires zero placeholder gas/fee fields until the compute gas schedule is specified"
                                    .into(),
                            );
                        }

                        InactiveVersionedTransactionTransitionV2::Compute(Box::new(
                            execute_inactive_compute_transaction_v2(
                                &mut next,
                                &transaction,
                                network,
                                current_height,
                            )?,
                        ))
                    }
                    NativeActionV2::ContractCall | NativeActionV2::ContractCreate => return Err(
                        "inactive V2 block executor does not yet execute smart-contract actions"
                            .into(),
                    ),
                }
            }
        };

        if transition.state_root_before() != before {
            return Err("inactive V2 block transition root discontinuity".into());
        }
        transitions.push(transition);
    }

    body.validate_fee_recipient_canonicality(producer_priority_fee)?;

    for window in transitions.windows(2) {
        if window[0].state_root_after() != window[1].state_root_before() {
            return Err("inactive V2 block transition roots are not contiguous".into());
        }
    }

    let state_root_after = next.state_root()?;
    if let Some(last) = transitions.last() {
        if last.state_root_after() != state_root_after {
            return Err("inactive V2 block final state root mismatch".into());
        }
    } else if state_root_after != state_root_before {
        return Err("inactive empty V2 block changed state".into());
    }

    let result = InactiveVersionedBlockTransitionV2 {
        transactions_root: body.transactions_root(),
        state_root_before,
        state_root_after,
        producer_priority_fee,
        transactions: transitions,
    };

    *state = next;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_block_body_v2::VersionedSignedNativeTransaction;
    use crate::native_compute_payloads::ComputeChannelOpenPayloadV1;
    use crate::native_execution::{AccountStateV1, NativeStateV1};
    use crate::native_state_v2::ComputeChannelSettlementPolicyV1;
    use crate::native_transaction::{
        NativeTransactionBodyV1, SignedNativeTransactionV1, DEVNET_CHAIN_ID, DEVNET_NETWORK_ID,
        NATIVE_TRANSFER_GAS_V1,
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

    #[test]
    fn mixed_v1_v2_transfers_execute_atomically_and_contiguously() {
        let key = SigningKey::from_slice(&[0x41; 32]).unwrap();
        let sender = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x51; 20],
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

        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            sender,
            AccountStateV1 {
                balance: 100_000,
                nonce: 0,
            },
        );
        let mut state = NativeStateV2::from_v1(accounts);

        let v1 = sign_v1(
            &key,
            NativeTransactionBodyV1 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV1::Transfer,
                target_payload: vec![0x61; 20],
                value: 100,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 1,
                data: Vec::new(),
            },
        );
        let v2 = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 1,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x62; 20],
                value: 200,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 1,
                data: Vec::new(),
            },
        );

        let body = NativeBlockBodyV2::from_versioned_transactions(
            [0x77; 20],
            &[
                VersionedSignedNativeTransaction::V1(v1),
                VersionedSignedNativeTransaction::V2(v2),
            ],
        )
        .unwrap();

        let result =
            execute_inactive_versioned_block_v2(&mut state, &body, AddressNetwork::Devnet, 10, 2)
                .unwrap();

        assert_eq!(result.transactions.len(), 2);
        assert_eq!(
            result.transactions[0].state_root_after(),
            result.transactions[1].state_root_before()
        );
        assert_eq!(result.state_root_after, state.state_root().unwrap());
        assert_eq!(state.accounts().account(sender).nonce, 2);
        assert_eq!(state.channel_count(), 0);
        assert_eq!(result.producer_priority_fee, 2_000);

        let snapshot = state.canonical_bytes().unwrap();
        let restored = NativeStateV2::from_canonical_bytes(&snapshot).unwrap();
        assert_eq!(restored, state);
        assert_eq!(restored.state_root().unwrap(), result.state_root_after);
    }

    #[test]
    fn later_failure_rolls_back_earlier_versioned_transfer() {
        let key = SigningKey::from_slice(&[0x42; 32]).unwrap();
        let first = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x63; 20],
                value: 100,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        );
        let sender = first
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;

        let second = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 9,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x64; 20],
                value: 100,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 5,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        );

        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            sender,
            AccountStateV1 {
                balance: 100_000,
                nonce: 0,
            },
        );
        let mut state = NativeStateV2::from_v1(accounts);
        let before = state.clone();

        let body = NativeBlockBodyV2::from_versioned_transactions(
            [0_u8; 20],
            &[
                VersionedSignedNativeTransaction::V2(first),
                VersionedSignedNativeTransaction::V2(second),
            ],
        )
        .unwrap();

        assert!(execute_inactive_versioned_block_v2(
            &mut state,
            &body,
            AddressNetwork::Devnet,
            10,
            2,
        )
        .is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn compute_open_can_follow_transfer_in_same_inactive_block() {
        let funding_key = SigningKey::from_slice(&[0x43; 32]).unwrap();
        let channel_key = SigningKey::from_slice(&[0x44; 32]).unwrap();
        let funding = sign_v2(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x65; 20],
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

        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            funding,
            AccountStateV1 {
                balance: 100_000,
                nonce: 0,
            },
        );
        let mut state = NativeStateV2::from_v1(accounts);

        let transfer = sign_v2(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x66; 20],
                value: 100,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 2,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        );

        let channel_public_key: [u8; 65] = channel_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .try_into()
            .unwrap();
        let open_payload = ComputeChannelOpenPayloadV1 {
            worker_id: [0x71; 32],
            operator_id: [0x72; 32],
            channel_public_key,
            worker_payment_account: [0x73; 20],
            authorized_amount: 1_000,
            expiry_height: 20,
            claim_deadline_height: 25,
            refund_available_height: 26,
            service_scope_commitment: [0x74; 32],
            model_scope_commitment: [0x75; 32],
            execution_profile_scope_commitment: [0x76; 32],
            settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
        };

        let open = sign_v2(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 1,
                action: NativeActionV2::ComputeChannelOpen,
                target_payload: Vec::new(),
                value: 1_000,
                gas_limit: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                data: open_payload.canonical_bytes().unwrap(),
            },
        );

        let body = NativeBlockBodyV2::from_versioned_transactions(
            [0_u8; 20],
            &[
                VersionedSignedNativeTransaction::V2(transfer),
                VersionedSignedNativeTransaction::V2(open),
            ],
        )
        .unwrap();

        let result =
            execute_inactive_versioned_block_v2(&mut state, &body, AddressNetwork::Devnet, 10, 2)
                .unwrap();

        assert_eq!(result.transactions.len(), 2);
        assert_eq!(
            result.transactions[0].state_root_after(),
            result.transactions[1].state_root_before()
        );
        assert_eq!(state.accounts().account(funding).nonce, 2);
        assert_eq!(state.channel_count(), 1);
    }

    #[test]
    fn smart_contract_action_remains_inactive_without_state_mutation() {
        let key = SigningKey::from_slice(&[0x47; 32]).unwrap();
        let transaction = sign_v2(
            &key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::ContractCall,
                target_payload: vec![0x91; 20],
                value: 0,
                gas_limit: 10_000,
                max_fee_per_gas: 1,
                max_priority_fee_per_gas: 0,
                data: vec![0x01],
            },
        );

        let mut state = NativeStateV2::default();
        let before = state.clone();
        let body = NativeBlockBodyV2::from_versioned_transactions(
            [0_u8; 20],
            &[VersionedSignedNativeTransaction::V2(transaction)],
        )
        .unwrap();

        let error =
            execute_inactive_versioned_block_v2(&mut state, &body, AddressNetwork::Devnet, 10, 0)
                .unwrap_err();

        assert!(error.contains("does not yet execute smart-contract actions"));
        assert_eq!(state, before);
    }

    #[test]
    fn compute_nonzero_placeholder_fee_fields_are_rejected_atomically() {
        let funding_key = SigningKey::from_slice(&[0x45; 32]).unwrap();
        let channel_key = SigningKey::from_slice(&[0x46; 32]).unwrap();
        let funding = sign_v2(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x67; 20],
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

        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            funding,
            AccountStateV1 {
                balance: 100_000,
                nonce: 0,
            },
        );
        let mut state = NativeStateV2::from_v1(accounts);
        let before = state.clone();

        let channel_public_key: [u8; 65] = channel_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .try_into()
            .unwrap();
        let open_payload = ComputeChannelOpenPayloadV1 {
            worker_id: [0x81; 32],
            operator_id: [0x82; 32],
            channel_public_key,
            worker_payment_account: [0x83; 20],
            authorized_amount: 1_000,
            expiry_height: 20,
            claim_deadline_height: 25,
            refund_available_height: 26,
            service_scope_commitment: [0x84; 32],
            model_scope_commitment: [0x85; 32],
            execution_profile_scope_commitment: [0x86; 32],
            settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
        };

        let open = sign_v2(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::ComputeChannelOpen,
                target_payload: Vec::new(),
                value: 1_000,
                gas_limit: 1,
                max_fee_per_gas: 1,
                max_priority_fee_per_gas: 0,
                data: open_payload.canonical_bytes().unwrap(),
            },
        );

        let body = NativeBlockBodyV2::from_versioned_transactions(
            [0_u8; 20],
            &[VersionedSignedNativeTransaction::V2(open)],
        )
        .unwrap();

        let error =
            execute_inactive_versioned_block_v2(&mut state, &body, AddressNetwork::Devnet, 10, 2)
                .unwrap_err();

        assert!(error.contains("zero placeholder gas/fee"));
        assert_eq!(state, before);
    }
}
