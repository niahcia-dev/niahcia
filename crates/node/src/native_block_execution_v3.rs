use crate::address::AddressNetwork;
use crate::native_block_body_v2::{NativeBlockBodyV2, VersionedSignedNativeTransaction};
use crate::native_compute_fee_v1::{
    execute_inactive_compute_transaction_with_fee_v2, InactiveComputeFeeExecutionResultV1,
};
use crate::native_contract_execution_v1::{
    execute_inactive_accepted_contract_call_v1, execute_inactive_accepted_contract_create_v1,
    InactiveAcceptedContractCallV1, InactiveAcceptedContractCreateV1,
};
use crate::native_contract_runtime_registry_v1::NativeContractRuntimeRegistryV1;
use crate::native_execution::{
    execute_transfer_v1, NativeExecutionContextV1, NativeTransferOutcomeV1,
};
use crate::native_execution_v2::{execute_transfer_v2, NativeTransferOutcomeV2};
use crate::native_state_v3::NativeStateV3;
use crate::native_transaction::NativeActionV1;
use crate::native_transaction_v2::NativeActionV2;
use crate::work::Hash32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InactiveVersionedTransactionTransitionV3 {
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
    Compute {
        result: Box<InactiveComputeFeeExecutionResultV1>,
        state_root_before: Hash32,
        state_root_after: Hash32,
    },
    ContractCreate(InactiveAcceptedContractCreateV1),
    ContractCall(InactiveAcceptedContractCallV1),
}

