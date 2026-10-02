use crate::native_transaction::SignedNativeTransactionV1;
use crate::nce::{decode_envelope, encode_array, encode_bytes, encode_envelope, encode_map};
use crate::work::Address20;

pub const NATIVE_BLOCK_BODY_OBJECT_TYPE: u64 = 0x0017;
pub const NATIVE_BLOCK_BODY_SCHEMA_VERSION: u64 = 1;
pub const MAX_BLOCK_BODY_TRANSACTIONS_V1: usize = 65_535;
pub const MAX_BLOCK_BODY_TRANSACTION_BYTES_V1: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeBlockBodyV1 {
    pub producer_fee_recipient: Address20,
    pub transactions: Vec<Vec<u8>>,
}

impl NativeBlockBodyV1 {
    pub fn empty() -> Self {
        Self {
            producer_fee_recipient: [0_u8; 20],
            transactions: Vec::new(),
        }
    }

    pub fn from_transactions(
        producer_fee_recipient: Address20,
        transactions: &[SignedNativeTransactionV1],
    ) -> Result<Self, String> {
        let canonical = transactions
            .iter()
            .map(SignedNativeTransactionV1::canonical_bytes)
            .collect::<Result<Vec<_>, _>>()?;

        Self::new(producer_fee_recipient, canonical)
    }

