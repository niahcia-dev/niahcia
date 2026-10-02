use crate::address::{AddressNetwork, NiahciaAddressV1, ADDRESS_PAYLOAD_LEN};
use crate::nce::{encode_bytes, encode_envelope, encode_map, encode_unsigned};
use crate::work::{keccak256, Hash32};
use k256::ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};

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

pub const SIGNED_NATIVE_TRANSACTION_OBJECT_TYPE: u64 = 0x0011;
pub const SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION: u64 = 1;

const NATIVE_TRANSACTION_SIGNING_PURPOSE: &[u8] = b"SIGN/NATIVE_TRANSACTION";
const NATIVE_TRANSACTION_TX_ID_DOMAIN: &[u8] = b"NIAHCIA/TX-ID/V1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedNativeTransactionV1 {
    pub body: NativeTransactionBodyV1,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl SignedNativeTransactionV1 {
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, String> {
        self.body.canonical_bytes()
    }

    pub fn signing_digest(&self) -> Result<Hash32, String> {
        let body = self.canonical_body_bytes()?;

        let network_id = u8::try_from(self.body.network_id)
            .map_err(|_| "native transaction network_id does not fit one byte".to_string())?;

        if !matches!(network_id, 0x00 | 0x01 | 0x02) {
            return Err(format!(
                "unsupported native transaction network_id: {}",
                self.body.network_id
            ));
        }

        let mut preimage = Vec::with_capacity(
            b"NIAHCIA".len()
                + 1
                + NATIVE_TRANSACTION_SIGNING_PURPOSE.len()
                + 1
                + 1
                + 1
                + body.len(),
        );

        preimage.extend_from_slice(b"NIAHCIA");
        preimage.push(0);
        preimage.extend_from_slice(NATIVE_TRANSACTION_SIGNING_PURPOSE);
        preimage.push(0);
        preimage.push(network_id);
        preimage.push(0);
        preimage.extend_from_slice(&body);

        Ok(keccak256(&preimage))
    }

    fn canonical_verifying_key(&self) -> Result<VerifyingKey, String> {
        if self.public_key.len() != 65 || self.public_key.first() != Some(&0x04) {
            return Err(
                "native transaction public key must be canonical uncompressed SEC1 \
                 (65 bytes, 0x04 prefix)"
                    .into(),
            );
        }

        let verifying_key = VerifyingKey::from_sec1_bytes(&self.public_key)
            .map_err(|_| "invalid secp256k1 native transaction public key".to_string())?;

        let canonical = verifying_key.to_encoded_point(false);
        if canonical.as_bytes() != self.public_key.as_slice() {
            return Err("non-canonical native transaction public key".into());
        }

        Ok(verifying_key)
    }

    fn canonical_signature(&self) -> Result<Signature, String> {
        if self.signature.len() != 64 {
            return Err("native transaction signature must be exactly 64 bytes".into());
        }

        let signature = Signature::from_slice(&self.signature)
            .map_err(|_| "invalid secp256k1 native transaction signature".to_string())?;

        if signature.normalize_s().is_some() {
            return Err("native transaction signature must use canonical low-S form".into());
        }

        Ok(signature)
    }

    pub fn verify_signature(&self, network: AddressNetwork) -> Result<(), String> {
        self.body.validate(network)?;

        let verifying_key = self.canonical_verifying_key()?;
        let signature = self.canonical_signature()?;
        let digest = self.signing_digest()?;

        verifying_key
            .verify_prehash(&digest, &signature)
            .map_err(|_| "invalid native transaction signature".to_string())
    }

    pub fn authenticated_sender(
        &self,
        network: AddressNetwork,
    ) -> Result<NiahciaAddressV1, String> {
        self.verify_signature(network)?;

        NiahciaAddressV1::account_from_uncompressed_public_key(network, &self.public_key)
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        self.canonical_verifying_key()?;
        self.canonical_signature()?;

        encode_map(&[
            (1, encode_bytes(&self.canonical_body_bytes()?)),
            (2, encode_bytes(&self.public_key)),
            (3, encode_bytes(&self.signature)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            SIGNED_NATIVE_TRANSACTION_OBJECT_TYPE,
            SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }

    pub fn tx_id(&self) -> Result<Hash32, String> {
        let canonical = self.canonical_bytes()?;

        let mut preimage =
            Vec::with_capacity(NATIVE_TRANSACTION_TX_ID_DOMAIN.len() + 1 + canonical.len());

        preimage.extend_from_slice(NATIVE_TRANSACTION_TX_ID_DOMAIN);
        preimage.push(0);
        preimage.extend_from_slice(&canonical);

        Ok(keccak256(&preimage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

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

    fn signed_transfer() -> SignedNativeTransactionV1 {
        let signing_key = SigningKey::from_slice(&[0x01; 32]).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

        let mut tx = SignedNativeTransactionV1 {
            body: transfer(),
            public_key,
            signature: vec![0; 64],
        };

        let digest = tx.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        tx.signature = signature.to_bytes().to_vec();
        tx
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
    fn signed_transaction_verifies_and_derives_sender() {
        let tx = signed_transfer();

        tx.verify_signature(AddressNetwork::Devnet).unwrap();

        let sender = tx.authenticated_sender(AddressNetwork::Devnet).unwrap();
        let expected = NiahciaAddressV1::account_from_uncompressed_public_key(
            AddressNetwork::Devnet,
            &tx.public_key,
        )
        .unwrap();

        assert_eq!(sender, expected);
    }

    #[test]
    fn signed_transaction_rejects_tampered_body() {
        let mut tx = signed_transfer();
        tx.body.value += 1;

        assert!(tx.verify_signature(AddressNetwork::Devnet).is_err());
    }

    #[test]
    fn signed_transaction_rejects_wrong_network() {
        let tx = signed_transfer();

        assert!(tx.verify_signature(AddressNetwork::Mainnet).is_err());
    }

    #[test]
    fn signed_transaction_requires_canonical_uncompressed_public_key() {
        let mut tx = signed_transfer();
        let signing_key = SigningKey::from_slice(&[0x01; 32]).unwrap();

        tx.public_key = signing_key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec();

        assert!(tx.verify_signature(AddressNetwork::Devnet).is_err());
        assert!(tx.canonical_bytes().is_err());
    }

    #[test]
    fn signed_transaction_rejects_high_s_signature() {
        let mut tx = signed_transfer();

        // secp256k1 group order:
        // FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
        const ORDER: [u8; 32] = [
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c,
            0xd0, 0x36, 0x41, 0x41,
        ];

        let low_s = &tx.signature[32..64];
        let mut high_s = [0u8; 32];
        let mut borrow = 0u16;

        for i in (0..32).rev() {
            let n = ORDER[i] as u16;
            let s = low_s[i] as u16 + borrow;

            if n >= s {
                high_s[i] = (n - s) as u8;
                borrow = 0;
            } else {
                high_s[i] = (256 + n - s) as u8;
                borrow = 1;
            }
        }

        assert_eq!(borrow, 0);

        tx.signature[32..64].copy_from_slice(&high_s);

        let parsed = Signature::from_slice(&tx.signature).unwrap();
        assert!(
            parsed.normalize_s().is_some(),
            "constructed signature must actually be high-S"
        );

        assert!(tx.verify_signature(AddressNetwork::Devnet).is_err());
        assert!(tx.canonical_bytes().is_err());
    }

    #[test]
    fn signed_transaction_id_is_deterministic_and_commits_signature() {
        let tx = signed_transfer();

        let first = tx.tx_id().unwrap();
        let second = tx.tx_id().unwrap();
        assert_eq!(first, second);

        let mut changed = tx.clone();
        changed.signature[0] ^= 1;

        assert_ne!(first, changed.tx_id().unwrap());
    }

    #[test]
    fn native_transaction_v1_interoperability_vector_is_exact() {
        let tx = signed_transfer();

        assert_eq!(
            hex::encode(tx.canonical_body_bytes().unwrap()),
            "a401010210030104a90102021b00000002444941480307040005542222222222222222222222222222222222222222065000000000000000000000000005f5e100071952080850000000000000000000000000000000190940"
        );

        assert_eq!(
            hex::encode(tx.signing_digest().unwrap()),
            "826dfcd3d1a94b70d3d3d300289561f727e30aa74fd367325dc1a1346ea995bc"
        );

        assert_eq!(
            hex::encode(&tx.public_key),
            "041b84c5567b126440995d3ed5aaba0565d71e1834604819ff9c17f5e9d5dd078f70beaf8f588b541507fed6a642c5ab42dfdf8120a7f639de5122d47a69a8e8d1"
        );

        assert_eq!(
            hex::encode(&tx.signature),
            "c8499c41816006f930d757b88c248cecd8a349c412308f5345fd5c93ce6942b41ea32d1719c27b8f5c80d8476b9b10296c4d1c1361f0b1adf123f8e4fb314841"
        );

        assert_eq!(
            hex::encode(tx.canonical_bytes().unwrap()),
            "a401010211030104a3015859a401010210030104a90102021b00000002444941480307040005542222222222222222222222222222222222222222065000000000000000000000000005f5e100071952080850000000000000000000000000000000190940025841041b84c5567b126440995d3ed5aaba0565d71e1834604819ff9c17f5e9d5dd078f70beaf8f588b541507fed6a642c5ab42dfdf8120a7f639de5122d47a69a8e8d1035840c8499c41816006f930d757b88c248cecd8a349c412308f5345fd5c93ce6942b41ea32d1719c27b8f5c80d8476b9b10296c4d1c1361f0b1adf123f8e4fb314841"
        );

        assert_eq!(
            hex::encode(tx.tx_id().unwrap()),
            "964132b74a2a14fdc75ebc7755960ca46ec0b042d8628e8dd0c85b56b605a664"
        );

        let sender = tx.authenticated_sender(AddressNetwork::Devnet).unwrap();

        assert_eq!(
            hex::encode(sender.payload),
            "1a642f0e3c3af545e7acbd38b07251b3990914f1"
        );

        assert_eq!(
            sender.encode().unwrap(),
            "dniah1qyqp5ep0pc7r4a29u7kt6w9swfgm8xgfzncskfytt4"
        );
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