impl InactiveVersionedTransactionTransitionV3 {
    pub fn transaction_id(&self) -> Hash32 {
        match self {
            Self::V1Transfer { transaction_id, .. } | Self::V2Transfer { transaction_id, .. } => {
                *transaction_id
            }
            Self::Compute { result, .. } => result.execution.transition.transaction_id(),
            Self::ContractCreate(result) => result.transaction_id,
            Self::ContractCall(result) => result.transaction_id,
        }
    }
    pub fn state_root_before(&self) -> Hash32 {
        match self {
            Self::V1Transfer {
                state_root_before, ..
            }
            | Self::V2Transfer {
                state_root_before, ..
            }
            | Self::Compute {
                state_root_before, ..
            } => *state_root_before,
            Self::ContractCreate(result) => result.state_root_before,
            Self::ContractCall(result) => result.state_root_before,
        }
    }
    pub fn state_root_after(&self) -> Hash32 {
        match self {
            Self::V1Transfer {
                state_root_after, ..
            }
            | Self::V2Transfer {
                state_root_after, ..
            }
            | Self::Compute {
                state_root_after, ..
            } => *state_root_after,
            Self::ContractCreate(result) => result.state_root_after,
            Self::ContractCall(result) => result.state_root_after,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveVersionedBlockTransitionV3 {
    pub transactions_root: Hash32,
    pub state_root_before: Hash32,
    pub state_root_after: Hash32,
    pub producer_priority_fee: u128,
    pub transactions: Vec<InactiveVersionedTransactionTransitionV3>,
}

pub fn execute_inactive_versioned_block_v3(
    state: &mut NativeStateV3,
    registry: &NativeContractRuntimeRegistryV1,
    body: &NativeBlockBodyV2,
    network: AddressNetwork,
    current_height: u64,
    base_fee_per_gas: u128,
) -> Result<InactiveVersionedBlockTransitionV3, String> {
    let state_root_before = state.state_root()?;
    let mut next = state.clone();
    let mut transitions = Vec::with_capacity(body.transactions.len());
    let mut producer_priority_fee = 0u128;

    for transaction in body.decoded_transactions()? {
        let before = next.state_root()?;
        let context = NativeExecutionContextV1 {
            base_fee_per_gas,
            cpu_producer: body.producer_fee_recipient,
        };
        let transition = match transaction {
            VersionedSignedNativeTransaction::V1(transaction) => {
                if transaction.body.action != NativeActionV1::Transfer {
                    return Err(
                        "inactive V3 block executor does not execute V1 contract actions".into(),
                    );
                }
                let outcome = execute_transfer_v1(
                    next.base_mut().accounts_mut(),
                    &transaction,
                    network,
                    context,
                )?;
                producer_priority_fee = producer_priority_fee
                    .checked_add(outcome.producer_priority_fee)
                    .ok_or_else(|| {
                        "inactive V3 block producer priority fee overflow".to_string()
                    })?;
                InactiveVersionedTransactionTransitionV3::V1Transfer {
                    transaction_id: transaction.tx_id()?,
                    state_root_before: before,
                    state_root_after: next.state_root()?,
                    outcome,
                }
            }
            VersionedSignedNativeTransaction::V2(transaction) => match transaction.body.action {
                NativeActionV2::Transfer => {
                    let outcome =
                        execute_transfer_v2(next.base_mut(), &transaction, network, context)?;
                    producer_priority_fee = producer_priority_fee
                        .checked_add(outcome.producer_priority_fee)
                        .ok_or_else(|| {
                            "inactive V3 block producer priority fee overflow".to_string()
                        })?;
                    InactiveVersionedTransactionTransitionV3::V2Transfer {
                        transaction_id: transaction.tx_id()?,
                        state_root_before: before,
                        state_root_after: next.state_root()?,
                        outcome,
                    }
                }
                NativeActionV2::ComputeChannelOpen
                | NativeActionV2::ComputeChannelSettle
                | NativeActionV2::ComputeChannelRefund => {
                    let result = execute_inactive_compute_transaction_with_fee_v2(
                        next.base_mut(),
                        &transaction,
                        network,
                        current_height,
                        context,
                    )?;
                    producer_priority_fee = producer_priority_fee
                        .checked_add(result.fee.producer_priority_fee)
                        .ok_or_else(|| {
                            "inactive V3 block producer priority fee overflow".to_string()
                        })?;
                    let state_root_after = next.state_root()?;
                    InactiveVersionedTransactionTransitionV3::Compute {
                        result: Box::new(result),
                        state_root_before: before,
                        state_root_after,
                    }
                }
                NativeActionV2::ContractCreate => {
                    let result = execute_inactive_accepted_contract_create_v1(
                        &mut next,
                        registry,
                        &transaction,
                        network,
                        current_height,
                        context,
                    )?;
                    producer_priority_fee = producer_priority_fee
                        .checked_add(result.producer_priority_fee)
                        .ok_or_else(|| {
                            "inactive V3 block producer priority fee overflow".to_string()
                        })?;
                    InactiveVersionedTransactionTransitionV3::ContractCreate(result)
                }
                NativeActionV2::ContractCall => {
                    let result = execute_inactive_accepted_contract_call_v1(
                        &mut next,
                        registry,
                        &transaction,
                        network,
                        current_height,
                        context,
                    )?;
                    producer_priority_fee = producer_priority_fee
                        .checked_add(result.producer_priority_fee)
                        .ok_or_else(|| {
                            "inactive V3 block producer priority fee overflow".to_string()
                        })?;
                    InactiveVersionedTransactionTransitionV3::ContractCall(result)
                }
            },
        };
        if transition.state_root_before() != before {
            return Err("inactive V3 block transition root discontinuity".into());
        }
        transitions.push(transition);
    }

    body.validate_fee_recipient_canonicality(producer_priority_fee)?;
    for window in transitions.windows(2) {
        if window[0].state_root_after() != window[1].state_root_before() {
            return Err("inactive V3 block transition roots are not contiguous".into());
        }
    }
    let state_root_after = next.state_root()?;
    if let Some(last) = transitions.last() {
        if last.state_root_after() != state_root_after {
            return Err("inactive V3 block final state root mismatch".into());
        }
    } else if state_root_after != state_root_before {
        return Err("inactive empty V3 block changed state".into());
    }

    let result = InactiveVersionedBlockTransitionV3 {
        transactions_root: body.transactions_root(),
        state_root_before,
        state_root_after,
        producer_priority_fee,
        transactions: transitions,
    };
    *state = next;
    Ok(result)
}
