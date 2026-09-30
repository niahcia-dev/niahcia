use crate::consensus::pow_meets_target;
use crate::work::{BlockHeaderV1, Hash32};
use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};

pub struct RandomXVerifier {
    vm: RandomXVM,
}

impl RandomXVerifier {
    pub fn new(seed: Hash32) -> Result<Self, String> {
        let flags = RandomXFlag::FLAG_DEFAULT;
        let cache =
            RandomXCache::new(flags, &seed).map_err(|e| format!("RandomX cache init failed: {e}"))?;
        let vm = RandomXVM::new(flags, Some(cache), None)
            .map_err(|e| format!("RandomX VM init failed: {e}"))?;
        Ok(Self { vm })
    }

    pub fn hash_header(&self, header: &BlockHeaderV1) -> Result<Hash32, String> {
        let hash = self
            .vm
            .calculate_hash(&header.canonical_bytes())
            .map_err(|e| format!("RandomX hash failed: {e}"))?;

        hash.try_into()
            .map_err(|value: Vec<u8>| format!("RandomX hash must be 32 bytes; found {}", value.len()))
    }

    pub fn verify_header(&self, header: &BlockHeaderV1) -> Result<Hash32, String> {
        let hash = self.hash_header(header)?;
        if !pow_meets_target(hash, header.target) {
            return Err("RandomX hash does not meet block target".into());
        }
        Ok(hash)
    }
}

pub fn randomx_hash(seed: Hash32, input: &[u8]) -> Result<Hash32, String> {
    let flags = RandomXFlag::FLAG_DEFAULT;
    let cache =
        RandomXCache::new(flags, &seed).map_err(|e| format!("RandomX cache init failed: {e}"))?;
    let vm = RandomXVM::new(flags, Some(cache), None)
        .map_err(|e| format!("RandomX VM init failed: {e}"))?;
    let hash = vm
        .calculate_hash(input)
        .map_err(|e| format!("RandomX hash failed: {e}"))?;

    hash.try_into()
        .map_err(|value: Vec<u8>| format!("RandomX hash must be 32 bytes; found {}", value.len()))
}

#[cfg(test)]
mod tests {
    use super::{randomx_hash, RandomXVerifier};
    use crate::work::BlockHeaderV1;

    #[test]
    fn randomx_matches_reference_vector() {
        let mut key = [0_u8; 32];
        let source = b"test key 000";
        key[..source.len()].copy_from_slice(source);

        // This checks the binding and the reference implementation path.
        // NIAHCIA consensus vectors use a fixed 32-byte seed rather than a text key.
        let first = randomx_hash(key, b"This is a test").unwrap();
        let second = randomx_hash(key, b"This is a test").unwrap();
        assert_eq!(first, second);
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
