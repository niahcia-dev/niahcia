use sha3::{Digest, Keccak256};

pub type Hash32 = [u8; 32];
pub type Address20 = [u8; 20];

const EXECUTION_DOMAIN: &[u8] = b"NIAHCIA/EXECUTION-COMMITMENT/V1";
const BLOCK_HEADER_DOMAIN: &[u8] = b"NIAHCIA/BLOCK-HEADER/V1";
const MINING_TEMPLATE_DOMAIN: &[u8] = b"NIAHCIA/MINING-TEMPLATE/V1";

pub const BLOCK_HEADER_V1_LEN: usize = 164;

const TX_DOMAIN: &[u8] = b"NIAHCIA/TX/V1";
const MERKLE_EMPTY_DOMAIN: &[u8] = b"NIAHCIA/MERKLE-EMPTY/V1";
const MERKLE_LEAF_DOMAIN: &[u8] = b"NIAHCIA/MERKLE-LEAF/V1";
const MERKLE_NODE_DOMAIN: &[u8] = b"NIAHCIA/MERKLE-NODE/V1";

pub fn transaction_merkle_root<T: AsRef<[u8]>>(transactions: &[T]) -> Hash32 {
    if transactions.is_empty() {
        return keccak256(MERKLE_EMPTY_DOMAIN);
    }

    let mut level = Vec::with_capacity(transactions.len());

    for transaction in transactions {
        let bytes = transaction.as_ref();

        let mut tx_preimage = Vec::with_capacity(TX_DOMAIN.len() + bytes.len());
        tx_preimage.extend_from_slice(TX_DOMAIN);
        tx_preimage.extend_from_slice(bytes);
        let tx_digest = keccak256(&tx_preimage);

        let mut leaf_preimage = Vec::with_capacity(MERKLE_LEAF_DOMAIN.len() + 32);
        leaf_preimage.extend_from_slice(MERKLE_LEAF_DOMAIN);
        leaf_preimage.extend_from_slice(&tx_digest);
        level.push(keccak256(&leaf_preimage));
    }

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));

        for pair in level.chunks(2) {
            let left = pair[0];
            let right = if pair.len() == 2 { pair[1] } else { pair[0] };

            let mut node_preimage = Vec::with_capacity(MERKLE_NODE_DOMAIN.len() + 64);
            node_preimage.extend_from_slice(MERKLE_NODE_DOMAIN);
            node_preimage.extend_from_slice(&left);
            node_preimage.extend_from_slice(&right);
            next.push(keccak256(&node_preimage));
        }

        level = next;
    }

    level[0]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPayloadCommitments {
    pub execution_parent_hash: Hash32,
    pub fee_recipient: Address20,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub transactions_root: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub base_fee_per_gas: Hash32,
}

