use crate::consensus::pow_meets_target;
use crate::work::{BlockHeaderV1, Hash32};
use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};

pub struct RandomXVerifier {
    vm: RandomXVM,
}

impl RandomXVerifier {
    pub fn new(seed: Hash32) -> Result<Self, String> {
        let flags = RandomXFlag::FLAG_DEFAULT;
        let cache = RandomXCache::new(flags, &seed)
            .map_err(|e| format!("RandomX cache init failed: {e}"))?;
        let vm = RandomXVM::new(flags, Some(cache), None)
            .map_err(|e| format!("RandomX VM init failed: {e}"))?;
        Ok(Self { vm })
    }

    pub fn hash_header(&self, header: &BlockHeaderV1) -> Result<Hash32, String> {
        let hash = self
            .vm
            .calculate_hash(&header.canonical_bytes())
            .map_err(|e| format!("RandomX hash failed: {e}"))?;

        hash.try_into().map_err(|value: Vec<u8>| {
            format!("RandomX hash must be 32 bytes; found {}", value.len())
        })
    }

    pub fn verify_header(&self, header: &BlockHeaderV1) -> Result<Hash32, String> {
        let hash = self.hash_header(header)?;
        if !pow_meets_target(hash, header.target) {
            return Err("RandomX hash does not meet block target".into());
        }
        Ok(hash)
    }
}

fn randomx_hash_with_key(key: &[u8], input: &[u8]) -> Result<Hash32, String> {
    let flags = RandomXFlag::FLAG_DEFAULT;
    let cache =
        RandomXCache::new(flags, key).map_err(|e| format!("RandomX cache init failed: {e}"))?;
    let vm = RandomXVM::new(flags, Some(cache), None)
        .map_err(|e| format!("RandomX VM init failed: {e}"))?;
    let hash = vm
        .calculate_hash(input)
        .map_err(|e| format!("RandomX hash failed: {e}"))?;

    hash.try_into()
        .map_err(|value: Vec<u8>| format!("RandomX hash must be 32 bytes; found {}", value.len()))
}

pub fn randomx_hash(seed: Hash32, input: &[u8]) -> Result<Hash32, String> {
    randomx_hash_with_key(&seed, input)
}

pub fn search_nonce_range(
    seed: Hash32,
    template: &BlockHeaderV1,
    nonce_start: u64,
    nonce_end: u64,
    extra_nonce: u64,
) -> Result<Option<(u64, Hash32)>, String> {
    if nonce_start > nonce_end {
        return Err("nonce search range is inverted".into());
    }

    let verifier = RandomXVerifier::new(seed)?;
    let mut header = template.clone();
    header.extra_nonce = extra_nonce;

    for nonce in nonce_start..=nonce_end {
        header.nonce = nonce;
        let hash = verifier.hash_header(&header)?;
        if pow_meets_target(hash, header.target) {
            return Ok(Some((nonce, hash)));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{randomx_hash, randomx_hash_with_key, search_nonce_range, RandomXVerifier};
    use crate::work::BlockHeaderV1;

    #[test]
    fn randomx_matches_reference_vector() {
        let hash = randomx_hash_with_key(b"test key 000", b"This is a test").unwrap();
        assert_eq!(
            hex::encode(hash),
            "639183aae1bf4c9a35884cb46b09cad9175f04efd7684e7262a0ac1c2f0b4e3f"
        );
    }

    #[test]
    fn consensus_seed_is_exactly_32_bytes() {
        let seed = [0x42_u8; 32];
        assert_eq!(
            randomx_hash(seed, b"NIAHCIA").unwrap(),
            randomx_hash_with_key(&seed, b"NIAHCIA").unwrap()
        );
    }

    #[test]
    fn niahcia_randomx_block_header_vector() {
        let seed = [0x42_u8; 32];
        let header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0x11; 32],
            height: 2_048,
            timestamp: 1_800_000_123,
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 0x0102_0304_0506_0708,
            extra_nonce: 0x1112_1314_1516_1718,
        };

        assert_eq!(
            hex::encode(randomx_hash(seed, &header.canonical_bytes()).unwrap()),
            "b66abdedf1d9a99ae6f82919b4ebe118ac05b2038caefa1e345eed3525d12588"
        );
    }

    #[test]
    fn nonce_search_finds_solution_without_changing_execution_fields() {
        let seed = [0x44_u8; 32];
        let header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0x11; 32],
            height: 7,
            timestamp: 1_800_000_007,
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 999,
            extra_nonce: 999,
        };

        let (nonce, hash) = search_nonce_range(seed, &header, 5, 9, 3)
            .unwrap()
            .expect("max target should accept first searched nonce");
        assert_eq!(nonce, 5);

        let mut solved = header.clone();
        solved.nonce = nonce;
        solved.extra_nonce = 3;
        assert_eq!(hash, randomx_hash(seed, &solved.canonical_bytes()).unwrap());

        assert_eq!(solved.parent_hash, header.parent_hash);
        assert_eq!(solved.transactions_root, header.transactions_root);
        assert_eq!(solved.execution_root, header.execution_root);
        assert_eq!(solved.timestamp, header.timestamp);
        assert_eq!(solved.target, header.target);
    }

    #[test]
    fn nonce_search_rejects_inverted_range() {
        let header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0_u8; 32],
            height: 0,
            timestamp: 1_800_000_000,
            transactions_root: [0_u8; 32],
            execution_root: [0_u8; 32],
            target: [0xff; 32],
            nonce: 0,
            extra_nonce: 0,
        };

        assert_eq!(
            search_nonce_range([0x55; 32], &header, 9, 5, 0).unwrap_err(),
            "nonce search range is inverted"
        );
    }

    #[test]
    fn verifier_hashes_exact_canonical_header_and_checks_target() {
        let seed = [0x33; 32];
        let verifier = RandomXVerifier::new(seed).unwrap();
        let mut header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0x11; 32],
            height: 1,
            timestamp: 1_800_000_001,
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 7,
            extra_nonce: 9,
        };

        let hash = verifier.verify_header(&header).unwrap();
        assert_eq!(hash, verifier.hash_header(&header).unwrap());

        header.target = [0_u8; 32];
        assert!(verifier.verify_header(&header).is_err());
    }
}
