use sha3::{Digest, Keccak256};

pub type Hash32 = [u8; 32];
pub type Address20 = [u8; 20];

const EXECUTION_DOMAIN: &[u8] = b"NIAHCIA/EXECUTION-COMMITMENT/V1";
const WORK_DOMAIN: &[u8] = b"NIAHCIA/POW-WORK/V1";
const HEADER_DOMAIN: &[u8] = b"NIAHCIA/POW-HEADER/V1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPayloadCommitments {
    pub parent_hash: Hash32,
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
        let mut out = Vec::with_capacity(
            EXECUTION_DOMAIN.len() + 32 + 20 + (32 * 5) + (8 * 4),
        );
        out.extend_from_slice(EXECUTION_DOMAIN);
        out.extend_from_slice(&self.parent_hash);
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
pub struct PowWorkTemplate {
    pub version: u32,
    pub height: u64,
    pub parent_hash: Hash32,
    pub execution_commitment: Hash32,
    pub timestamp: u64,
    pub difficulty: u64,
    pub target: Hash32,
}

impl PowWorkTemplate {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(WORK_DOMAIN.len() + 4 + 8 + 32 + 32 + 8 + 8 + 32);
        out.extend_from_slice(WORK_DOMAIN);
        out.extend_from_slice(&self.version.to_be_bytes());
        out.extend_from_slice(&self.height.to_be_bytes());
        out.extend_from_slice(&self.parent_hash);
        out.extend_from_slice(&self.execution_commitment);
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&self.difficulty.to_be_bytes());
        out.extend_from_slice(&self.target);
        out
    }

    pub fn template_id(&self) -> Hash32 {
        keccak256(&self.canonical_bytes())
    }

    pub fn header_bytes(&self, nonce: u64) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_DOMAIN.len() + self.canonical_bytes().len() + 8);
        out.extend_from_slice(HEADER_DOMAIN);
        out.extend_from_slice(&self.canonical_bytes());
        out.extend_from_slice(&nonce.to_be_bytes());
        out
    }
}

pub fn keccak256(bytes: &[u8]) -> Hash32 {
    let digest = Keccak256::digest(bytes);
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::{ExecutionPayloadCommitments, PowWorkTemplate};

    fn sample_execution() -> ExecutionPayloadCommitments {
        ExecutionPayloadCommitments {
            parent_hash: [0x11; 32],
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

    fn sample_work() -> PowWorkTemplate {
        PowWorkTemplate {
            version: 1,
            height: 42,
            parent_hash: [0x11; 32],
            execution_commitment: sample_execution().commitment_hash(),
            timestamp: 1_800_000_000,
            difficulty: 1,
            target: [0xff; 32],
        }
    }

    #[test]
    fn execution_commitment_is_deterministic() {
        let execution = sample_execution();
        assert_eq!(execution.commitment_hash(), execution.commitment_hash());
        assert_eq!(execution.canonical_bytes(), execution.canonical_bytes());
    }

    #[test]
    fn template_id_does_not_depend_on_nonce() {
        let work = sample_work();
        let template_id = work.template_id();

        let header_a = work.header_bytes(7);
        let header_b = work.header_bytes(8);

        assert_eq!(template_id, work.template_id());
        assert_ne!(header_a, header_b);
        assert_eq!(
            &header_a[..header_a.len() - 8],
            &header_b[..header_b.len() - 8]
        );
    }

    #[test]
    fn nonce_is_encoded_as_last_eight_header_bytes() {
        let work = sample_work();
        let nonce = 0x0102_0304_0506_0708_u64;
        let header = work.header_bytes(nonce);

        assert_eq!(&header[header.len() - 8..], &nonce.to_be_bytes());
    }

    #[test]
    fn changing_execution_changes_work_identity() {
        let first = sample_execution();

        let mut second = first.clone();
        second.state_root[0] ^= 0xff;

        let mut work_a = sample_work();
        let mut work_b = sample_work();

        work_a.execution_commitment = first.commitment_hash();
        work_b.execution_commitment = second.commitment_hash();

        assert_ne!(work_a.template_id(), work_b.template_id());
    }
}
