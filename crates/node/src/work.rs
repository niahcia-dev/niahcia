use sha3::{Digest, Keccak256};

pub type Hash32 = [u8; 32];
pub type Address20 = [u8; 20];

const EXECUTION_DOMAIN: &[u8] = b"NIAHCIA/EXECUTION-COMMITMENT/V1";
const BLOCK_HEADER_DOMAIN: &[u8] = b"NIAHCIA/BLOCK-HEADER/V1";
const MINING_TEMPLATE_DOMAIN: &[u8] = b"NIAHCIA/MINING-TEMPLATE/V1";

pub const BLOCK_HEADER_V1_LEN: usize = 164;

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
    use super::{BlockHeaderV1, ExecutionPayloadCommitments, BLOCK_HEADER_V1_LEN};

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
        println!("BLOCK_HEADER_V1_VECTOR_BLOCK_ID={}", hex::encode(header.block_id()));
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
