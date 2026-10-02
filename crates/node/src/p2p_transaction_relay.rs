use crate::native_mempool::NativeMempoolV1;
use crate::work::Hash32;

pub const MAX_TX_INV_ITEMS_V1: usize = 1024;
pub const MAX_TX_REQUEST_ITEMS_V1: usize = 1024;
pub const MAX_TX_BODY_ITEMS_V1: usize = 256;
pub const MAX_RELAY_TRANSACTION_BYTES_V1: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxInvV1 {
    pub tx_ids: Vec<Hash32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetTxV1 {
    pub tx_ids: Vec<Hash32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxV1 {
    pub canonical_transactions: Vec<Vec<u8>>,
}

impl TxInvV1 {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        encode_ids(&self.tx_ids, MAX_TX_INV_ITEMS_V1, "TxInvV1")
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        Ok(Self {
            tx_ids: decode_ids(bytes, MAX_TX_INV_ITEMS_V1, "TxInvV1")?,
        })
    }
}

impl GetTxV1 {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        encode_ids(&self.tx_ids, MAX_TX_REQUEST_ITEMS_V1, "GetTxV1")
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        Ok(Self {
            tx_ids: decode_ids(bytes, MAX_TX_REQUEST_ITEMS_V1, "GetTxV1")?,
        })
    }
}

impl TxV1 {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        if self.canonical_transactions.len() > MAX_TX_BODY_ITEMS_V1 {
            return Err("TxV1 exceeds transaction batch limit".into());
        }

        let mut out = Vec::new();
        out.extend_from_slice(&(self.canonical_transactions.len() as u16).to_be_bytes());

        for tx in &self.canonical_transactions {
            if tx.is_empty() {
                return Err("TxV1 contains an empty transaction body".into());
            }
            if tx.len() > MAX_RELAY_TRANSACTION_BYTES_V1 {
                return Err("TxV1 transaction exceeds relay byte limit".into());
            }
            let len = u32::try_from(tx.len())
                .map_err(|_| "TxV1 transaction length exceeds u32".to_string())?;
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(tx);
        }

        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(bytes);
        let count = cursor.u16()? as usize;
        if count > MAX_TX_BODY_ITEMS_V1 {
            return Err("TxV1 exceeds transaction batch limit".into());
        }

        let mut canonical_transactions = Vec::with_capacity(count);
        for _ in 0..count {
            let len = cursor.u32()? as usize;
            if len == 0 {
                return Err("TxV1 contains an empty transaction body".into());
            }
            if len > MAX_RELAY_TRANSACTION_BYTES_V1 {
                return Err("TxV1 transaction exceeds relay byte limit".into());
            }
            canonical_transactions.push(cursor.bytes(len)?.to_vec());
        }

        if !cursor.finished() {
            return Err("TxV1 contains trailing bytes".into());
        }

        Ok(Self {
            canonical_transactions,
        })
    }
}

pub fn inventory_for_mempool(pool: &NativeMempoolV1) -> TxInvV1 {
    TxInvV1 {
        tx_ids: pool
            .ordered_entries()
            .take(MAX_TX_INV_ITEMS_V1)
            .map(|entry| entry.tx_id)
            .collect(),
    }
}

pub fn missing_from_inventory(pool: &NativeMempoolV1, inventory: &TxInvV1) -> GetTxV1 {
    GetTxV1 {
        tx_ids: inventory
            .tx_ids
            .iter()
            .copied()
            .filter(|tx_id| !pool.contains(tx_id))
            .take(MAX_TX_REQUEST_ITEMS_V1)
            .collect(),
    }
}

pub fn transactions_for_request(
    pool: &NativeMempoolV1,
    request: &GetTxV1,
) -> Result<TxV1, String> {
    if request.tx_ids.len() > MAX_TX_REQUEST_ITEMS_V1 {
        return Err("GetTxV1 exceeds transaction request limit".into());
    }

    let mut canonical_transactions = Vec::new();
    for tx_id in request.tx_ids.iter().take(MAX_TX_BODY_ITEMS_V1) {
        if let Some(entry) = pool.get(tx_id) {
            canonical_transactions.push(entry.canonical_bytes.clone());
        }
    }

    Ok(TxV1 {
        canonical_transactions,
    })
}

pub fn admit_relay_transactions(
    pool: &mut NativeMempoolV1,
    message: &TxV1,
) -> Vec<Result<Hash32, String>> {
    message
        .canonical_transactions
        .iter()
        .map(|bytes| pool.admit_canonical_bytes(bytes))
        .collect()
}

