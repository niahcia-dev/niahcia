use crate::address::{AddressNetwork, ADDRESS_PAYLOAD_LEN};
use crate::nce::{encode_bytes, encode_envelope, encode_map, encode_unsigned};

pub const NATIVE_TRANSACTION_BODY_OBJECT_TYPE: u64 = 0x0010;
pub const NATIVE_TRANSACTION_SCHEMA_VERSION: u64 = 1;

pub const MAINNET_NETWORK_ID: u64 = 0x00;
pub const TESTNET_NETWORK_ID: u64 = 0x01;
pub const DEVNET_NETWORK_ID: u64 = 0x02;

pub const MAINNET_CHAIN_ID: u64 = 0x0000_0000_4E49_4148;
pub const TESTNET_CHAIN_ID: u64 = 0x0000_0001_5449_4148;
pub const DEVNET_CHAIN_ID: u64 = 0x0000_0002_4449_4148;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum NativeActionV1 {
    Transfer = 0x00,
    ContractCall = 0x01,
    ContractCreate = 0x02,
}

impl TryFrom<u64> for NativeActionV1 {
    type Error = String;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Transfer),
            0x01 => Ok(Self::ContractCall),
            0x02 => Ok(Self::ContractCreate),
            _ => Err(format!("unsupported native transaction action: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeTransactionBodyV1 {
    pub network_id: u64,
    pub chain_id: u64,
    pub nonce: u64,
    pub action: NativeActionV1,
    pub target_payload: Vec<u8>,
    pub value: u128,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub data: Vec<u8>,
}

impl NativeTransactionBodyV1 {
    pub fn network_identity(network: AddressNetwork) -> (u64, u64) {
        match network {
            AddressNetwork::Mainnet => (MAINNET_NETWORK_ID, MAINNET_CHAIN_ID),
            AddressNetwork::Testnet => (TESTNET_NETWORK_ID, TESTNET_CHAIN_ID),
            AddressNetwork::Devnet => (DEVNET_NETWORK_ID, DEVNET_CHAIN_ID),
        }
    }

    pub fn validate_network(&self, network: AddressNetwork) -> Result<(), String> {
        let (expected_network_id, expected_chain_id) = Self::network_identity(network);

        if self.network_id != expected_network_id {
            return Err(format!(
                "native transaction network mismatch: expected {}, found {}",
                expected_network_id, self.network_id
            ));
        }

        if self.chain_id != expected_chain_id {
            return Err(format!(
                "native transaction chain ID mismatch: expected {}, found {}",
                expected_chain_id, self.chain_id
            ));
        }

        Ok(())
    }

    pub fn validate_action(&self) -> Result<(), String> {
        match self.action {
            NativeActionV1::Transfer => {
                if self.target_payload.len() != ADDRESS_PAYLOAD_LEN {
                    return Err(format!(
                        "Transfer target payload must be exactly {} bytes",
                        ADDRESS_PAYLOAD_LEN
                    ));
                }

                if !self.data.is_empty() {
                    return Err("Transfer data must be empty".into());
                }
            }

            NativeActionV1::ContractCall => {
                if self.target_payload.len() != ADDRESS_PAYLOAD_LEN {
                    return Err(format!(
                        "ContractCall target payload must be exactly {} bytes",
                        ADDRESS_PAYLOAD_LEN
                    ));
                }
            }

            NativeActionV1::ContractCreate => {
                if !self.target_payload.is_empty() {
                    return Err("ContractCreate target payload must be empty".into());
                }
            }
        }

        Ok(())
    }

    pub fn validate(&self, network: AddressNetwork) -> Result<(), String> {
        self.validate_network(network)?;
        self.validate_action()
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        encode_map(&[
            (1, encode_unsigned(self.network_id)),
            (2, encode_unsigned(self.chain_id)),
            (3, encode_unsigned(self.nonce)),
            (4, encode_unsigned(self.action as u64)),
            (5, encode_bytes(&self.target_payload)),
            (6, encode_bytes(&self.value.to_be_bytes())),
            (7, encode_unsigned(self.gas_limit)),
            (8, encode_bytes(&self.max_fee_per_gas.to_be_bytes())),
            (9, encode_bytes(&self.data)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            NATIVE_TRANSACTION_BODY_OBJECT_TYPE,
            NATIVE_TRANSACTION_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transfer() -> NativeTransactionBodyV1 {
        NativeTransactionBodyV1 {
            network_id: DEVNET_NETWORK_ID,
            chain_id: DEVNET_CHAIN_ID,
            nonce: 7,
            action: NativeActionV1::Transfer,
            target_payload: vec![0x22; ADDRESS_PAYLOAD_LEN],
            value: 100_000_000,
            gas_limit: 21_000,
            max_fee_per_gas: 25,
            data: Vec::new(),
        }
    }

    #[test]
    fn native_network_identity_is_exact() {
        assert_eq!(
            NativeTransactionBodyV1::network_identity(AddressNetwork::Mainnet),
            (0, 1_313_423_688)
        );
        assert_eq!(
            NativeTransactionBodyV1::network_identity(AddressNetwork::Testnet),
            (1, 5_709_054_280)
        );
        assert_eq!(
            NativeTransactionBodyV1::network_identity(AddressNetwork::Devnet),
            (2, 9_735_586_120)
        );
    }

    #[test]
    fn action_values_are_exact() {
        assert_eq!(NativeActionV1::Transfer as u64, 0x00);
        assert_eq!(NativeActionV1::ContractCall as u64, 0x01);
        assert_eq!(NativeActionV1::ContractCreate as u64, 0x02);
        assert!(NativeActionV1::try_from(3).is_err());
    }

    #[test]
    fn transfer_rules_are_enforced() {
        let tx = transfer();
        assert!(tx.validate(AddressNetwork::Devnet).is_ok());

        let mut wrong_target = tx.clone();
        wrong_target.target_payload.pop();
        assert!(wrong_target.validate_action().is_err());

        let mut with_data = tx;
        with_data.data.push(1);
        assert!(with_data.validate_action().is_err());
    }

    #[test]
    fn contract_action_target_rules_are_enforced() {
        let mut call = transfer();
        call.action = NativeActionV1::ContractCall;
        call.data = vec![1, 2, 3];
        assert!(call.validate_action().is_ok());

        let mut create = call;
        create.action = NativeActionV1::ContractCreate;
        create.target_payload.clear();
        assert!(create.validate_action().is_ok());

        create.target_payload.push(1);
        assert!(create.validate_action().is_err());
    }

    #[test]
    fn wrong_network_or_chain_is_rejected() {
        let tx = transfer();
        assert!(tx.validate_network(AddressNetwork::Devnet).is_ok());
        assert!(tx.validate_network(AddressNetwork::Mainnet).is_err());

        let mut wrong_chain = tx;
        wrong_chain.chain_id = MAINNET_CHAIN_ID;
        assert!(wrong_chain
            .validate_network(AddressNetwork::Devnet)
            .is_err());
    }

    #[test]
    fn monetary_fields_are_fixed_width_big_endian_byte_strings() {
        let mut tx = transfer();
        tx.value = 1;
        tx.max_fee_per_gas = 2;

        let payload = tx.canonical_payload().unwrap();

        let one = encode_bytes(&1u128.to_be_bytes());
        let two = encode_bytes(&2u128.to_be_bytes());

        assert!(payload.windows(one.len()).any(|w| w == one.as_slice()));
        assert!(payload.windows(two.len()).any(|w| w == two.as_slice()));
    }

    #[test]
    fn canonical_body_encoding_is_deterministic() {
        let tx = transfer();

        let first = tx.canonical_bytes().unwrap();
        let second = tx.canonical_bytes().unwrap();

        assert_eq!(first, second);
        assert!(!first.is_empty());
    }
}
