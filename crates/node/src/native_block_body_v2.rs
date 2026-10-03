use crate::native_block_body::NATIVE_BLOCK_BODY_OBJECT_TYPE;
use crate::native_transaction::{
    SignedNativeTransactionV1, SIGNED_NATIVE_TRANSACTION_OBJECT_TYPE,
    SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION,
};
use crate::native_transaction_v2::{
    SignedNativeTransactionV2, SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION_V2,
};
use crate::nce::{
    decode_envelope, encode_array, encode_bytes, encode_envelope, encode_map, envelope_identity,
};
use crate::work::{transaction_merkle_root, Address20, Hash32};

pub const NATIVE_BLOCK_BODY_SCHEMA_VERSION_V2: u64 = 2;
pub const MAX_BLOCK_BODY_TRANSACTIONS_V2: usize = 65_535;
pub const MAX_BLOCK_BODY_TRANSACTION_BYTES_V2: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VersionedSignedNativeTransaction {
    V1(SignedNativeTransactionV1),
    V2(SignedNativeTransactionV2),
}

impl VersionedSignedNativeTransaction {
    pub fn schema_version(&self) -> u64 {
        match self {
            Self::V1(_) => SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION,
            Self::V2(_) => SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION_V2,
        }
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        match self {
            Self::V1(transaction) => transaction.canonical_bytes(),
            Self::V2(transaction) => transaction.canonical_bytes(),
        }
    }
}

