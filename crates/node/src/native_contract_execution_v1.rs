use crate::address::{AddressNetwork, NiahciaAddressV1};
use crate::native_contract_payload_v1::ContractCreatePayloadV1;
use crate::native_contract_runtime_registry_v1::NativeContractRuntimeRegistryV1;
use crate::native_contract_state_v1::ContractStateV1;
use crate::native_contract_vm_v1::{
    execute_nvm1_core_with_context_and_gas, validate_nvm1_code, validate_nvm1_jump_targets,
    Nvm1ExecutionContext, Nvm1ExecutionResult, Nvm1Halt, NVM1_CODE_FORMAT_VERSION, NVM1_RUNTIME_ID,
};
use crate::native_execution::{AccountId, NativeExecutionContextV1};
use crate::native_state_v3::NativeStateV3;
use crate::native_transaction_v2::{NativeActionV2, SignedNativeTransactionV2};
use crate::work::Hash32;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ContractFeeAccountingV1 {
    max_execution_charge: u128,
    effective_fee_per_gas: u128,
    base_fee_burned: u128,
    producer_priority_fee: u128,
    actual_fee: u128,
    unused_fee_reserve: u128,
}

fn contract_fee_accounting_v1(
    gas_limit: u64,
    gas_used: u64,
    max_fee_per_gas: u128,
    max_priority_fee_per_gas: u128,
    base_fee_per_gas: u128,
    label: &str,
) -> Result<ContractFeeAccountingV1, String> {
    if max_fee_per_gas < base_fee_per_gas {
        return Err(format!(
            "native transaction V2 max_fee_per_gas {} is below base_fee_per_gas {}",
            max_fee_per_gas, base_fee_per_gas
        ));
    }
    let max_execution_charge = max_fee_per_gas
        .checked_mul(gas_limit as u128)
        .ok_or_else(|| format!("{label} maximum execution charge overflow"))?;
    let priority_headroom = max_fee_per_gas
        .checked_sub(base_fee_per_gas)
        .ok_or_else(|| format!("{label} priority fee headroom underflow"))?;
    let priority_fee_per_gas = max_priority_fee_per_gas.min(priority_headroom);
    let effective_fee_per_gas = base_fee_per_gas
        .checked_add(priority_fee_per_gas)
        .ok_or_else(|| format!("{label} effective fee overflow"))?;
    let base_fee_burned = base_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| format!("{label} base fee burn overflow"))?;
    let producer_priority_fee = priority_fee_per_gas
        .checked_mul(gas_used as u128)
        .ok_or_else(|| format!("{label} producer priority fee overflow"))?;
    let actual_fee = base_fee_burned
        .checked_add(producer_priority_fee)
        .ok_or_else(|| format!("{label} actual fee overflow"))?;
    let unused_fee_reserve = max_execution_charge
        .checked_sub(actual_fee)
        .ok_or_else(|| format!("{label} actual fee exceeds maximum reserve"))?;
    Ok(ContractFeeAccountingV1 {
        max_execution_charge,
        effective_fee_per_gas,
        base_fee_burned,
        producer_priority_fee,
        actual_fee,
        unused_fee_reserve,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InactiveContractCreateRequestV1 {
    pub network: AddressNetwork,
    pub chain_id: u64,
    pub current_height: u64,
    pub creator_payload: [u8; 20],
    pub creator_nonce: u64,
    pub value: u128,
    pub gas_limit: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveContractCreateTransitionV1 {
    pub creator_payload: [u8; 20],
    pub creator_nonce: u64,
    pub contract_id: [u8; 20],
    pub runtime_id: u32,
    pub value: u128,
    pub state_root_before: Hash32,
    pub state_root_after: Hash32,
    pub execution: Nvm1ExecutionResult,
    pub contract_created: bool,
}

pub fn execute_inactive_contract_create_transition_v1(
    state: &mut NativeStateV3,
    registry: &NativeContractRuntimeRegistryV1,
    payload: &ContractCreatePayloadV1,
    request: InactiveContractCreateRequestV1,
) -> Result<InactiveContractCreateTransitionV1, String> {
    let InactiveContractCreateRequestV1 {
        network,
        chain_id,
        current_height,
        creator_payload,
        creator_nonce,
        value,
        gas_limit,
    } = request;
    let descriptor = registry.validate_create_payload(current_height, payload)?;
    if descriptor.runtime_id != NVM1_RUNTIME_ID
        || descriptor.code_format_version != NVM1_CODE_FORMAT_VERSION
    {
        return Err(format!(
            "inactive ContractCreate executor supports only NVM1 runtime_id {} code_format_version {}",
            NVM1_RUNTIME_ID, NVM1_CODE_FORMAT_VERSION
        ));
    }

    validate_nvm1_code(&payload.code)?;
    validate_nvm1_jump_targets(&payload.code)?;

    let contract_address =
        NiahciaAddressV1::contract_from_creator(network, chain_id, creator_payload, creator_nonce);
    let contract_id = contract_address.payload;

    if state.contract(contract_id).is_some() {
        return Err("contract state already exists at derived contract id".into());
    }

    let state_root_before = state.state_root()?;
    let context = Nvm1ExecutionContext {
        input: payload.init_data.clone(),
        caller_payload: creator_payload,
        call_value: value,
        storage: BTreeMap::new(),
    };
    let execution = execute_nvm1_core_with_context_and_gas(&payload.code, &context, gas_limit)?;

    let mut next = state.clone();
    let contract_created = matches!(execution.halt, Nvm1Halt::Stop | Nvm1Halt::Return(_));

    if contract_created {
        let storage = execution.committed_storage.as_ref().ok_or_else(|| {
            "successful NVM1 create execution omitted committed storage".to_string()
        })?;

        let mut contract =
            ContractStateV1::new(contract_id, value, payload.runtime_id, payload.code.clone());
        for (key, stored_value) in storage {
            contract.set_storage(*key, *stored_value);
        }
        next.insert_contract(contract)?;
        *state = next;
    }

    let state_root_after = state.state_root()?;
    if !contract_created && state_root_after != state_root_before {
        return Err("failed ContractCreate transition mutated NativeStateV3".into());
    }

    Ok(InactiveContractCreateTransitionV1 {
        creator_payload,
        creator_nonce,
        contract_id,
        runtime_id: payload.runtime_id,
        value,
        state_root_before,
        state_root_after,
        execution,
        contract_created,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveAcceptedContractCreateV1 {
    pub transaction_id: Hash32,
    pub sender: AccountId,
    pub contract_id: [u8; 20],
    pub contract_created: bool,
    pub gas_used: u64,
    pub max_execution_charge: u128,
    pub effective_fee_per_gas: u128,
    pub base_fee_burned: u128,
    pub producer_priority_fee: u128,
    pub actual_fee: u128,
    pub unused_fee_reserve: u128,
    pub nonce_before: u64,
    pub nonce_after: u64,
    pub state_root_before: Hash32,
    pub state_root_after: Hash32,
    pub execution: Nvm1ExecutionResult,
}

/// Executes an accepted schema-V2 ContractCreate transaction against NativeStateV3.
///
/// Validation failures reject atomically before acceptance. Once accepted, the sender nonce is
/// consumed and execution gas is charged regardless of constructor success. Constructor state and
/// transferred value commit only for STOP/RETURN. REVERT preserves unused gas; a VM trap consumes
/// the full transaction gas limit (including out-of-gas) and commits no contract/value state.
pub fn execute_inactive_accepted_contract_create_v1(
    state: &mut NativeStateV3,
    registry: &NativeContractRuntimeRegistryV1,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    current_height: u64,
    context: NativeExecutionContextV1,
) -> Result<InactiveAcceptedContractCreateV1, String> {
    transaction.body.validate(network)?;
    if transaction.body.action != NativeActionV2::ContractCreate {
        return Err("accepted ContractCreate executor requires ContractCreate action".into());
    }
    let payload = ContractCreatePayloadV1::from_canonical_bytes(&transaction.body.data)?;
    let sender = transaction.authenticated_sender(network)?.payload;
    let sender_before = state.base().accounts().account(sender);
    if sender_before.nonce != transaction.body.nonce {
        return Err(format!(
            "native account nonce mismatch: expected {}, found {}",
            sender_before.nonce, transaction.body.nonce
        ));
    }
    let preflight_fee = contract_fee_accounting_v1(
        transaction.body.gas_limit,
        0,
        transaction.body.max_fee_per_gas,
        transaction.body.max_priority_fee_per_gas,
        context.base_fee_per_gas,
        "ContractCreate",
    )?;
    let max_execution_charge = preflight_fee.max_execution_charge;
    let required_balance = transaction
        .body
        .value
        .checked_add(max_execution_charge)
        .ok_or_else(|| "ContractCreate required balance overflow".to_string())?;
    if sender_before.balance < required_balance {
        return Err(format!(
            "insufficient native account balance: required {}, available {}",
            required_balance, sender_before.balance
        ));
    }

    // Run the constructor against a clone. Its contract/value changes are provisional until its
    // halt status is known; accepted transaction accounting is applied separately below.
    let state_root_before = state.state_root()?;
    let mut constructor_state = state.clone();
    let transition = execute_inactive_contract_create_transition_v1(
        &mut constructor_state,
        registry,
        &payload,
        InactiveContractCreateRequestV1 {
            network,
            chain_id: transaction.body.chain_id,
            current_height,
            creator_payload: sender,
            creator_nonce: transaction.body.nonce,
            value: transaction.body.value,
            gas_limit: transaction.body.gas_limit,
        },
    )?;

    let gas_used = if matches!(transition.execution.halt, Nvm1Halt::Trap(_)) {
        transaction.body.gas_limit
    } else {
        transition.execution.gas_used
    };
    let fee = contract_fee_accounting_v1(
        transaction.body.gas_limit,
        gas_used,
        transaction.body.max_fee_per_gas,
        transaction.body.max_priority_fee_per_gas,
        context.base_fee_per_gas,
        "ContractCreate",
    )?;
    let effective_fee_per_gas = fee.effective_fee_per_gas;
    let base_fee_burned = fee.base_fee_burned;
    let producer_priority_fee = fee.producer_priority_fee;
    let actual_fee = fee.actual_fee;
    let unused_fee_reserve = fee.unused_fee_reserve;

    let contract_created = transition.contract_created;
    let mut next = if contract_created {
        constructor_state
    } else {
        state.clone()
    };
    next.base_mut()
        .accounts_mut()
        .consume_nonce(sender, transaction.body.nonce)?;

    // Value moves only when creation succeeds. Failed constructors still pay their execution fee.
    let sender_debit = actual_fee
        .checked_add(if contract_created {
            transaction.body.value
        } else {
            0
        })
        .ok_or_else(|| "ContractCreate sender debit overflow".to_string())?;
    next.base_mut().accounts_mut().debit(sender, sender_debit)?;
    next.base_mut()
        .accounts_mut()
        .credit(context.cpu_producer, producer_priority_fee)?;

    let nonce_after = next.base().accounts().account(sender).nonce;
    let state_root_after = next.state_root()?;
    *state = next;

    Ok(InactiveAcceptedContractCreateV1 {
        transaction_id: transaction.tx_id()?,
        sender,
        contract_id: transition.contract_id,
        contract_created,
        gas_used,
        max_execution_charge,
        effective_fee_per_gas,
        base_fee_burned,
        producer_priority_fee,
        actual_fee,
        unused_fee_reserve,
        nonce_before: sender_before.nonce,
        nonce_after,
        state_root_before,
        state_root_after,
        execution: transition.execution,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InactiveAcceptedContractCallV1 {
    pub transaction_id: Hash32,
    pub sender: AccountId,
    pub contract_id: [u8; 20],
    pub call_succeeded: bool,
    pub gas_used: u64,
    pub max_execution_charge: u128,
    pub effective_fee_per_gas: u128,
    pub base_fee_burned: u128,
    pub producer_priority_fee: u128,
    pub actual_fee: u128,
    pub unused_fee_reserve: u128,
    pub nonce_before: u64,
    pub nonce_after: u64,
    pub state_root_before: Hash32,
    pub state_root_after: Hash32,
    pub execution: Nvm1ExecutionResult,
}

/// Executes an accepted schema-V2 ContractCall against an existing NVM1 contract.
///
/// Pre-execution validation errors reject atomically. Once accepted, nonce and gas effects persist.
/// Contract storage and call value commit only for STOP/RETURN; REVERT and traps discard both.
pub fn execute_inactive_accepted_contract_call_v1(
    state: &mut NativeStateV3,
    registry: &NativeContractRuntimeRegistryV1,
    transaction: &SignedNativeTransactionV2,
    network: AddressNetwork,
    current_height: u64,
    context: NativeExecutionContextV1,
) -> Result<InactiveAcceptedContractCallV1, String> {
    transaction.body.validate(network)?;
    if transaction.body.action != NativeActionV2::ContractCall {
        return Err("accepted ContractCall executor requires ContractCall action".into());
    }

    let sender = transaction.authenticated_sender(network)?.payload;
    let contract_id: [u8; 20] = transaction
        .body
        .target_payload
        .as_slice()
        .try_into()
        .map_err(|_| "ContractCall target must be exactly 20 bytes".to_string())?;
    let contract = state
        .contract(contract_id)
        .cloned()
        .ok_or_else(|| "ContractCall target contract does not exist".to_string())?;
    registry.active_descriptor(contract.runtime_id, current_height)?;
    if contract.runtime_id != NVM1_RUNTIME_ID {
        return Err(format!(
            "inactive ContractCall executor supports only NVM1 runtime_id {}",
            NVM1_RUNTIME_ID
        ));
    }
    validate_nvm1_code(&contract.code)?;
    validate_nvm1_jump_targets(&contract.code)?;

    let sender_before = state.base().accounts().account(sender);
    if sender_before.nonce != transaction.body.nonce {
        return Err(format!(
            "native account nonce mismatch: expected {}, found {}",
            sender_before.nonce, transaction.body.nonce
        ));
    }
    let preflight_fee = contract_fee_accounting_v1(
        transaction.body.gas_limit,
        0,
        transaction.body.max_fee_per_gas,
        transaction.body.max_priority_fee_per_gas,
        context.base_fee_per_gas,
        "ContractCall",
    )?;
    let max_execution_charge = preflight_fee.max_execution_charge;
    let required_balance = transaction
        .body
        .value
        .checked_add(max_execution_charge)
        .ok_or_else(|| "ContractCall required balance overflow".to_string())?;
    if sender_before.balance < required_balance {
        return Err(format!(
            "insufficient native account balance: required {}, available {}",
            required_balance, sender_before.balance
        ));
    }
    contract
        .balance
        .checked_add(transaction.body.value)
        .ok_or_else(|| "ContractCall target balance overflow".to_string())?;

    let state_root_before = state.state_root()?;
    let storage = contract
        .storage_entries()
        .map(|(key, value)| (*key, *value))
        .collect();
    let vm_context = Nvm1ExecutionContext {
        input: transaction.body.data.clone(),
        caller_payload: sender,
        call_value: transaction.body.value,
        storage,
    };
    let execution = execute_nvm1_core_with_context_and_gas(
        &contract.code,
        &vm_context,
        transaction.body.gas_limit,
    )?;
    let call_succeeded = matches!(execution.halt, Nvm1Halt::Stop | Nvm1Halt::Return(_));
    let gas_used = if matches!(execution.halt, Nvm1Halt::Trap(_)) {
        transaction.body.gas_limit
    } else {
        execution.gas_used
    };

    let fee = contract_fee_accounting_v1(
        transaction.body.gas_limit,
        gas_used,
        transaction.body.max_fee_per_gas,
        transaction.body.max_priority_fee_per_gas,
        context.base_fee_per_gas,
        "ContractCall",
    )?;
    let effective_fee_per_gas = fee.effective_fee_per_gas;
    let base_fee_burned = fee.base_fee_burned;
    let producer_priority_fee = fee.producer_priority_fee;
    let actual_fee = fee.actual_fee;
    let unused_fee_reserve = fee.unused_fee_reserve;

    let mut next = state.clone();
    if call_succeeded {
        let committed = execution
            .committed_storage
            .as_ref()
            .ok_or_else(|| "successful NVM1 call omitted committed storage".to_string())?;
        let target = next
            .contract_mut(contract_id)
            .ok_or_else(|| "ContractCall target disappeared during execution".to_string())?;
        target.credit(transaction.body.value)?;
        let old_keys: Vec<_> = target.storage_entries().map(|(key, _)| *key).collect();
        for key in old_keys {
            target.remove_storage(key);
        }
        for (key, value) in committed {
            target.set_storage(*key, *value);
        }
    }

    next.base_mut()
        .accounts_mut()
        .consume_nonce(sender, transaction.body.nonce)?;
    let sender_debit = actual_fee
        .checked_add(if call_succeeded {
            transaction.body.value
        } else {
            0
        })
        .ok_or_else(|| "ContractCall sender debit overflow".to_string())?;
    next.base_mut().accounts_mut().debit(sender, sender_debit)?;
    next.base_mut()
        .accounts_mut()
        .credit(context.cpu_producer, producer_priority_fee)?;

    let nonce_after = next.base().accounts().account(sender).nonce;
    let state_root_after = next.state_root()?;
    *state = next;

    Ok(InactiveAcceptedContractCallV1 {
        transaction_id: transaction.tx_id()?,
        sender,
        contract_id,
        call_succeeded,
        gas_used,
        max_execution_charge,
        effective_fee_per_gas,
        base_fee_burned,
        producer_priority_fee,
        actual_fee,
        unused_fee_reserve,
        nonce_before: sender_before.nonce,
        nonce_after,
        state_root_before,
        state_root_after,
        execution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_contract_runtime_registry_v1::ContractRuntimeDescriptorV1;

    fn registry() -> NativeContractRuntimeRegistryV1 {
        NativeContractRuntimeRegistryV1::new([ContractRuntimeDescriptorV1 {
            runtime_id: NVM1_RUNTIME_ID,
            code_format_version: NVM1_CODE_FORMAT_VERSION,
            activation_height: 10,
            retirement_height: None,
            max_code_bytes: 65_536,
            max_init_data_bytes: 65_536,
        }])
        .unwrap()
    }

    fn module(instruction_count: u32, max_stack_items: u16, instructions: &[u8]) -> Vec<u8> {
        let mut code = Vec::with_capacity(16 + instructions.len());
        code.extend_from_slice(b"NVM1");
        code.extend_from_slice(&1u16.to_be_bytes());
        code.extend_from_slice(&0u16.to_be_bytes());
        code.extend_from_slice(&instruction_count.to_be_bytes());
        code.extend_from_slice(&max_stack_items.to_be_bytes());
        code.extend_from_slice(&0u16.to_be_bytes());
        code.extend_from_slice(instructions);
        code
    }

    fn payload(code: Vec<u8>, init_data: Vec<u8>) -> ContractCreatePayloadV1 {
        ContractCreatePayloadV1 {
            runtime_id: NVM1_RUNTIME_ID,
            code,
            init_data,
        }
    }

    use crate::native_execution::{AccountStateV1, NativeStateV1};
    use crate::native_state_v2::NativeStateV2;
    use crate::native_transaction::{DEVNET_CHAIN_ID, DEVNET_NETWORK_ID};
    use crate::native_transaction_v2::{NativeTransactionBodyV2, SignedNativeTransactionV2};
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn signed_create(
        key: &SigningKey,
        nonce: u64,
        value: u128,
        gas_limit: u64,
        max_fee_per_gas: u128,
        code: Vec<u8>,
    ) -> SignedNativeTransactionV2 {
        let create = payload(code, Vec::new());
        let body = NativeTransactionBodyV2 {
            network_id: DEVNET_NETWORK_ID,
            chain_id: DEVNET_CHAIN_ID,
            nonce,
            action: NativeActionV2::ContractCreate,
            target_payload: Vec::new(),
            value,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas: 2,
            data: create.canonical_bytes().unwrap(),
        };
        let mut tx = SignedNativeTransactionV2 {
            body,
            public_key: key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            signature: vec![0; 64],
        };
        let digest = tx.signing_digest().unwrap();
        let signature: Signature = key.sign_prehash(&digest).unwrap();
        tx.signature = signature.to_bytes().to_vec();
        tx
    }

    fn funded_v3(sender: AccountId, balance: u128, nonce: u64) -> NativeStateV3 {
        let mut accounts = NativeStateV1::default();
        accounts.set_account(sender, AccountStateV1 { balance, nonce });
        NativeStateV3::from_v2(NativeStateV2::from_v1(accounts))
    }

    fn signed_call(
        key: &SigningKey,
        nonce: u64,
        contract_id: [u8; 20],
        value: u128,
        gas_limit: u64,
        max_fee_per_gas: u128,
        data: Vec<u8>,
    ) -> SignedNativeTransactionV2 {
        let body = NativeTransactionBodyV2 {
            network_id: DEVNET_NETWORK_ID,
            chain_id: DEVNET_CHAIN_ID,
            nonce,
            action: NativeActionV2::ContractCall,
            target_payload: contract_id.to_vec(),
            value,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas: 2,
            data,
        };
        let mut tx = SignedNativeTransactionV2 {
            body,
            public_key: key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            signature: vec![0; 64],
        };
        let digest = tx.signing_digest().unwrap();
        let signature: Signature = key.sign_prehash(&digest).unwrap();
        tx.signature = signature.to_bytes().to_vec();
        tx
    }

    fn insert_contract(state: &mut NativeStateV3, id: [u8; 20], balance: u128, code: Vec<u8>) {
        state
            .insert_contract(ContractStateV1::new(id, balance, NVM1_RUNTIME_ID, code))
            .unwrap();
    }

    #[test]
    fn accepted_call_success_commits_value_storage_and_exact_fee() {
        let key = SigningKey::from_slice(&[0x31; 32]).unwrap();
        let sender_tx = signed_call(&key, 0, [0x71; 20], 500, 205, 5, vec![0xaa]);
        let sender = sender_tx
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let producer = [0x93; 20];
        let contract_id = [0x71; 20];
        let storage_key = [0x44; 32];
        let storage_value = [0x55; 32];
        let mut instructions = vec![0x02];
        instructions.extend_from_slice(&storage_key);
        instructions.push(0x02);
        instructions.extend_from_slice(&storage_value);
        instructions.push(0x21);
        instructions.push(0x00);
        let mut state = funded_v3(sender, 10_000, 0);
        insert_contract(&mut state, contract_id, 100, module(4, 2, &instructions));

        let result = execute_inactive_accepted_contract_call_v1(
            &mut state,
            &registry(),
            &sender_tx,
            AddressNetwork::Devnet,
            10,
            NativeExecutionContextV1 {
                base_fee_per_gas: 2,
                cpu_producer: producer,
            },
        )
        .unwrap();

        assert!(result.call_succeeded);
        assert_eq!(state.base().accounts().account(sender).nonce, 1);
        assert_eq!(state.base().accounts().account(sender).balance, 8_680);
        assert_eq!(state.base().accounts().account(producer).balance, 410);
        assert_eq!(state.contract(contract_id).unwrap().balance, 600);
        assert_eq!(
            state.contract(contract_id).unwrap().storage(storage_key),
            Some(storage_value)
        );
        assert_eq!(result.gas_used, 205);
        assert_eq!(result.actual_fee, 820);
        assert_eq!(result.unused_fee_reserve, 205);
    }

    #[test]
    fn accepted_call_revert_discards_value_and_storage_but_charges_used_gas() {
        let key = SigningKey::from_slice(&[0x32; 32]).unwrap();
        let contract_id = [0x72; 20];
        let tx = signed_call(&key, 0, contract_id, 500, 10, 5, Vec::new());
        let sender = tx
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let producer = [0x94; 20];
        let mut state = funded_v3(sender, 10_000, 0);
        insert_contract(&mut state, contract_id, 100, module(1, 1, &[0x41]));

        let result = execute_inactive_accepted_contract_call_v1(
            &mut state,
            &registry(),
            &tx,
            AddressNetwork::Devnet,
            10,
            NativeExecutionContextV1 {
                base_fee_per_gas: 2,
                cpu_producer: producer,
            },
        )
        .unwrap();

        assert!(!result.call_succeeded);
        assert!(matches!(result.execution.halt, Nvm1Halt::Revert(_)));
        assert_eq!(result.gas_used, 1);
        assert_eq!(result.actual_fee, 4);
        assert_eq!(state.base().accounts().account(sender).nonce, 1);
        assert_eq!(state.base().accounts().account(sender).balance, 9_996);
        assert_eq!(state.base().accounts().account(producer).balance, 2);
        assert_eq!(state.contract(contract_id).unwrap().balance, 100);
    }

    #[test]
    fn accepted_call_out_of_gas_charges_full_limit_and_discards_value() {
        let key = SigningKey::from_slice(&[0x33; 32]).unwrap();
        let contract_id = [0x73; 20];
        let tx = signed_call(&key, 0, contract_id, 500, 1, 5, Vec::new());
        let sender = tx
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let producer = [0x95; 20];
        let mut state = funded_v3(sender, 10_000, 0);
        insert_contract(
            &mut state,
            contract_id,
            100,
            module(
                1,
                1,
                &[
                    0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
            ),
        );

        let result = execute_inactive_accepted_contract_call_v1(
            &mut state,
            &registry(),
            &tx,
            AddressNetwork::Devnet,
            10,
            NativeExecutionContextV1 {
                base_fee_per_gas: 2,
                cpu_producer: producer,
            },
        )
        .unwrap();

        assert!(!result.call_succeeded);
        assert!(matches!(result.execution.halt, Nvm1Halt::Trap(_)));
        assert_eq!(result.gas_used, 1);
        assert_eq!(result.actual_fee, 4);
        assert_eq!(state.base().accounts().account(sender).nonce, 1);
        assert_eq!(state.base().accounts().account(sender).balance, 9_996);
        assert_eq!(state.contract(contract_id).unwrap().balance, 100);
    }

    #[test]
    fn accepted_create_success_consumes_nonce_value_and_exact_fee() {
        let key = SigningKey::from_slice(&[0x21; 32]).unwrap();
        let tx = signed_create(&key, 0, 500, 10, 5, module(1, 1, &[0x00]));
        let sender = tx
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let producer = [0x90; 20];
        let mut state = funded_v3(sender, 10_000, 0);

        let result = execute_inactive_accepted_contract_create_v1(
            &mut state,
            &registry(),
            &tx,
            AddressNetwork::Devnet,
            10,
            NativeExecutionContextV1 {
                base_fee_per_gas: 2,
                cpu_producer: producer,
            },
        )
        .unwrap();

        assert!(result.contract_created);
        assert_eq!(result.gas_used, 1);
        assert_eq!(result.base_fee_burned, 2);
        assert_eq!(result.producer_priority_fee, 2);
        assert_eq!(result.actual_fee, 4);
        assert_eq!(result.unused_fee_reserve, 46);
        assert_eq!(state.base().accounts().account(sender).nonce, 1);
        assert_eq!(state.base().accounts().account(sender).balance, 9_496);
        assert_eq!(state.base().accounts().account(producer).balance, 2);
        assert_eq!(state.contract(result.contract_id).unwrap().balance, 500);
    }

    #[test]
    fn accepted_create_revert_consumes_nonce_and_used_gas_but_refunds_value() {
        let key = SigningKey::from_slice(&[0x22; 32]).unwrap();
        let tx = signed_create(&key, 0, 500, 10, 5, module(1, 1, &[0x41]));
        let sender = tx
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let producer = [0x91; 20];
        let mut state = funded_v3(sender, 10_000, 0);

        let result = execute_inactive_accepted_contract_create_v1(
            &mut state,
            &registry(),
            &tx,
            AddressNetwork::Devnet,
            10,
            NativeExecutionContextV1 {
                base_fee_per_gas: 2,
                cpu_producer: producer,
            },
        )
        .unwrap();

        assert!(!result.contract_created);
        assert!(matches!(result.execution.halt, Nvm1Halt::Revert(_)));
        assert_eq!(result.gas_used, 1);
        assert_eq!(state.base().accounts().account(sender).nonce, 1);
        assert_eq!(state.base().accounts().account(sender).balance, 9_996);
        assert_eq!(state.base().accounts().account(producer).balance, 2);
        assert!(state.contract(result.contract_id).is_none());
    }

    #[test]
    fn accepted_create_out_of_gas_consumes_full_limit_and_refunds_value() {
        let key = SigningKey::from_slice(&[0x23; 32]).unwrap();
        let tx = signed_create(
            &key,
            0,
            500,
            1,
            5,
            module(
                1,
                1,
                &[
                    0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
            ),
        );
        let sender = tx
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;
        let producer = [0x92; 20];
        let mut state = funded_v3(sender, 10_000, 0);

        let result = execute_inactive_accepted_contract_create_v1(
            &mut state,
            &registry(),
            &tx,
            AddressNetwork::Devnet,
            10,
            NativeExecutionContextV1 {
                base_fee_per_gas: 2,
                cpu_producer: producer,
            },
        )
        .unwrap();

        assert!(!result.contract_created);
        assert!(matches!(result.execution.halt, Nvm1Halt::Trap(_)));
        assert_eq!(result.gas_used, 1);
        assert_eq!(result.actual_fee, 4);
        assert_eq!(state.base().accounts().account(sender).nonce, 1);
        assert_eq!(state.base().accounts().account(sender).balance, 9_996);
        assert!(state.contract(result.contract_id).is_none());
    }

    #[test]
    fn successful_constructor_creates_exact_contract_record() {
        let creator = [0x11; 20];
        let mut state = NativeStateV3::default();
        let create = payload(module(1, 1, &[0x00]), vec![0xaa, 0xbb]);

        let transition = execute_inactive_contract_create_transition_v1(
            &mut state,
            &registry(),
            &create,
            InactiveContractCreateRequestV1 {
                network: AddressNetwork::Devnet,
                chain_id: 0x0000_0002_4449_4148,
                current_height: 10,
                creator_payload: creator,
                creator_nonce: 7,
                value: u128::MAX - 9,
                gas_limit: 1,
            },
        )
        .unwrap();

        assert!(transition.contract_created);
        assert_eq!(transition.execution.halt, Nvm1Halt::Stop);
        let contract = state.contract(transition.contract_id).unwrap();
        assert_eq!(contract.balance, u128::MAX - 9);
        assert_eq!(contract.runtime_id, NVM1_RUNTIME_ID);
        assert_eq!(contract.code, create.code);
        assert_eq!(contract.storage_count(), 0);
        assert_ne!(transition.state_root_before, transition.state_root_after);
    }

    #[test]
    fn successful_constructor_commits_working_storage() {
        let key = [0x22; 32];
        let stored = [0x33; 32];
        let mut instructions = vec![0x02];
        instructions.extend_from_slice(&key);
        instructions.push(0x02);
        instructions.extend_from_slice(&stored);
        instructions.push(0x21);
        instructions.push(0x00);

        let mut state = NativeStateV3::default();
        let create = payload(module(4, 2, &instructions), Vec::new());
        let transition = execute_inactive_contract_create_transition_v1(
            &mut state,
            &registry(),
            &create,
            InactiveContractCreateRequestV1 {
                network: AddressNetwork::Devnet,
                chain_id: 0x0000_0002_4449_4148,
                current_height: 10,
                creator_payload: [0x44; 20],
                creator_nonce: 0,
                value: 0,
                gas_limit: 205,
            },
        )
        .unwrap();

        assert!(transition.contract_created);
        assert_eq!(
            state.contract(transition.contract_id).unwrap().storage(key),
            Some(stored)
        );
    }

    #[test]
    fn revert_discards_contract_and_state_changes() {
        let mut state = NativeStateV3::default();
        let before = state.clone();
        let create = payload(module(1, 1, &[0x41]), vec![0x01, 0x02]);

        let transition = execute_inactive_contract_create_transition_v1(
            &mut state,
            &registry(),
            &create,
            InactiveContractCreateRequestV1 {
                network: AddressNetwork::Devnet,
                chain_id: 0x0000_0002_4449_4148,
                current_height: 10,
                creator_payload: [0x55; 20],
                creator_nonce: 3,
                value: 99,
                gas_limit: 1,
            },
        )
        .unwrap();

        assert!(!transition.contract_created);
        assert!(matches!(transition.execution.halt, Nvm1Halt::Revert(_)));
        assert_eq!(state, before);
        assert_eq!(transition.state_root_before, transition.state_root_after);
    }

    #[test]
    fn trap_or_out_of_gas_does_not_create_contract() {
        let mut state = NativeStateV3::default();
        let before = state.clone();
        let create = payload(module(1, 1, &[0x00]), Vec::new());

        let transition = execute_inactive_contract_create_transition_v1(
            &mut state,
            &registry(),
            &create,
            InactiveContractCreateRequestV1 {
                network: AddressNetwork::Devnet,
                chain_id: 0x0000_0002_4449_4148,
                current_height: 10,
                creator_payload: [0x66; 20],
                creator_nonce: 4,
                value: 0,
                gas_limit: 0,
            },
        )
        .unwrap();

        assert!(!transition.contract_created);
        assert!(matches!(transition.execution.halt, Nvm1Halt::Trap(_)));
        assert_eq!(state, before);
    }

    #[test]
    fn collision_rejects_before_constructor_execution() {
        let creator = [0x77; 20];
        let nonce = 5;
        let chain_id = 0x0000_0002_4449_4148;
        let contract_id = NiahciaAddressV1::contract_from_creator(
            AddressNetwork::Devnet,
            chain_id,
            creator,
            nonce,
        )
        .payload;

        let mut state = NativeStateV3::default();
        state
            .insert_contract(ContractStateV1::new(
                contract_id,
                0,
                NVM1_RUNTIME_ID,
                module(1, 1, &[0x00]),
            ))
            .unwrap();
        let before = state.clone();

        let error = execute_inactive_contract_create_transition_v1(
            &mut state,
            &registry(),
            &payload(module(1, 1, &[0x00]), Vec::new()),
            InactiveContractCreateRequestV1 {
                network: AddressNetwork::Devnet,
                chain_id,
                current_height: 10,
                creator_payload: creator,
                creator_nonce: nonce,
                value: 0,
                gas_limit: 1,
            },
        )
        .unwrap_err();

        assert!(error.contains("already exists"));
        assert_eq!(state, before);
    }
}