impl ExecutionPayloadCommitments {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(EXECUTION_DOMAIN.len() + (32 * 5) + 20 + (8 * 4));
        out.extend_from_slice(EXECUTION_DOMAIN);
        out.extend_from_slice(&self.execution_parent_hash);
        out.extend_from_slice(&self.fee_recipient);
        out.extend_from_slice(&self.state_root);
        out.extend_from_slice(&self.receipts_root);
        out.extend_from_slice(&self.transactions_root);
        out.extend_from_slice(&self.block_number.to_be_bytes());
        out.extend_from_slice(&self.gas_limit.to_be_bytes());
        out.extend_from_slice(&self.gas_used.to_be_bytes());
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&self.base_fee_per_gas);
        out
    }

    pub fn commitment_hash(&self) -> Hash32 {
        keccak256(&self.canonical_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeaderV1 {
    pub version: u32,
    pub parent_hash: Hash32,
    pub height: u64,
    pub timestamp: u64,
    pub transactions_root: Hash32,
    pub execution_root: Hash32,
    pub target: Hash32,
    pub nonce: u64,
    pub extra_nonce: u64,
}

impl BlockHeaderV1 {
    pub fn canonical_bytes(&self) -> [u8; BLOCK_HEADER_V1_LEN] {
        let mut out = [0_u8; BLOCK_HEADER_V1_LEN];
        let mut offset = 0;

        write(&mut out, &mut offset, &self.version.to_be_bytes());
        write(&mut out, &mut offset, &self.parent_hash);
        write(&mut out, &mut offset, &self.height.to_be_bytes());
        write(&mut out, &mut offset, &self.timestamp.to_be_bytes());
        write(&mut out, &mut offset, &self.transactions_root);
        write(&mut out, &mut offset, &self.execution_root);
        write(&mut out, &mut offset, &self.target);
        write(&mut out, &mut offset, &self.nonce.to_be_bytes());
        write(&mut out, &mut offset, &self.extra_nonce.to_be_bytes());

        debug_assert_eq!(offset, BLOCK_HEADER_V1_LEN);
        out
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != BLOCK_HEADER_V1_LEN {
            return Err(format!(
                "BlockHeaderV1 must be {BLOCK_HEADER_V1_LEN} bytes; found {}",
                bytes.len()
            ));
        }

        Ok(Self {
            version: u32::from_be_bytes(bytes[0..4].try_into().unwrap()),
            parent_hash: bytes[4..36].try_into().unwrap(),
            height: u64::from_be_bytes(bytes[36..44].try_into().unwrap()),
            timestamp: u64::from_be_bytes(bytes[44..52].try_into().unwrap()),
            transactions_root: bytes[52..84].try_into().unwrap(),
            execution_root: bytes[84..116].try_into().unwrap(),
            target: bytes[116..148].try_into().unwrap(),
            nonce: u64::from_be_bytes(bytes[148..156].try_into().unwrap()),
            extra_nonce: u64::from_be_bytes(bytes[156..164].try_into().unwrap()),
        })
    }

    pub fn block_id(&self) -> Hash32 {
        let mut preimage = Vec::with_capacity(BLOCK_HEADER_DOMAIN.len() + BLOCK_HEADER_V1_LEN);
        preimage.extend_from_slice(BLOCK_HEADER_DOMAIN);
        preimage.extend_from_slice(&self.canonical_bytes());
        keccak256(&preimage)
    }

    pub fn mining_template_id(&self) -> Hash32 {
        let mut template = self.clone();
        template.nonce = 0;
        template.extra_nonce = 0;

        let mut preimage = Vec::with_capacity(MINING_TEMPLATE_DOMAIN.len() + BLOCK_HEADER_V1_LEN);
        preimage.extend_from_slice(MINING_TEMPLATE_DOMAIN);
        preimage.extend_from_slice(&template.canonical_bytes());
        keccak256(&preimage)
    }

    pub fn with_miner_values(&self, nonce: u64, extra_nonce: u64) -> Self {
        let mut header = self.clone();
        header.nonce = nonce;
        header.extra_nonce = extra_nonce;
        header
    }
}

fn write<const N: usize>(out: &mut [u8; N], offset: &mut usize, value: &[u8]) {
    let end = *offset + value.len();
    out[*offset..end].copy_from_slice(value);
    *offset = end;
}

pub fn keccak256(bytes: &[u8]) -> Hash32 {
    let digest = Keccak256::digest(bytes);
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::{
        transaction_merkle_root, BlockHeaderV1, ExecutionPayloadCommitments, BLOCK_HEADER_V1_LEN,
    };

    #[test]
    fn transaction_merkle_empty_root_is_exact() {
        let transactions: Vec<Vec<u8>> = Vec::new();

        assert_eq!(
            transaction_merkle_root(&transactions),
            super::keccak256(b"NIAHCIA/MERKLE-EMPTY/V1")
        );
    }

    #[test]
    fn transaction_merkle_single_leaf_has_no_synthetic_sibling() {
        let tx = b"native transaction".to_vec();

        let mut tx_preimage = b"NIAHCIA/TX/V1".to_vec();
        tx_preimage.extend_from_slice(&tx);
        let tx_digest = super::keccak256(&tx_preimage);

        let mut leaf_preimage = b"NIAHCIA/MERKLE-LEAF/V1".to_vec();
        leaf_preimage.extend_from_slice(&tx_digest);
        let expected = super::keccak256(&leaf_preimage);

        assert_eq!(transaction_merkle_root(&[tx]), expected);
    }

    #[test]
    fn transaction_merkle_order_is_consensus_significant() {
        let first = vec![b"tx-a".to_vec(), b"tx-b".to_vec()];
        let second = vec![b"tx-b".to_vec(), b"tx-a".to_vec()];

        assert_ne!(
            transaction_merkle_root(&first),
            transaction_merkle_root(&second)
        );
    }

    #[test]
    fn transaction_merkle_odd_leaf_is_duplicated() {
        fn leaf(tx: &[u8]) -> [u8; 32] {
            let mut tx_preimage = b"NIAHCIA/TX/V1".to_vec();
            tx_preimage.extend_from_slice(tx);
            let tx_digest = super::keccak256(&tx_preimage);

            let mut leaf_preimage = b"NIAHCIA/MERKLE-LEAF/V1".to_vec();
            leaf_preimage.extend_from_slice(&tx_digest);
            super::keccak256(&leaf_preimage)
        }

        fn node(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
            let mut preimage = b"NIAHCIA/MERKLE-NODE/V1".to_vec();
            preimage.extend_from_slice(&left);
            preimage.extend_from_slice(&right);
            super::keccak256(&preimage)
        }

        let transactions = vec![b"tx-a".to_vec(), b"tx-b".to_vec(), b"tx-c".to_vec()];

        let left = node(leaf(b"tx-a"), leaf(b"tx-b"));
        let right = node(leaf(b"tx-c"), leaf(b"tx-c"));
        let expected = node(left, right);

        assert_eq!(transaction_merkle_root(&transactions), expected);
    }

    fn sample_execution() -> ExecutionPayloadCommitments {
        ExecutionPayloadCommitments {
            execution_parent_hash: [0x11; 32],
            fee_recipient: [0x22; 20],
            state_root: [0x33; 32],
            receipts_root: [0x44; 32],
            transactions_root: [0x55; 32],
            block_number: 42,
            gas_limit: 30_000_000,
            gas_used: 12_345,
            timestamp: 1_800_000_000,
            base_fee_per_gas: [0x66; 32],
        }
    }

    fn sample_header() -> BlockHeaderV1 {
        BlockHeaderV1 {
            version: 1,
            parent_hash: [0x11; 32],
            height: 42,
            timestamp: 1_800_000_000,
            transactions_root: [0x22; 32],
            execution_root: sample_execution().commitment_hash(),
            target: [0xff; 32],
            nonce: 0x0102_0304_0506_0708,
            extra_nonce: 0x1112_1314_1516_1718,
        }
    }

    #[test]
    fn block_header_v1_round_trips_canonical_bytes() {
        let header = sample_header();
        let encoded = header.canonical_bytes();
        let decoded = BlockHeaderV1::from_canonical_bytes(&encoded).unwrap();

        assert_eq!(decoded, header);
        assert!(BlockHeaderV1::from_canonical_bytes(&encoded[..163]).is_err());
    }

    #[test]
    fn block_header_v1_is_exactly_164_bytes() {
        assert_eq!(sample_header().canonical_bytes().len(), BLOCK_HEADER_V1_LEN);
        assert_eq!(BLOCK_HEADER_V1_LEN, 164);
    }

    #[test]
    fn block_header_protocol_vector_one() {
        let header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0x11; 32],
            height: 42,
            timestamp: 1_800_000_000,
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 0x0102_0304_0506_0708,
            extra_nonce: 0x1112_1314_1516_1718,
        };

        let canonical = hex::encode(header.canonical_bytes());
        let expected = concat!(
            "00000001",
            "1111111111111111111111111111111111111111111111111111111111111111",
            "000000000000002a",
            "000000006b49d200",
            "2222222222222222222222222222222222222222222222222222222222222222",
            "3333333333333333333333333333333333333333333333333333333333333333",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "0102030405060708",
            "1112131415161718"
        );

        assert_eq!(canonical, expected);
        println!(
            "BLOCK_HEADER_V1_VECTOR_BLOCK_ID={}",
            hex::encode(header.block_id())
        );
        println!(
            "BLOCK_HEADER_V1_VECTOR_TEMPLATE_ID={}",
            hex::encode(header.mining_template_id())
        );
    }

    #[test]
    fn block_header_integer_fields_are_big_endian() {
        let bytes = sample_header().canonical_bytes();
        assert_eq!(&bytes[0..4], &1_u32.to_be_bytes());
        assert_eq!(&bytes[36..44], &42_u64.to_be_bytes());
        assert_eq!(&bytes[44..52], &1_800_000_000_u64.to_be_bytes());
        assert_eq!(&bytes[148..156], &0x0102_0304_0506_0708_u64.to_be_bytes());
        assert_eq!(&bytes[156..164], &0x1112_1314_1516_1718_u64.to_be_bytes());
    }

    #[test]
    fn mining_template_identity_ignores_only_miner_values() {
        let header = sample_header();
        let changed = header.with_miner_values(7, 9);

        assert_eq!(header.mining_template_id(), changed.mining_template_id());
        assert_ne!(header.block_id(), changed.block_id());
    }

    #[test]
    fn execution_change_changes_block_identity() {
        let mut first = sample_header();
        let mut second = sample_header();

        second.execution_root[0] ^= 0xff;

        assert_ne!(first.block_id(), second.block_id());

        first.execution_root = second.execution_root;
        assert_eq!(first.block_id(), second.block_id());
    }
}
