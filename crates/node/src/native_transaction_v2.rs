use crate::address::{AddressNetwork, NiahciaAddressV1, ADDRESS_PAYLOAD_LEN};
use crate::native_transaction::{
    DEVNET_CHAIN_ID, DEVNET_NETWORK_ID, MAINNET_CHAIN_ID, MAINNET_NETWORK_ID,
    NATIVE_TRANSACTION_BODY_OBJECT_TYPE, NATIVE_TRANSFER_GAS_V1,
    SIGNED_NATIVE_TRANSACTION_OBJECT_TYPE, TESTNET_CHAIN_ID, TESTNET_NETWORK_ID,
};
use crate::nce::{decode_envelope, encode_bytes, encode_envelope, encode_map, encode_unsigned};
use crate::work::{keccak256, transaction_merkle_root, Hash32};
use k256::ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};

pub const NATIVE_TRANSACTION_SCHEMA_VERSION_V2: u64 = 2;
pub const SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION_V2: u64 = 2;

const NATIVE_TRANSACTION_SIGNING_PURPOSE_V2: &[u8] = b"SIGN/NATIVE_TRANSACTION/V2";
const NATIVE_TRANSACTION_TX_ID_DOMAIN_V2: &[u8] = b"NIAHCIA/TX-ID/V2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum NativeActionV2 {
    Transfer = 0x00,
    ContractCall = 0x01,
    ContractCreate = 0x02,
    ComputeChannelOpen = 0x10,
    ComputeChannelSettle = 0x11,
    ComputeChannelRefund = 0x12,
}

