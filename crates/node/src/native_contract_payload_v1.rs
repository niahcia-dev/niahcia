use crate::nce::{decode_envelope, encode_bytes, encode_envelope, encode_map, encode_unsigned};

pub const CONTRACT_CREATE_PAYLOAD_OBJECT_TYPE: u64 = 0x0018;
pub const CONTRACT_CREATE_PAYLOAD_SCHEMA_VERSION: u64 = 1;

pub const MAX_CONTRACT_CODE_BYTES_V1: usize = 65_536;
pub const MAX_CONTRACT_INIT_DATA_BYTES_V1: usize = 65_536;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractCreatePayloadV1 {
    pub runtime_id: u32,
    pub code: Vec<u8>,
    pub init_data: Vec<u8>,
}

impl ContractCreatePayloadV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.runtime_id == 0 {
            return Err("contract runtime_id 0 is reserved".into());
        }
        if self.code.is_empty() {
            return Err("ContractCreate code must not be empty".into());
        }
        if self.code.len() > MAX_CONTRACT_CODE_BYTES_V1 {
            return Err(format!(
                "ContractCreate code exceeds {} bytes",
                MAX_CONTRACT_CODE_BYTES_V1
            ));
        }
        if self.init_data.len() > MAX_CONTRACT_INIT_DATA_BYTES_V1 {
            return Err(format!(
                "ContractCreate init_data exceeds {} bytes",
                MAX_CONTRACT_INIT_DATA_BYTES_V1
            ));
        }
        Ok(())
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        Ok(encode_map(&[
            (1, encode_unsigned(self.runtime_id as u64)),
            (2, encode_bytes(&self.code)),
            (3, encode_bytes(&self.init_data)),
        ]))
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            CONTRACT_CREATE_PAYLOAD_OBJECT_TYPE,
            CONTRACT_CREATE_PAYLOAD_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            CONTRACT_CREATE_PAYLOAD_OBJECT_TYPE,
            CONTRACT_CREATE_PAYLOAD_SCHEMA_VERSION,
        )?;

        if reader.map_len()? != 3 {
            return Err("ContractCreate payload must contain exactly three fields".into());
        }

        if reader.unsigned()? != 1 {
            return Err("invalid ContractCreate runtime_id field".into());
        }
        let runtime_id = reader.unsigned()?;
        let runtime_id = u32::try_from(runtime_id)
            .map_err(|_| "ContractCreate runtime_id exceeds u32".to_string())?;

        if reader.unsigned()? != 2 {
            return Err("invalid ContractCreate code field".into());
        }
        let code = reader.bytes()?.to_vec();

        if reader.unsigned()? != 3 {
            return Err("invalid ContractCreate init_data field".into());
        }
        let init_data = reader.bytes()?.to_vec();

        if !reader.finished() {
            return Err("trailing bytes after ContractCreate payload".into());
        }

        let payload = Self {
            runtime_id,
            code,
            init_data,
        };
        payload.validate()?;

        if payload.canonical_bytes()? != bytes {
            return Err("ContractCreate payload is not canonically encoded".into());
        }

        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_payload_round_trips_and_separates_code_from_init_data() {
        let payload = ContractCreatePayloadV1 {
            runtime_id: 1,
            code: vec![0x01, 0x02, 0x03, 0x04],
            init_data: vec![0xa1, 0xb2, 0xc3],
        };
        let bytes = payload.canonical_bytes().unwrap();
        let decoded = ContractCreatePayloadV1::from_canonical_bytes(&bytes).unwrap();

        assert_eq!(decoded, payload);
        assert_eq!(decoded.code, vec![0x01, 0x02, 0x03, 0x04]);
        assert_eq!(decoded.init_data, vec![0xa1, 0xb2, 0xc3]);
    }

    #[test]
    fn creation_payload_rejects_reserved_runtime_and_empty_code() {
        let reserved = ContractCreatePayloadV1 {
            runtime_id: 0,
            code: vec![0x01],
            init_data: Vec::new(),
        };
        assert!(reserved.validate().is_err());

        let empty = ContractCreatePayloadV1 {
            runtime_id: 1,
            code: Vec::new(),
            init_data: Vec::new(),
        };
        assert!(empty.validate().is_err());
    }

    #[test]
    fn creation_payload_bounds_are_exact() {
        let max = ContractCreatePayloadV1 {
            runtime_id: 1,
            code: vec![0xaa; MAX_CONTRACT_CODE_BYTES_V1],
            init_data: vec![0xbb; MAX_CONTRACT_INIT_DATA_BYTES_V1],
        };
        assert!(max.validate().is_ok());

        let mut too_much_code = max.clone();
        too_much_code.code.push(0);
        assert!(too_much_code.validate().is_err());

        let mut too_much_init = max;
        too_much_init.init_data.push(0);
        assert!(too_much_init.validate().is_err());
    }

    #[test]
    fn locked_contract_create_payload_vector_matches_json() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/native-contract-create-payload-v1.json"
        ))
        .unwrap();
        let fixture = &vectors["fixture"];

        let payload = ContractCreatePayloadV1 {
            runtime_id: fixture["runtime_id"].as_u64().unwrap() as u32,
            code: hex::decode(fixture["code_hex"].as_str().unwrap()).unwrap(),
            init_data: hex::decode(fixture["init_data_hex"].as_str().unwrap()).unwrap(),
        };

        assert_eq!(
            hex::encode(payload.canonical_bytes().unwrap()),
            fixture["canonical_hex"].as_str().unwrap()
        );
        assert_eq!(
            vectors["limits"]["max_code_bytes"].as_u64().unwrap() as usize,
            MAX_CONTRACT_CODE_BYTES_V1
        );
        assert_eq!(
            vectors["limits"]["max_init_data_bytes"].as_u64().unwrap() as usize,
            MAX_CONTRACT_INIT_DATA_BYTES_V1
        );
    }
}
