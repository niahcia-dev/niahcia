use crate::native_contract_payload_v1::{
    ContractCreatePayloadV1, MAX_CONTRACT_CODE_BYTES_V1, MAX_CONTRACT_INIT_DATA_BYTES_V1,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContractRuntimeDescriptorV1 {
    pub runtime_id: u32,
    pub code_format_version: u32,
    pub activation_height: u64,
    pub retirement_height: Option<u64>,
    pub max_code_bytes: u32,
    pub max_init_data_bytes: u32,
}

impl ContractRuntimeDescriptorV1 {
    pub fn validate(self) -> Result<Self, String> {
        if self.runtime_id == 0 {
            return Err("contract runtime_id 0 is reserved".into());
        }
        if self.code_format_version == 0 {
            return Err("contract code_format_version 0 is reserved".into());
        }
        if self.max_code_bytes == 0 || self.max_code_bytes as usize > MAX_CONTRACT_CODE_BYTES_V1 {
            return Err(format!(
                "contract runtime max_code_bytes must be within 1..={MAX_CONTRACT_CODE_BYTES_V1}"
            ));
        }
        if self.max_init_data_bytes as usize > MAX_CONTRACT_INIT_DATA_BYTES_V1 {
            return Err(format!(
                "contract runtime max_init_data_bytes must not exceed {MAX_CONTRACT_INIT_DATA_BYTES_V1}"
            ));
        }
        if let Some(retirement_height) = self.retirement_height {
            if retirement_height <= self.activation_height {
                return Err(
                    "contract runtime retirement_height must be greater than activation_height"
                        .into(),
                );
            }
        }
        Ok(self)
    }

    pub fn is_active_at(self, height: u64) -> bool {
        height >= self.activation_height
            && self
                .retirement_height
                .map(|retirement| height < retirement)
                .unwrap_or(true)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeContractRuntimeRegistryV1 {
    runtimes: BTreeMap<u32, ContractRuntimeDescriptorV1>,
}

impl NativeContractRuntimeRegistryV1 {
    pub fn new(
        descriptors: impl IntoIterator<Item = ContractRuntimeDescriptorV1>,
    ) -> Result<Self, String> {
        let mut runtimes = BTreeMap::new();
        for descriptor in descriptors {
            let descriptor = descriptor.validate()?;
            if runtimes.insert(descriptor.runtime_id, descriptor).is_some() {
                return Err(format!(
                    "duplicate contract runtime_id {}",
                    descriptor.runtime_id
                ));
            }
        }
        Ok(Self { runtimes })
    }

    pub fn descriptor(&self, runtime_id: u32) -> Option<ContractRuntimeDescriptorV1> {
        self.runtimes.get(&runtime_id).copied()
    }

    pub fn active_descriptor(
        &self,
        runtime_id: u32,
        height: u64,
    ) -> Result<ContractRuntimeDescriptorV1, String> {
        let descriptor = self
            .descriptor(runtime_id)
            .ok_or_else(|| format!("unknown contract runtime_id {runtime_id}"))?;

        if !descriptor.is_active_at(height) {
            return Err(format!(
                "contract runtime_id {runtime_id} is not active at height {height}"
            ));
        }

        Ok(descriptor)
    }

    pub fn validate_create_payload(
        &self,
        height: u64,
        payload: &ContractCreatePayloadV1,
    ) -> Result<ContractRuntimeDescriptorV1, String> {
        payload.validate()?;
        let descriptor = self.active_descriptor(payload.runtime_id, height)?;

        if payload.code.len() > descriptor.max_code_bytes as usize {
            return Err(format!(
                "ContractCreate code exceeds runtime {} limit of {} bytes",
                descriptor.runtime_id, descriptor.max_code_bytes
            ));
        }
        if payload.init_data.len() > descriptor.max_init_data_bytes as usize {
            return Err(format!(
                "ContractCreate init_data exceeds runtime {} limit of {} bytes",
                descriptor.runtime_id, descriptor.max_init_data_bytes
            ));
        }

        Ok(descriptor)
    }

    pub fn runtime_count(&self) -> usize {
        self.runtimes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> ContractRuntimeDescriptorV1 {
        ContractRuntimeDescriptorV1 {
            runtime_id: 1,
            code_format_version: 1,
            activation_height: 100,
            retirement_height: Some(200),
            max_code_bytes: 32_768,
            max_init_data_bytes: 16_384,
        }
    }

    fn payload() -> ContractCreatePayloadV1 {
        ContractCreatePayloadV1 {
            runtime_id: 1,
            code: vec![0x01, 0x02, 0x03],
            init_data: vec![0xa1, 0xb2],
        }
    }

    #[test]
    fn empty_registry_activates_no_runtime() {
        let registry = NativeContractRuntimeRegistryV1::default();
        assert_eq!(registry.runtime_count(), 0);
        assert!(registry.validate_create_payload(100, &payload()).is_err());
    }

    #[test]
    fn runtime_activation_and_retirement_boundaries_are_exact() {
        let registry = NativeContractRuntimeRegistryV1::new([descriptor()]).unwrap();

        assert!(registry.validate_create_payload(99, &payload()).is_err());
        assert!(registry.validate_create_payload(100, &payload()).is_ok());
        assert!(registry.validate_create_payload(199, &payload()).is_ok());
        assert!(registry.validate_create_payload(200, &payload()).is_err());
    }

    #[test]
    fn runtime_specific_payload_limits_are_enforced() {
        let registry = NativeContractRuntimeRegistryV1::new([descriptor()]).unwrap();

        let mut oversized_code = payload();
        oversized_code.code = vec![0xaa; 32_769];
        assert!(registry
            .validate_create_payload(100, &oversized_code)
            .unwrap_err()
            .contains("32768"));

        let mut oversized_init = payload();
        oversized_init.init_data = vec![0xbb; 16_385];
        assert!(registry
            .validate_create_payload(100, &oversized_init)
            .unwrap_err()
            .contains("16384"));
    }

    #[test]
    fn registry_rejects_duplicate_or_invalid_descriptors() {
        assert!(NativeContractRuntimeRegistryV1::new([descriptor(), descriptor()]).is_err());

        let mut invalid = descriptor();
        invalid.runtime_id = 0;
        assert!(NativeContractRuntimeRegistryV1::new([invalid]).is_err());

        let mut invalid = descriptor();
        invalid.retirement_height = Some(invalid.activation_height);
        assert!(NativeContractRuntimeRegistryV1::new([invalid]).is_err());
    }

    #[test]
    fn locked_runtime_registry_vector_matches_json() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/native-contract-runtime-registry-v1.json"
        ))
        .unwrap();
        let fixture = &vectors["fixture"];

        let descriptor = ContractRuntimeDescriptorV1 {
            runtime_id: fixture["runtime_id"].as_u64().unwrap() as u32,
            code_format_version: fixture["code_format_version"].as_u64().unwrap() as u32,
            activation_height: fixture["activation_height"].as_u64().unwrap(),
            retirement_height: Some(fixture["retirement_height"].as_u64().unwrap()),
            max_code_bytes: fixture["max_code_bytes"].as_u64().unwrap() as u32,
            max_init_data_bytes: fixture["max_init_data_bytes"].as_u64().unwrap() as u32,
        };
        let registry = NativeContractRuntimeRegistryV1::new([descriptor]).unwrap();

        for case in vectors["height_cases"].as_array().unwrap() {
            let height = case["height"].as_u64().unwrap();
            let active = case["active"].as_bool().unwrap();
            assert_eq!(
                registry
                    .active_descriptor(descriptor.runtime_id, height)
                    .is_ok(),
                active
            );
        }
    }
}