impl TryFrom<u64> for NativeActionV2 {
    type Error = String;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Transfer),
            0x01 => Ok(Self::ContractCall),
            0x02 => Ok(Self::ContractCreate),
            0x10 => Ok(Self::ComputeChannelOpen),
            0x11 => Ok(Self::ComputeChannelSettle),
            0x12 => Ok(Self::ComputeChannelRefund),
            _ => Err(format!("unsupported native transaction V2 action: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeTransactionBodyV2 {
    pub network_id: u64,
    pub chain_id: u64,
    pub nonce: u64,
    pub action: NativeActionV2,
    pub target_payload: Vec<u8>,
    pub value: u128,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    pub data: Vec<u8>,
}

impl NativeTransactionBodyV2 {
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
                "native transaction V2 network mismatch: expected {}, found {}",
                expected_network_id, self.network_id
            ));
        }

        if self.chain_id != expected_chain_id {
            return Err(format!(
                "native transaction V2 chain ID mismatch: expected {}, found {}",
                expected_chain_id, self.chain_id
            ));
        }

        Ok(())
    }

    pub fn validate_action_shape(&self) -> Result<(), String> {
        match self.action {
            NativeActionV2::Transfer => {
                if self.target_payload.len() != ADDRESS_PAYLOAD_LEN {
                    return Err(format!(
                        "Transfer target payload must be exactly {} bytes",
                        ADDRESS_PAYLOAD_LEN
                    ));
                }
                if !self.data.is_empty() {
                    return Err("Transfer data must be empty".into());
                }
                if self.gas_limit < NATIVE_TRANSFER_GAS_V1 {
                    return Err(format!(
                        "Transfer gas_limit must be at least {}",
                        NATIVE_TRANSFER_GAS_V1
                    ));
                }
            }
            NativeActionV2::ContractCall => {
                if self.target_payload.len() != ADDRESS_PAYLOAD_LEN {
                    return Err(format!(
                        "ContractCall target payload must be exactly {} bytes",
                        ADDRESS_PAYLOAD_LEN
                    ));
                }
            }
            NativeActionV2::ContractCreate => {
                if !self.target_payload.is_empty() {
                    return Err("ContractCreate target payload must be empty".into());
                }
            }
            NativeActionV2::ComputeChannelOpen
            | NativeActionV2::ComputeChannelSettle
            | NativeActionV2::ComputeChannelRefund => {
                if !self.target_payload.is_empty() {
                    return Err("compute channel action target payload must be empty".into());
                }
                if self.data.is_empty() {
                    return Err("compute channel action data payload must not be empty".into());
                }
            }
        }

        Ok(())
    }

    pub fn validate(&self, network: AddressNetwork) -> Result<(), String> {
        self.validate_network(network)?;
        self.validate_action_shape()
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
            (
                9,
                encode_bytes(&self.max_priority_fee_per_gas.to_be_bytes()),
            ),
            (10, encode_bytes(&self.data)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            NATIVE_TRANSACTION_BODY_OBJECT_TYPE,
            NATIVE_TRANSACTION_SCHEMA_VERSION_V2,
            self.canonical_payload()?,
        )
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            NATIVE_TRANSACTION_BODY_OBJECT_TYPE,
            NATIVE_TRANSACTION_SCHEMA_VERSION_V2,
        )?;

        if reader.map_len()? != 10 {
            return Err("native transaction V2 body must contain exactly ten fields".into());
        }

        if reader.unsigned()? != 1 {
            return Err("invalid native transaction V2 network_id field".into());
        }
        let network_id = reader.unsigned()?;

        if reader.unsigned()? != 2 {
            return Err("invalid native transaction V2 chain_id field".into());
        }
        let chain_id = reader.unsigned()?;

        if reader.unsigned()? != 3 {
            return Err("invalid native transaction V2 nonce field".into());
        }
        let nonce = reader.unsigned()?;

        if reader.unsigned()? != 4 {
            return Err("invalid native transaction V2 action field".into());
        }
        let action = NativeActionV2::try_from(reader.unsigned()?)?;

        if reader.unsigned()? != 5 {
            return Err("invalid native transaction V2 target field".into());
        }
        let target_payload = reader.bytes()?.to_vec();

        if reader.unsigned()? != 6 {
            return Err("invalid native transaction V2 value field".into());
        }
        let value_bytes = reader.bytes()?;
        if value_bytes.len() != 16 {
            return Err("native transaction V2 value must be exactly 16 bytes".into());
        }
        let value = u128::from_be_bytes(value_bytes.try_into().unwrap());

        if reader.unsigned()? != 7 {
            return Err("invalid native transaction V2 gas_limit field".into());
        }
        let gas_limit = reader.unsigned()?;

        if reader.unsigned()? != 8 {
            return Err("invalid native transaction V2 max_fee_per_gas field".into());
        }
        let max_fee_bytes = reader.bytes()?;
        if max_fee_bytes.len() != 16 {
            return Err("native transaction V2 max_fee_per_gas must be exactly 16 bytes".into());
        }
        let max_fee_per_gas = u128::from_be_bytes(max_fee_bytes.try_into().unwrap());

        if reader.unsigned()? != 9 {
            return Err("invalid native transaction V2 max_priority_fee_per_gas field".into());
        }
        let priority_fee_bytes = reader.bytes()?;
        if priority_fee_bytes.len() != 16 {
            return Err(
                "native transaction V2 max_priority_fee_per_gas must be exactly 16 bytes".into(),
            );
        }
        let max_priority_fee_per_gas = u128::from_be_bytes(priority_fee_bytes.try_into().unwrap());

        if reader.unsigned()? != 10 {
            return Err("invalid native transaction V2 data field".into());
        }
        let data = reader.bytes()?.to_vec();

        if !reader.finished() {
            return Err("trailing bytes after native transaction V2 body".into());
        }

        let body = Self {
            network_id,
            chain_id,
            nonce,
            action,
            target_payload,
            value,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            data,
        };

        if body.canonical_bytes()? != bytes {
            return Err("native transaction V2 body is not canonically encoded".into());
        }

        Ok(body)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedNativeTransactionV2 {
    pub body: NativeTransactionBodyV2,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl SignedNativeTransactionV2 {
    pub fn canonical_body_bytes(&self) -> Result<Vec<u8>, String> {
        self.body.canonical_bytes()
    }

    pub fn signing_digest(&self) -> Result<Hash32, String> {
        let body = self.canonical_body_bytes()?;
        let network_id = u8::try_from(self.body.network_id)
            .map_err(|_| "native transaction V2 network_id does not fit one byte".to_string())?;

        if !matches!(network_id, 0x00..=0x02) {
            return Err(format!(
                "unsupported native transaction V2 network_id: {}",
                self.body.network_id
            ));
        }

        let mut preimage = Vec::with_capacity(
            b"NIAHCIA".len()
                + 1
                + NATIVE_TRANSACTION_SIGNING_PURPOSE_V2.len()
                + 1
                + 1
                + 1
                + body.len(),
        );
        preimage.extend_from_slice(b"NIAHCIA");
        preimage.push(0);
        preimage.extend_from_slice(NATIVE_TRANSACTION_SIGNING_PURPOSE_V2);
        preimage.push(0);
        preimage.push(network_id);
        preimage.push(0);
        preimage.extend_from_slice(&body);

        Ok(keccak256(&preimage))
    }

    fn canonical_verifying_key(&self) -> Result<VerifyingKey, String> {
        if self.public_key.len() != 65 || self.public_key.first() != Some(&0x04) {
            return Err(
                "native transaction V2 public key must be canonical uncompressed SEC1                  (65 bytes, 0x04 prefix)"
                    .into(),
            );
        }

        let verifying_key = VerifyingKey::from_sec1_bytes(&self.public_key)
            .map_err(|_| "invalid secp256k1 native transaction V2 public key".to_string())?;

        let canonical = verifying_key.to_encoded_point(false);
        if canonical.as_bytes() != self.public_key.as_slice() {
            return Err("non-canonical native transaction V2 public key".into());
        }

        Ok(verifying_key)
    }

    fn canonical_signature(&self) -> Result<Signature, String> {
        if self.signature.len() != 64 {
            return Err("native transaction V2 signature must be exactly 64 bytes".into());
        }

        let signature = Signature::from_slice(&self.signature)
            .map_err(|_| "invalid secp256k1 native transaction V2 signature".to_string())?;

        if signature.normalize_s().is_some() {
            return Err("native transaction V2 signature must use canonical low-S form".into());
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
            .map_err(|_| "invalid native transaction V2 signature".to_string())
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
            SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION_V2,
            self.canonical_payload()?,
        )
    }

    pub fn tx_id(&self) -> Result<Hash32, String> {
        let canonical = self.canonical_bytes()?;
        let mut preimage =
            Vec::with_capacity(NATIVE_TRANSACTION_TX_ID_DOMAIN_V2.len() + 1 + canonical.len());
        preimage.extend_from_slice(NATIVE_TRANSACTION_TX_ID_DOMAIN_V2);
        preimage.push(0);
        preimage.extend_from_slice(&canonical);
        Ok(keccak256(&preimage))
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            SIGNED_NATIVE_TRANSACTION_OBJECT_TYPE,
            SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION_V2,
        )?;

        if reader.map_len()? != 3 {
            return Err("signed native transaction V2 must contain exactly three fields".into());
        }

        if reader.unsigned()? != 1 {
            return Err("invalid signed native transaction V2 body field".into());
        }
        let body = NativeTransactionBodyV2::from_canonical_bytes(reader.bytes()?)?;

        if reader.unsigned()? != 2 {
            return Err("invalid signed native transaction V2 public_key field".into());
        }
        let public_key = reader.bytes()?.to_vec();

        if reader.unsigned()? != 3 {
            return Err("invalid signed native transaction V2 signature field".into());
        }
        let signature = reader.bytes()?.to_vec();

        if !reader.finished() {
            return Err("trailing bytes after signed native transaction V2".into());
        }

        let transaction = Self {
            body,
            public_key,
            signature,
        };
        transaction.canonical_verifying_key()?;
        transaction.canonical_signature()?;

        if transaction.canonical_bytes()? != bytes {
            return Err("signed native transaction V2 is not canonically encoded".into());
        }

        Ok(transaction)
    }
}

