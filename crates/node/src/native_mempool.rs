use crate::address::AddressNetwork;
use crate::native_transaction::SignedNativeTransactionV1;
use crate::work::Hash32;
use std::collections::BTreeMap;

pub const DEFAULT_MAX_MEMPOOL_TRANSACTIONS: usize = 10_000;
pub const DEFAULT_MAX_TRANSACTION_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMempoolEntryV1 {
    pub tx_id: Hash32,
    pub canonical_bytes: Vec<u8>,
    pub transaction: SignedNativeTransactionV1,
}

#[derive(Debug)]
pub struct NativeMempoolV1 {
    network: AddressNetwork,
    max_transactions: usize,
    max_transaction_bytes: usize,
    entries: BTreeMap<Hash32, NativeMempoolEntryV1>,
}

impl NativeMempoolV1 {
    pub fn new(network: AddressNetwork) -> Self {
        Self::with_limits(
            network,
            DEFAULT_MAX_MEMPOOL_TRANSACTIONS,
            DEFAULT_MAX_TRANSACTION_BYTES,
        )
    }

    pub fn with_limits(
        network: AddressNetwork,
        max_transactions: usize,
        max_transaction_bytes: usize,
    ) -> Self {
        Self {
            network,
            max_transactions,
            max_transaction_bytes,
            entries: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, tx_id: &Hash32) -> bool {
        self.entries.contains_key(tx_id)
    }

    pub fn get(&self, tx_id: &Hash32) -> Option<&NativeMempoolEntryV1> {
        self.entries.get(tx_id)
    }

    pub fn admit_canonical_bytes(&mut self, bytes: &[u8]) -> Result<Hash32, String> {
        if bytes.len() > self.max_transaction_bytes {
            return Err(format!(
                "native transaction exceeds mempool byte limit: {} > {}",
                bytes.len(),
                self.max_transaction_bytes
            ));
        }

        let transaction = SignedNativeTransactionV1::from_canonical_bytes(bytes)?;
        self.admit_transaction(transaction)
    }

    pub fn admit_transaction(
        &mut self,
        transaction: SignedNativeTransactionV1,
    ) -> Result<Hash32, String> {
        transaction.verify_signature(self.network)?;

        let canonical_bytes = transaction.canonical_bytes()?;
        if canonical_bytes.len() > self.max_transaction_bytes {
            return Err(format!(
                "native transaction exceeds mempool byte limit: {} > {}",
                canonical_bytes.len(),
                self.max_transaction_bytes
            ));
        }

        let tx_id = transaction.tx_id()?;

        if self.entries.contains_key(&tx_id) {
            return Err(format!(
                "duplicate native transaction: {}",
                hex::encode(tx_id)
            ));
        }

        if self.entries.len() >= self.max_transactions {
            return Err("native mempool transaction limit reached".into());
        }

        self.entries.insert(
            tx_id,
            NativeMempoolEntryV1 {
                tx_id,
                canonical_bytes,
                transaction,
            },
        );

        Ok(tx_id)
    }

    pub fn remove(&mut self, tx_id: &Hash32) -> Option<NativeMempoolEntryV1> {
        self.entries.remove(tx_id)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn ordered_entries(&self) -> impl Iterator<Item = &NativeMempoolEntryV1> {
        self.entries.values()
    }

    pub fn ordered_transactions(&self) -> Vec<SignedNativeTransactionV1> {
        self.entries
            .values()
            .map(|entry| entry.transaction.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_transaction::{
        NativeActionV1, NativeTransactionBodyV1, DEVNET_CHAIN_ID, DEVNET_NETWORK_ID,
        NATIVE_TRANSFER_GAS_V1,
    };
    use k256::ecdsa::{
        signature::hazmat::PrehashSigner, Signature, SigningKey,
    };

    fn signed_transfer(nonce: u64, signing_byte: u8) -> SignedNativeTransactionV1 {
        let signing_key = SigningKey::from_slice(&[signing_byte; 32]).unwrap();
        let public_key = signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

        let mut transaction = SignedNativeTransactionV1 {
            body: NativeTransactionBodyV1 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce,
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

        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        transaction
    }

    #[test]
    fn admits_canonical_signed_transaction_bytes() {
        let transaction = signed_transfer(0, 1);
        let canonical = transaction.canonical_bytes().unwrap();
        let expected_id = transaction.tx_id().unwrap();

        let mut mempool = NativeMempoolV1::new(AddressNetwork::Devnet);
        let tx_id = mempool.admit_canonical_bytes(&canonical).unwrap();

        assert_eq!(tx_id, expected_id);
        assert_eq!(mempool.len(), 1);
        assert!(mempool.contains(&expected_id));
        assert_eq!(mempool.get(&expected_id).unwrap().canonical_bytes, canonical);
    }

    #[test]
    fn rejects_duplicate_transaction_id() {
        let transaction = signed_transfer(0, 1);
        let mut mempool = NativeMempoolV1::new(AddressNetwork::Devnet);

        mempool.admit_transaction(transaction.clone()).unwrap();
        assert!(mempool.admit_transaction(transaction).is_err());
        assert_eq!(mempool.len(), 1);
    }

    #[test]
    fn rejects_wrong_network_transaction() {
        let transaction = signed_transfer(0, 1);
        let mut mempool = NativeMempoolV1::new(AddressNetwork::Mainnet);

        assert!(mempool.admit_transaction(transaction).is_err());
        assert!(mempool.is_empty());
    }

    #[test]
    fn rejects_tampered_signature() {
        let mut transaction = signed_transfer(0, 1);
        transaction.signature[0] ^= 1;

        let mut mempool = NativeMempoolV1::new(AddressNetwork::Devnet);
        assert!(mempool.admit_transaction(transaction).is_err());
        assert!(mempool.is_empty());
    }

    #[test]
    fn rejects_noncanonical_or_trailing_bytes() {
        let transaction = signed_transfer(0, 1);
        let mut canonical = transaction.canonical_bytes().unwrap();
        canonical.push(0);

        let mut mempool = NativeMempoolV1::new(AddressNetwork::Devnet);
        assert!(mempool.admit_canonical_bytes(&canonical).is_err());
        assert!(mempool.is_empty());
    }

    #[test]
    fn enforces_transaction_count_limit() {
        let mut mempool = NativeMempoolV1::with_limits(AddressNetwork::Devnet, 1, 1024 * 1024);

        mempool.admit_transaction(signed_transfer(0, 1)).unwrap();
        assert!(mempool.admit_transaction(signed_transfer(0, 2)).is_err());
        assert_eq!(mempool.len(), 1);
    }

    #[test]
    fn enforces_transaction_byte_limit_before_decode() {
        let mut mempool = NativeMempoolV1::with_limits(AddressNetwork::Devnet, 10, 4);

        assert!(mempool.admit_canonical_bytes(&[0u8; 5]).is_err());
        assert!(mempool.is_empty());
    }

    #[test]
    fn deterministic_iteration_is_transaction_id_order() {
        let first = signed_transfer(0, 1);
        let second = signed_transfer(0, 2);
        let first_id = first.tx_id().unwrap();
        let second_id = second.tx_id().unwrap();

        let mut mempool = NativeMempoolV1::new(AddressNetwork::Devnet);
        mempool.admit_transaction(second).unwrap();
        mempool.admit_transaction(first).unwrap();

        let ids = mempool
            .ordered_entries()
            .map(|entry| entry.tx_id)
            .collect::<Vec<_>>();

        let mut expected = vec![first_id, second_id];
        expected.sort();

        assert_eq!(ids, expected);
    }

    #[test]
    fn remove_returns_entry_and_updates_membership() {
        let transaction = signed_transfer(0, 1);
        let tx_id = transaction.tx_id().unwrap();

        let mut mempool = NativeMempoolV1::new(AddressNetwork::Devnet);
        mempool.admit_transaction(transaction).unwrap();

        let removed = mempool.remove(&tx_id).unwrap();
        assert_eq!(removed.tx_id, tx_id);
        assert!(!mempool.contains(&tx_id));
        assert!(mempool.is_empty());
    }
}