pub fn decode_versioned_signed_native_transaction(
    bytes: &[u8],
) -> Result<VersionedSignedNativeTransaction, String> {
    let (object_type, schema_version) = envelope_identity(bytes)?;
    if object_type != SIGNED_NATIVE_TRANSACTION_OBJECT_TYPE {
        return Err("native block body V2 transaction has unexpected object type".into());
    }

    let transaction = match schema_version {
        SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION => VersionedSignedNativeTransaction::V1(
            SignedNativeTransactionV1::from_canonical_bytes(bytes)?,
        ),
        SIGNED_NATIVE_TRANSACTION_SCHEMA_VERSION_V2 => VersionedSignedNativeTransaction::V2(
            SignedNativeTransactionV2::from_canonical_bytes(bytes)?,
        ),
        other => {
            return Err(format!(
                "unsupported signed native transaction schema version in block body V2: {other}"
            ))
        }
    };

    if transaction.canonical_bytes()?.as_slice() != bytes {
        return Err("native block body V2 contains non-canonical transaction bytes".into());
    }

    Ok(transaction)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeBlockBodyV2 {
    pub producer_fee_recipient: Address20,
    pub transactions: Vec<Vec<u8>>,
}

impl NativeBlockBodyV2 {
    pub fn empty() -> Self {
        Self {
            producer_fee_recipient: [0_u8; 20],
            transactions: Vec::new(),
        }
    }

    pub fn from_versioned_transactions(
        producer_fee_recipient: Address20,
        transactions: &[VersionedSignedNativeTransaction],
    ) -> Result<Self, String> {
        let canonical = transactions
            .iter()
            .map(VersionedSignedNativeTransaction::canonical_bytes)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(producer_fee_recipient, canonical)
    }

    pub fn new(
        producer_fee_recipient: Address20,
        transactions: Vec<Vec<u8>>,
    ) -> Result<Self, String> {
        validate_transaction_bytes_v2(&transactions)?;
        Ok(Self {
            producer_fee_recipient,
            transactions,
        })
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        validate_transaction_bytes_v2(&self.transactions)?;

        let transactions = self
            .transactions
            .iter()
            .map(|transaction| encode_bytes(transaction))
            .collect::<Vec<_>>();

        encode_map(&[
            (1, encode_bytes(&self.producer_fee_recipient)),
            (2, encode_array(&transactions)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            NATIVE_BLOCK_BODY_OBJECT_TYPE,
            NATIVE_BLOCK_BODY_SCHEMA_VERSION_V2,
            self.canonical_payload()?,
        )
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            NATIVE_BLOCK_BODY_OBJECT_TYPE,
            NATIVE_BLOCK_BODY_SCHEMA_VERSION_V2,
        )?;

        if reader.map_len()? != 2 {
            return Err("native block body V2 must contain exactly two fields".into());
        }

        if reader.unsigned()? != 1 {
            return Err("invalid native block body V2 producer fee recipient field".into());
        }
        let recipient = reader.bytes()?;
        if recipient.len() != 20 {
            return Err(
                "native block body V2 producer fee recipient must be exactly 20 bytes".into(),
            );
        }
        let producer_fee_recipient: Address20 = recipient.try_into().unwrap();

        if reader.unsigned()? != 2 {
            return Err("invalid native block body V2 transactions field".into());
        }
        let count = reader.array_len()?;
        if count > MAX_BLOCK_BODY_TRANSACTIONS_V2 {
            return Err("native block body V2 exceeds transaction count limit".into());
        }

        let mut transactions = Vec::with_capacity(count);
        for _ in 0..count {
            let transaction = reader.bytes()?;
            if transaction.is_empty() {
                return Err("native block body V2 contains an empty transaction".into());
            }
            if transaction.len() > MAX_BLOCK_BODY_TRANSACTION_BYTES_V2 {
                return Err("native block body V2 transaction exceeds byte limit".into());
            }

            decode_versioned_signed_native_transaction(transaction)?;
            transactions.push(transaction.to_vec());
        }

        if !reader.finished() {
            return Err("trailing bytes after native block body V2".into());
        }

        Ok(Self {
            producer_fee_recipient,
            transactions,
        })
    }

    pub fn decoded_transactions(&self) -> Result<Vec<VersionedSignedNativeTransaction>, String> {
        self.transactions
            .iter()
            .map(|transaction| decode_versioned_signed_native_transaction(transaction))
            .collect()
    }

    pub fn transactions_root(&self) -> Hash32 {
        transaction_merkle_root(&self.transactions)
    }

    pub fn validate_fee_recipient_canonicality(
        &self,
        total_producer_priority_fee: u128,
    ) -> Result<(), String> {
        if total_producer_priority_fee == 0 && self.producer_fee_recipient != [0_u8; 20] {
            return Err(
                "zero producer priority fee requires zero native block body V2 fee recipient"
                    .into(),
            );
        }
        Ok(())
    }
}

fn validate_transaction_bytes_v2(transactions: &[Vec<u8>]) -> Result<(), String> {
    if transactions.len() > MAX_BLOCK_BODY_TRANSACTIONS_V2 {
        return Err("native block body V2 exceeds transaction count limit".into());
    }

    for transaction in transactions {
        if transaction.is_empty() {
            return Err("native block body V2 contains an empty transaction".into());
        }
        if transaction.len() > MAX_BLOCK_BODY_TRANSACTION_BYTES_V2 {
            return Err("native block body V2 transaction exceeds byte limit".into());
        }
        decode_versioned_signed_native_transaction(transaction)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_transaction::{
        NativeActionV1, NativeTransactionBodyV1, DEVNET_CHAIN_ID, DEVNET_NETWORK_ID,
        NATIVE_TRANSFER_GAS_V1,
    };
    use crate::native_transaction_v2::{NativeActionV2, NativeTransactionBodyV2};
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn signed_v1(signing_byte: u8) -> SignedNativeTransactionV1 {
        let signing_key = SigningKey::from_slice(&[signing_byte; 32]).unwrap();
        let mut transaction = SignedNativeTransactionV1 {
            body: NativeTransactionBodyV1 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV1::Transfer,
                target_payload: vec![0x22; 20],
                value: 1,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 1,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
            public_key: signing_key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            signature: vec![0; 64],
        };
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        transaction
    }

    fn signed_v2(signing_byte: u8) -> SignedNativeTransactionV2 {
        let signing_key = SigningKey::from_slice(&[signing_byte; 32]).unwrap();
        let mut transaction = SignedNativeTransactionV2 {
            body: NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 1,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x33; 20],
                value: 2,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 1,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
            public_key: signing_key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            signature: vec![0; 64],
        };
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        transaction
    }

    #[test]
    fn body_v2_round_trips_mixed_transaction_versions() {
        let v1 = VersionedSignedNativeTransaction::V1(signed_v1(0x11));
        let v2 = VersionedSignedNativeTransaction::V2(signed_v2(0x12));
        let body =
            NativeBlockBodyV2::from_versioned_transactions([0_u8; 20], &[v1.clone(), v2.clone()])
                .unwrap();

        let encoded = body.canonical_bytes().unwrap();
        let decoded = NativeBlockBodyV2::from_canonical_bytes(&encoded).unwrap();

        assert_eq!(decoded, body);
        assert_eq!(decoded.canonical_bytes().unwrap(), encoded);
        assert_eq!(decoded.decoded_transactions().unwrap(), vec![v1, v2]);
    }

    #[test]
    fn body_v2_uses_explicit_signed_transaction_schema_identity() {
        let v1 = signed_v1(0x21).canonical_bytes().unwrap();
        let v2 = signed_v2(0x22).canonical_bytes().unwrap();

        assert!(matches!(
            decode_versioned_signed_native_transaction(&v1).unwrap(),
            VersionedSignedNativeTransaction::V1(_)
        ));
        assert!(matches!(
            decode_versioned_signed_native_transaction(&v2).unwrap(),
            VersionedSignedNativeTransaction::V2(_)
        ));
    }

    #[test]
    fn body_v2_transaction_root_commits_exact_versioned_bytes_and_order() {
        let v1 = signed_v1(0x31).canonical_bytes().unwrap();
        let v2 = signed_v2(0x32).canonical_bytes().unwrap();
        let first = NativeBlockBodyV2::new([0_u8; 20], vec![v1.clone(), v2.clone()]).unwrap();
        let second = NativeBlockBodyV2::new([0_u8; 20], vec![v2, v1]).unwrap();

        assert_ne!(first.transactions_root(), second.transactions_root());
        assert_eq!(
            first.transactions_root(),
            transaction_merkle_root(&first.transactions)
        );
    }

    #[test]
    fn body_v1_rejects_v2_transaction_bytes() {
        let v2 = signed_v2(0x41).canonical_bytes().unwrap();
        assert!(crate::native_block_body::NativeBlockBodyV1::new([0_u8; 20], vec![v2]).is_err());
    }

    #[test]
    fn body_v2_rejects_wrong_envelope_object_type() {
        let transaction = signed_v1(0x51);
        let body_bytes = transaction.body.canonical_bytes().unwrap();

        let error = NativeBlockBodyV2::new([0_u8; 20], vec![body_bytes]).unwrap_err();
        assert!(error.contains("unexpected object type"));
    }

    #[test]
    fn empty_body_v2_is_canonical_and_zero_fee_recipient_is_required() {
        let body = NativeBlockBodyV2::empty();
        let encoded = body.canonical_bytes().unwrap();
        assert_eq!(
            NativeBlockBodyV2::from_canonical_bytes(&encoded).unwrap(),
            body
        );
        body.validate_fee_recipient_canonicality(0).unwrap();

        let nonzero = NativeBlockBodyV2::new([0x01; 20], Vec::new()).unwrap();
        assert!(nonzero.validate_fee_recipient_canonicality(0).is_err());
    }
}