pub fn native_transactions_root_v2(
    transactions: &[SignedNativeTransactionV2],
) -> Result<Hash32, String> {
    let canonical = transactions
        .iter()
        .map(SignedNativeTransactionV2::canonical_bytes)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(transaction_merkle_root(&canonical))
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn transfer_body() -> NativeTransactionBodyV2 {
        NativeTransactionBodyV2 {
            network_id: DEVNET_NETWORK_ID,
            chain_id: DEVNET_CHAIN_ID,
            nonce: 7,
            action: NativeActionV2::Transfer,
            target_payload: vec![0x22; ADDRESS_PAYLOAD_LEN],
            value: 100_000_000,
            gas_limit: NATIVE_TRANSFER_GAS_V1,
            max_fee_per_gas: 25,
            max_priority_fee_per_gas: 5,
            data: Vec::new(),
        }
    }

    fn compute_body(action: NativeActionV2) -> NativeTransactionBodyV2 {
        NativeTransactionBodyV2 {
            network_id: DEVNET_NETWORK_ID,
            chain_id: DEVNET_CHAIN_ID,
            nonce: 8,
            action,
            target_payload: Vec::new(),
            value: 0,
            gas_limit: 0,
            max_fee_per_gas: 0,
            max_priority_fee_per_gas: 0,
            data: vec![0xa1, 0x01, 0x01],
        }
    }

    fn signed(mut body: NativeTransactionBodyV2) -> SignedNativeTransactionV2 {
        let signing_key = SigningKey::from_slice(&[0x01; 32]).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

        if matches!(body.action, NativeActionV2::ComputeChannelOpen) {
            body.value = 1_000;
        }

        let mut tx = SignedNativeTransactionV2 {
            body,
            public_key,
            signature: vec![0; 64],
        };
        let digest = tx.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        tx.signature = signature.to_bytes().to_vec();
        tx
    }

    #[test]
    fn action_values_are_exact() {
        assert_eq!(NativeActionV2::Transfer as u64, 0x00);
        assert_eq!(NativeActionV2::ContractCall as u64, 0x01);
        assert_eq!(NativeActionV2::ContractCreate as u64, 0x02);
        assert_eq!(NativeActionV2::ComputeChannelOpen as u64, 0x10);
        assert_eq!(NativeActionV2::ComputeChannelSettle as u64, 0x11);
        assert_eq!(NativeActionV2::ComputeChannelRefund as u64, 0x12);
        assert!(NativeActionV2::try_from(0x13).is_err());
    }

    #[test]
    fn v2_transfer_body_round_trips_without_changing_v1_shape() {
        let body = transfer_body();
        let bytes = body.canonical_bytes().unwrap();
        let decoded = NativeTransactionBodyV2::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(decoded, body);
        assert!(decoded.validate(AddressNetwork::Devnet).is_ok());
    }

    #[test]
    fn compute_action_shape_requires_empty_target_and_nonempty_data() {
        for action in [
            NativeActionV2::ComputeChannelOpen,
            NativeActionV2::ComputeChannelSettle,
            NativeActionV2::ComputeChannelRefund,
        ] {
            let body = compute_body(action);
            assert!(body.validate_action_shape().is_ok());

            let mut with_target = body.clone();
            with_target.target_payload = vec![0_u8; 20];
            assert!(with_target.validate_action_shape().is_err());

            let mut without_data = body;
            without_data.data.clear();
            assert!(without_data.validate_action_shape().is_err());
        }
    }

    #[test]
    fn v2_signed_transaction_round_trips_and_verifies() {
        let tx = signed(transfer_body());
        tx.verify_signature(AddressNetwork::Devnet).unwrap();

        let bytes = tx.canonical_bytes().unwrap();
        let decoded = SignedNativeTransactionV2::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(decoded, tx);
        decoded.verify_signature(AddressNetwork::Devnet).unwrap();
    }

    #[test]
    fn v2_domains_are_distinct_from_v1() {
        let v2 = signed(transfer_body());
        let v2_digest = v2.signing_digest().unwrap();

        let v1_body = crate::native_transaction::NativeTransactionBodyV1 {
            network_id: v2.body.network_id,
            chain_id: v2.body.chain_id,
            nonce: v2.body.nonce,
            action: crate::native_transaction::NativeActionV1::Transfer,
            target_payload: v2.body.target_payload.clone(),
            value: v2.body.value,
            gas_limit: v2.body.gas_limit,
            max_fee_per_gas: v2.body.max_fee_per_gas,
            max_priority_fee_per_gas: v2.body.max_priority_fee_per_gas,
            data: v2.body.data.clone(),
        };
        let signing_key = SigningKey::from_slice(&[0x01; 32]).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        let mut v1 = crate::native_transaction::SignedNativeTransactionV1 {
            body: v1_body,
            public_key,
            signature: vec![0; 64],
        };
        let v1_digest = v1.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&v1_digest).unwrap();
        v1.signature = signature.to_bytes().to_vec();

        assert_ne!(v2_digest, v1_digest);
        assert_ne!(v2.tx_id().unwrap(), v1.tx_id().unwrap());
        assert_ne!(v2.canonical_bytes().unwrap(), v1.canonical_bytes().unwrap());
    }

    #[test]
    fn v1_decoder_rejects_v2_and_v2_decoder_rejects_v1() {
        let v2 = signed(transfer_body());
        assert!(
            crate::native_transaction::SignedNativeTransactionV1::from_canonical_bytes(
                &v2.canonical_bytes().unwrap()
            )
            .is_err()
        );

        let signing_key = SigningKey::from_slice(&[0x01; 32]).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        let mut v1 = crate::native_transaction::SignedNativeTransactionV1 {
            body: crate::native_transaction::NativeTransactionBodyV1 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 7,
                action: crate::native_transaction::NativeActionV1::Transfer,
                target_payload: vec![0x22; ADDRESS_PAYLOAD_LEN],
                value: 100_000_000,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 25,
                max_priority_fee_per_gas: 5,
                data: Vec::new(),
            },
            public_key,
            signature: vec![0; 64],
        };
        let digest = v1.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        v1.signature = signature.to_bytes().to_vec();

        assert!(
            SignedNativeTransactionV2::from_canonical_bytes(&v1.canonical_bytes().unwrap())
                .is_err()
        );
    }

    #[test]
    fn native_transaction_v2_vector_matches_locked_json() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/native-transaction-v2.json"
        ))
        .unwrap();

        let transfer = &vectors["transfer"];
        let body = transfer_body();
        assert_eq!(
            hex::encode(body.canonical_bytes().unwrap()),
            transfer["body_canonical_hex"].as_str().unwrap()
        );

        let signing_key_bytes = hex::decode(transfer["test_private_key"].as_str().unwrap()).unwrap();
        let signing_key = SigningKey::from_slice(&signing_key_bytes).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        assert_eq!(
            hex::encode(&public_key),
            transfer["public_key"].as_str().unwrap()
        );

        let mut transaction = SignedNativeTransactionV2 {
            body,
            public_key,
            signature: vec![0; 64],
        };

        assert_eq!(
            hex::encode(transaction.signing_digest().unwrap()),
            transfer["signing_digest"].as_str().unwrap()
        );

        let signature: Signature = signing_key
            .sign_prehash(&transaction.signing_digest().unwrap())
            .unwrap();
        transaction.signature = signature.to_bytes().to_vec();

        assert_eq!(
            hex::encode(&transaction.signature),
            transfer["signature"].as_str().unwrap()
        );
        assert_eq!(
            hex::encode(transaction.canonical_bytes().unwrap()),
            transfer["signed_canonical_hex"].as_str().unwrap()
        );
        assert_eq!(
            hex::encode(transaction.tx_id().unwrap()),
            transfer["tx_id"].as_str().unwrap()
        );

        let sender = transaction
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap();
        assert_eq!(
            hex::encode(sender.payload),
            transfer["sender_payload"].as_str().unwrap()
        );
        assert_eq!(
            sender.to_string(),
            transfer["sender_address"].as_str().unwrap()
        );

        let open = &vectors["compute_channel_open_body"];
        let open_body = NativeTransactionBodyV2 {
            network_id: DEVNET_NETWORK_ID,
            chain_id: DEVNET_CHAIN_ID,
            nonce: 8,
            action: NativeActionV2::ComputeChannelOpen,
            target_payload: Vec::new(),
            value: 1_000,
            gas_limit: 0,
            max_fee_per_gas: 0,
            max_priority_fee_per_gas: 0,
            data: hex::decode(open["data"].as_str().unwrap()).unwrap(),
        };
        assert_eq!(
            hex::encode(open_body.canonical_bytes().unwrap()),
            open["body_canonical_hex"].as_str().unwrap()
        );

        let decoded = NativeTransactionBodyV2::from_canonical_bytes(
            &hex::decode(open["body_canonical_hex"].as_str().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded, open_body);
        assert_eq!(decoded.action as u64, 0x10);
    }

    #[test]
    fn v2_wrong_network_and_signature_tampering_are_rejected() {
        let tx = signed(transfer_body());
        assert!(tx.verify_signature(AddressNetwork::Mainnet).is_err());

        let mut tampered = tx;
        tampered.signature[0] ^= 1;
        assert!(tampered.verify_signature(AddressNetwork::Devnet).is_err());
    }
}