fn encode_ids(ids: &[Hash32], limit: usize, name: &str) -> Result<Vec<u8>, String> {
    if ids.len() > limit {
        return Err(format!("{name} exceeds item limit"));
    }
    let mut out = Vec::with_capacity(2 + ids.len().saturating_mul(32));
    out.extend_from_slice(&(ids.len() as u16).to_be_bytes());
    for id in ids {
        out.extend_from_slice(id);
    }
    Ok(out)
}

fn decode_ids(bytes: &[u8], limit: usize, name: &str) -> Result<Vec<Hash32>, String> {
    let mut cursor = Cursor::new(bytes);
    let count = cursor.u16()? as usize;
    if count > limit {
        return Err(format!("{name} exceeds item limit"));
    }
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(cursor.hash32()?);
    }
    if !cursor.finished() {
        return Err(format!("{name} contains trailing bytes"));
    }
    Ok(ids)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "transaction relay payload length overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("truncated transaction relay payload".into());
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn hash32(&mut self) -> Result<Hash32, String> {
        Ok(self.bytes(32)?.try_into().unwrap())
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::AddressNetwork;
    use crate::native_transaction::{
        NativeActionV1, NativeTransactionBodyV1, SignedNativeTransactionV1, DEVNET_CHAIN_ID,
        DEVNET_NETWORK_ID, NATIVE_TRANSFER_GAS_V1,
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
    fn inventory_and_request_round_trip() {
        let ids = vec![[0x11; 32], [0x22; 32]];
        let inv = TxInvV1 { tx_ids: ids.clone() };
        assert_eq!(TxInvV1::decode(&inv.encode().unwrap()).unwrap(), inv);

        let request = GetTxV1 { tx_ids: ids };
        assert_eq!(GetTxV1::decode(&request.encode().unwrap()).unwrap(), request);
    }

    #[test]
    fn transaction_batch_round_trip_preserves_exact_bytes() {
        let tx = signed_transfer(1).canonical_bytes().unwrap();
        let message = TxV1 {
            canonical_transactions: vec![tx.clone()],
        };
        let decoded = TxV1::decode(&message.encode().unwrap()).unwrap();
        assert_eq!(decoded.canonical_transactions, vec![tx]);
    }

    #[test]
    fn missing_inventory_requests_only_unknown_ids() {
        let known = signed_transfer(1);
        let unknown = signed_transfer(2);
        let known_id = known.tx_id().unwrap();
        let unknown_id = unknown.tx_id().unwrap();

        let mut pool = NativeMempoolV1::new(AddressNetwork::Devnet);
        pool.admit_transaction(known).unwrap();

        let request = missing_from_inventory(
            &pool,
            &TxInvV1 {
                tx_ids: vec![known_id, unknown_id],
            },
        );
        assert_eq!(request.tx_ids, vec![unknown_id]);
    }

    #[test]
    fn requested_transactions_are_returned_as_canonical_bytes() {
        let tx = signed_transfer(1);
        let tx_id = tx.tx_id().unwrap();
        let canonical = tx.canonical_bytes().unwrap();

        let mut pool = NativeMempoolV1::new(AddressNetwork::Devnet);
        pool.admit_transaction(tx).unwrap();

        let response = transactions_for_request(
            &pool,
            &GetTxV1 {
                tx_ids: vec![tx_id],
            },
        )
        .unwrap();
        assert_eq!(response.canonical_transactions, vec![canonical]);
    }

    #[test]
    fn relay_admission_uses_existing_mempool_canonical_validation() {
        let tx = signed_transfer(1);
        let canonical = tx.canonical_bytes().unwrap();
        let tx_id = tx.tx_id().unwrap();

        let mut pool = NativeMempoolV1::new(AddressNetwork::Devnet);
        let results = admit_relay_transactions(
            &mut pool,
            &TxV1 {
                canonical_transactions: vec![canonical],
            },
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap(), &tx_id);
        assert!(pool.contains(&tx_id));
    }

    #[test]
    fn relay_rejects_trailing_and_oversized_encodings() {
        let inv = TxInvV1 {
            tx_ids: vec![[0x11; 32]],
        };
        let mut bytes = inv.encode().unwrap();
        bytes.push(0);
        assert!(TxInvV1::decode(&bytes).is_err());

        let message = TxV1 {
            canonical_transactions: vec![vec![0u8; MAX_RELAY_TRANSACTION_BYTES_V1 + 1]],
        };
        assert!(message.encode().is_err());
    }
}