    pub fn new(
        producer_fee_recipient: Address20,
        transactions: Vec<Vec<u8>>,
    ) -> Result<Self, String> {
        validate_transaction_bytes(&transactions)?;
        Ok(Self {
            producer_fee_recipient,
            transactions,
        })
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        validate_transaction_bytes(&self.transactions)?;

        let transactions = self
            .transactions
            .iter()
            .map(|tx| encode_bytes(tx))
            .collect::<Vec<_>>();

        encode_map(&[
            (1, encode_bytes(&self.producer_fee_recipient)),
            (2, encode_array(&transactions)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            NATIVE_BLOCK_BODY_OBJECT_TYPE,
            NATIVE_BLOCK_BODY_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            NATIVE_BLOCK_BODY_OBJECT_TYPE,
            NATIVE_BLOCK_BODY_SCHEMA_VERSION,
        )?;

        if reader.map_len()? != 2 {
            return Err("native block body must contain exactly two fields".into());
        }

        if reader.unsigned()? != 1 {
            return Err("invalid native block body producer fee recipient field".into());
        }
        let recipient = reader.bytes()?;
        if recipient.len() != 20 {
            return Err("native block body producer fee recipient must be exactly 20 bytes".into());
        }
        let producer_fee_recipient: Address20 = recipient.try_into().unwrap();

        if reader.unsigned()? != 2 {
            return Err("invalid native block body transactions field".into());
        }
        let count = reader.array_len()?;
        if count > MAX_BLOCK_BODY_TRANSACTIONS_V1 {
            return Err("native block body exceeds transaction count limit".into());
        }

        let mut transactions = Vec::with_capacity(count);
        for _ in 0..count {
            let tx = reader.bytes()?;
            if tx.is_empty() {
                return Err("native block body contains an empty transaction".into());
            }
            if tx.len() > MAX_BLOCK_BODY_TRANSACTION_BYTES_V1 {
                return Err("native block body transaction exceeds byte limit".into());
            }

            let decoded = SignedNativeTransactionV1::from_canonical_bytes(tx)?;
            let canonical = decoded.canonical_bytes()?;
            if canonical.as_slice() != tx {
                return Err("native block body contains non-canonical transaction bytes".into());
            }

            transactions.push(tx.to_vec());
        }

        if !reader.finished() {
            return Err("trailing bytes after native block body".into());
        }

        Ok(Self {
            producer_fee_recipient,
            transactions,
        })
    }

    pub fn decoded_transactions(&self) -> Result<Vec<SignedNativeTransactionV1>, String> {
        self.transactions
            .iter()
            .map(|tx| SignedNativeTransactionV1::from_canonical_bytes(tx))
            .collect()
    }

    pub fn validate_fee_recipient_canonicality(
        &self,
        total_producer_priority_fee: u128,
    ) -> Result<(), String> {
        if total_producer_priority_fee == 0 && self.producer_fee_recipient != [0_u8; 20] {
            return Err(
                "zero producer priority fee requires zero native block body fee recipient".into(),
            );
        }
        Ok(())
    }
}

fn validate_transaction_bytes(transactions: &[Vec<u8>]) -> Result<(), String> {
    if transactions.len() > MAX_BLOCK_BODY_TRANSACTIONS_V1 {
        return Err("native block body exceeds transaction count limit".into());
    }

    for tx in transactions {
        if tx.is_empty() {
            return Err("native block body contains an empty transaction".into());
        }
        if tx.len() > MAX_BLOCK_BODY_TRANSACTION_BYTES_V1 {
            return Err("native block body transaction exceeds byte limit".into());
        }

        let decoded = SignedNativeTransactionV1::from_canonical_bytes(tx)?;
        if decoded.canonical_bytes()?.as_slice() != tx {
            return Err("native block body contains non-canonical transaction bytes".into());
        }
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
    use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    fn signed_transfer(signing_byte: u8) -> SignedNativeTransactionV1 {
        let signing_key = SigningKey::from_slice(&[signing_byte; 32]).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

        let mut tx = SignedNativeTransactionV1 {
            body: NativeTransactionBodyV1 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV1::Transfer,
                target_payload: vec![0x22; 20],
                value: 1,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
            public_key,
            signature: vec![0; 64],
        };

        let digest = tx.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        tx.signature = signature.to_bytes().to_vec();
        tx
    }

    #[test]
    fn empty_body_round_trips_exactly() {
        let body = NativeBlockBodyV1::empty();
        let encoded = body.canonical_bytes().unwrap();
        let decoded = NativeBlockBodyV1::from_canonical_bytes(&encoded).unwrap();
        assert_eq!(decoded, body);
    }

    #[test]
    fn body_preserves_exact_ordered_transaction_bytes() {
        let first = signed_transfer(1);
        let second = signed_transfer(2);
        let first_bytes = first.canonical_bytes().unwrap();
        let second_bytes = second.canonical_bytes().unwrap();

        let body = NativeBlockBodyV1::from_transactions([0x44; 20], &[first, second]).unwrap();
        assert_eq!(
            body.transactions,
            vec![first_bytes.clone(), second_bytes.clone()]
        );

        let decoded =
            NativeBlockBodyV1::from_canonical_bytes(&body.canonical_bytes().unwrap()).unwrap();
        assert_eq!(decoded.transactions, vec![first_bytes, second_bytes]);
    }

    #[test]
    fn transaction_order_changes_canonical_body() {
        let first = signed_transfer(1).canonical_bytes().unwrap();
        let second = signed_transfer(2).canonical_bytes().unwrap();

        let a = NativeBlockBodyV1::new([0x44; 20], vec![first.clone(), second.clone()])
            .unwrap()
            .canonical_bytes()
            .unwrap();
        let b = NativeBlockBodyV1::new([0x44; 20], vec![second, first])
            .unwrap()
            .canonical_bytes()
            .unwrap();

        assert_ne!(a, b);
    }

    #[test]
    fn zero_priority_fee_requires_zero_recipient() {
        NativeBlockBodyV1::empty()
            .validate_fee_recipient_canonicality(0)
            .unwrap();

        let body = NativeBlockBodyV1::new([0x44; 20], Vec::new()).unwrap();
        assert!(body.validate_fee_recipient_canonicality(0).is_err());
        body.validate_fee_recipient_canonicality(1).unwrap();
    }

    #[test]
    fn decoder_rejects_trailing_bytes() {
        let mut encoded = NativeBlockBodyV1::empty().canonical_bytes().unwrap();
        encoded.push(0);
        assert!(NativeBlockBodyV1::from_canonical_bytes(&encoded).is_err());
    }
}
