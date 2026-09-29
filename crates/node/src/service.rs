use crate::work::{keccak256, Hash32};

pub const STORAGE_CHALLENGE_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-CHALLENGE/V1";
pub const STORAGE_SELECT_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-SELECT/V1";
pub const STORAGE_MANIFEST_LEAF_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-MANIFEST-LEAF/V1";
pub const STORAGE_MANIFEST_NODE_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-MANIFEST-NODE/V1";
pub const STORAGE_MANIFEST_EMPTY_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-MANIFEST-EMPTY/V1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeRange {
    pub chunk_index: u64,
    pub offset: u64,
    pub length: u64,
}

pub fn storage_challenge_seed(challenge_block_id: Hash32, commitment_id: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(
        STORAGE_CHALLENGE_DOMAIN.len() + challenge_block_id.len() + commitment_id.len(),
    );
    preimage.extend_from_slice(STORAGE_CHALLENGE_DOMAIN);
    preimage.extend_from_slice(&challenge_block_id);
    preimage.extend_from_slice(&commitment_id);
    keccak256(&preimage)
}

/// Deterministically select challenge ranges from a challenge seed.
///
/// chunk_lengths is the exact ordered chunk-length list committed by the
/// storage manifest. requested_ranges controls how many independent samples
/// are requested. Each selected range is at most max_range_length bytes.
///
/// This is service-layer measurement logic; it does not affect PoW consensus.
pub fn select_storage_ranges(
    challenge_seed: Hash32,
    chunk_lengths: &[u64],
    requested_ranges: usize,
    max_range_length: u64,
) -> Result<Vec<ChallengeRange>, String> {
    if chunk_lengths.is_empty() {
        return Err("cannot challenge an empty manifest".into());
    }
    if requested_ranges == 0 {
        return Err("requested_ranges must be non-zero".into());
    }
    if max_range_length == 0 {
        return Err("max_range_length must be non-zero".into());
    }
    if chunk_lengths.contains(&0) {
        return Err("chunk lengths must be non-zero".into());
    }

    let chunk_count = u64::try_from(chunk_lengths.len())
        .map_err(|_| "chunk count does not fit u64".to_string())?;
    let mut out = Vec::with_capacity(requested_ranges);

    for counter in 0..requested_ranges {
        let digest = storage_selection_digest(challenge_seed, counter as u64);

        let chunk_word = u64::from_be_bytes(
            digest[0..8]
                .try_into()
                .map_err(|_| "invalid selection digest".to_string())?,
        );
        let offset_word = u64::from_be_bytes(
            digest[8..16]
                .try_into()
                .map_err(|_| "invalid selection digest".to_string())?,
        );

        let chunk_index = chunk_word % chunk_count;
        let chunk_length = chunk_lengths[chunk_index as usize];
        let length = chunk_length.min(max_range_length);
        let max_offset = chunk_length - length;
        let offset = if max_offset == 0 {
            0
        } else {
            offset_word % (max_offset + 1)
        };

        out.push(ChallengeRange {
            chunk_index,
            offset,
            length,
        });
    }

    Ok(out)
}

fn storage_selection_digest(challenge_seed: Hash32, counter: u64) -> Hash32 {
    let mut preimage = Vec::with_capacity(STORAGE_SELECT_DOMAIN.len() + challenge_seed.len() + 8);
    preimage.extend_from_slice(STORAGE_SELECT_DOMAIN);
    preimage.extend_from_slice(&challenge_seed);
    preimage.extend_from_slice(&counter.to_be_bytes());
    keccak256(&preimage)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestProofStep {
    pub sibling: Hash32,
    pub sibling_is_left: bool,
}

pub fn storage_manifest_leaf(chunk_index: u64, chunk_length: u64, chunk_hash: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(STORAGE_MANIFEST_LEAF_DOMAIN.len() + 8 + 8 + 32);
    preimage.extend_from_slice(STORAGE_MANIFEST_LEAF_DOMAIN);
    preimage.extend_from_slice(&chunk_index.to_be_bytes());
    preimage.extend_from_slice(&chunk_length.to_be_bytes());
    preimage.extend_from_slice(&chunk_hash);
    keccak256(&preimage)
}

pub fn storage_manifest_node(left: Hash32, right: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(STORAGE_MANIFEST_NODE_DOMAIN.len() + 64);
    preimage.extend_from_slice(STORAGE_MANIFEST_NODE_DOMAIN);
    preimage.extend_from_slice(&left);
    preimage.extend_from_slice(&right);
    keccak256(&preimage)
}

pub fn storage_manifest_root(chunks: &[(u64, Hash32)]) -> Hash32 {
    if chunks.is_empty() {
        return keccak256(STORAGE_MANIFEST_EMPTY_DOMAIN);
    }

    let mut level: Vec<Hash32> = chunks
        .iter()
        .enumerate()
        .map(|(index, (length, hash))| storage_manifest_leaf(index as u64, *length, *hash))
        .collect();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let left = pair[0];
            let right = if pair.len() == 2 { pair[1] } else { left };
            next.push(storage_manifest_node(left, right));
        }
        level = next;
    }

    level[0]
}

