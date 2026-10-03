use crate::address::{AddressNetwork, NiahciaAddressV1};
use crate::native_contract_payload_v1::ContractCreatePayloadV1;
use crate::native_contract_runtime_registry_v1::NativeContractRuntimeRegistryV1;
use crate::native_contract_state_v1::ContractStateV1;
use crate::native_contract_vm_v1::{
    execute_nvm1_core_with_context_and_gas, validate_nvm1_code, validate_nvm1_jump_targets,
    Nvm1ExecutionContext, Nvm1ExecutionResult, Nvm1Halt, NVM1_CODE_FORMAT_VERSION, NVM1_RUNTIME_ID,
};
use crate::native_state_v3::NativeStateV3;
use crate::work::Hash32;
use std::collections::BTreeMap;

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
    network: AddressNetwork,
    chain_id: u64,
    current_height: u64,
    creator_payload: [u8; 20],
    creator_nonce: u64,
    value: u128,
    payload: &ContractCreatePayloadV1,
    gas_limit: u64,
) -> Result<InactiveContractCreateTransitionV1, String> {
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

    #[test]
    fn successful_constructor_creates_exact_contract_record() {
        let creator = [0x11; 20];
        let mut state = NativeStateV3::default();
        let create = payload(module(1, 1, &[0x00]), vec![0xaa, 0xbb]);

        let transition = execute_inactive_contract_create_transition_v1(
            &mut state,
            &registry(),
            AddressNetwork::Devnet,
            0x0000_0002_4449_4148,
            10,
            creator,
            7,
            u128::MAX - 9,
            &create,
            1,
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
            AddressNetwork::Devnet,
            0x0000_0002_4449_4148,
            10,
            [0x44; 20],
            0,
            0,
            &create,
            205,
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
            AddressNetwork::Devnet,
            0x0000_0002_4449_4148,
            10,
            [0x55; 20],
            3,
            99,
            &create,
            1,
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
            AddressNetwork::Devnet,
            0x0000_0002_4449_4148,
            10,
            [0x66; 20],
            4,
            0,
            &create,
            0,
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
            AddressNetwork::Devnet,
            chain_id,
            10,
            creator,
            nonce,
            0,
            &payload(module(1, 1, &[0x00]), Vec::new()),
            1,
        )
        .unwrap_err();

        assert!(error.contains("already exists"));
        assert_eq!(state, before);
    }
}