pub fn verify_storage_manifest_proof(
    expected_root: Hash32,
    chunk_index: u64,
    chunk_length: u64,
    chunk_hash: Hash32,
    proof: &[ManifestProofStep],
) -> bool {
    let mut current = storage_manifest_leaf(chunk_index, chunk_length, chunk_hash);

    for step in proof {
        current = if step.sibling_is_left {
            storage_manifest_node(step.sibling, current)
        } else {
            storage_manifest_node(current, step.sibling)
        };
    }

    current == expected_root
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseMeta {
    pub challenge_id: Hash32,
    pub commitment_id: Hash32,
    pub answered_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedResponseMeta {
    pub challenge_id: Hash32,
    pub commitment_id: Hash32,
    pub response_deadline: u64,
}

pub fn verify_response_meta(
    expected: &ExpectedResponseMeta,
    response: &ResponseMeta,
) -> Result<(), String> {
    if response.challenge_id != expected.challenge_id {
        return Err("storage response challenge_id mismatch".into());
    }

    if response.commitment_id != expected.commitment_id {
        return Err("storage response commitment_id mismatch".into());
    }

    if response.answered_at > expected.response_deadline {
        return Err(format!(
            "storage response missed deadline: answered_at={} deadline={}",
            response.answered_at, expected.response_deadline
        ));
    }

    Ok(())
}

pub fn evidence_replay_key(
    challenge_id: Hash32,
    service_node_id: Hash32,
    response_hash: Hash32,
) -> Hash32 {
    const DOMAIN: &[u8] = b"NIAHCIA/SERVICE-EVIDENCE/V1";
    let mut preimage = Vec::with_capacity(DOMAIN.len() + 96);
    preimage.extend_from_slice(DOMAIN);
    preimage.extend_from_slice(&challenge_id);
    preimage.extend_from_slice(&service_node_id);
    preimage.extend_from_slice(&response_hash);
    keccak256(&preimage)
}

#[cfg(test)]
mod tests {
    use super::{
        evidence_replay_key, select_storage_ranges, storage_challenge_seed, storage_manifest_leaf,
        storage_manifest_node, storage_manifest_root, verify_response_meta,
        verify_storage_manifest_proof, ExpectedResponseMeta, ManifestProofStep, ResponseMeta,
    };

    #[test]
    fn challenge_seed_is_deterministic_and_domain_separated() {
        let first = storage_challenge_seed([0x11; 32], [0x22; 32]);
        let second = storage_challenge_seed([0x11; 32], [0x22; 32]);
        let changed_block = storage_challenge_seed([0x12; 32], [0x22; 32]);
        let changed_commitment = storage_challenge_seed([0x11; 32], [0x23; 32]);

        assert_eq!(first, second);
        assert_ne!(first, changed_block);
        assert_ne!(first, changed_commitment);
    }

    #[test]
    fn selection_is_deterministic_and_in_bounds() {
        let seed = storage_challenge_seed([0x11; 32], [0x22; 32]);
        let chunk_lengths = [4096, 4096, 1024, 8192];

        let first = select_storage_ranges(seed, &chunk_lengths, 8, 512).unwrap();
        let second = select_storage_ranges(seed, &chunk_lengths, 8, 512).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 8);

        for range in first {
            let chunk_len = chunk_lengths[range.chunk_index as usize];
            assert!(range.length > 0);
            assert!(range.length <= 512);
            assert!(range.offset + range.length <= chunk_len);
        }
    }

    #[test]
    fn short_chunks_are_challenged_in_full() {
        let seed = storage_challenge_seed([0x44; 32], [0x55; 32]);
        let ranges = select_storage_ranges(seed, &[64], 3, 512).unwrap();

        for range in ranges {
            assert_eq!(range.chunk_index, 0);
            assert_eq!(range.offset, 0);
            assert_eq!(range.length, 64);
        }
    }

    #[test]
    fn manifest_merkle_root_is_deterministic() {
        let chunks = [
            (100_u64, [0x11; 32]),
            (200_u64, [0x22; 32]),
            (300_u64, [0x33; 32]),
        ];

        let first = storage_manifest_root(&chunks);
        let second = storage_manifest_root(&chunks);

        assert_eq!(first, second);
        assert_ne!(first, storage_manifest_root(&chunks[..2]));
    }

    #[test]
    fn manifest_proof_verifies_and_rejects_tampering() {
        let leaf0 = storage_manifest_leaf(0, 100, [0x11; 32]);
        let leaf1 = storage_manifest_leaf(1, 200, [0x22; 32]);
        let leaf2 = storage_manifest_leaf(2, 300, [0x33; 32]);

        let parent01 = storage_manifest_node(leaf0, leaf1);
        let parent22 = storage_manifest_node(leaf2, leaf2);
        let root = storage_manifest_node(parent01, parent22);

        let proof = [
            ManifestProofStep {
                sibling: leaf0,
                sibling_is_left: true,
            },
            ManifestProofStep {
                sibling: parent22,
                sibling_is_left: false,
            },
        ];

        assert!(verify_storage_manifest_proof(
            root, 1, 200, [0x22; 32], &proof
        ));
        assert!(!verify_storage_manifest_proof(
            root, 1, 201, [0x22; 32], &proof
        ));
        assert!(!verify_storage_manifest_proof(
            root, 1, 200, [0x23; 32], &proof
        ));
    }

    #[test]
    fn empty_manifest_has_domain_separated_root() {
        let empty = storage_manifest_root(&[]);
        assert_ne!(empty, [0_u8; 32]);
        assert_eq!(empty, storage_manifest_root(&[]));
    }

    #[test]
    fn response_metadata_enforces_ids_and_deadline() {
        let expected = ExpectedResponseMeta {
            challenge_id: [0x10; 32],
            commitment_id: [0x20; 32],
            response_deadline: 1000,
        };

        let valid = ResponseMeta {
            challenge_id: [0x10; 32],
            commitment_id: [0x20; 32],
            answered_at: 1000,
        };
        assert!(verify_response_meta(&expected, &valid).is_ok());

        let mut wrong_challenge = valid.clone();
        wrong_challenge.challenge_id = [0x11; 32];
        assert!(verify_response_meta(&expected, &wrong_challenge).is_err());

        let mut wrong_commitment = valid.clone();
        wrong_commitment.commitment_id = [0x21; 32];
        assert!(verify_response_meta(&expected, &wrong_commitment).is_err());

        let mut late = valid;
        late.answered_at = 1001;
        assert!(verify_response_meta(&expected, &late).is_err());
    }

    #[test]
    fn replay_key_binds_challenge_provider_and_response() {
        let first = evidence_replay_key([0x01; 32], [0x02; 32], [0x03; 32]);
        let same = evidence_replay_key([0x01; 32], [0x02; 32], [0x03; 32]);
        let other_challenge = evidence_replay_key([0x04; 32], [0x02; 32], [0x03; 32]);
        let other_provider = evidence_replay_key([0x01; 32], [0x05; 32], [0x03; 32]);
        let other_response = evidence_replay_key([0x01; 32], [0x02; 32], [0x06; 32]);

        assert_eq!(first, same);
        assert_ne!(first, other_challenge);
        assert_ne!(first, other_provider);
        assert_ne!(first, other_response);
    }

    #[test]
    fn invalid_selector_inputs_are_rejected() {
        let seed = [0x77; 32];

        assert!(select_storage_ranges(seed, &[], 1, 512).is_err());
        assert!(select_storage_ranges(seed, &[4096], 0, 512).is_err());
        assert!(select_storage_ranges(seed, &[4096], 1, 0).is_err());
        assert!(select_storage_ranges(seed, &[0, 4096], 1, 512).is_err());
    }
}
