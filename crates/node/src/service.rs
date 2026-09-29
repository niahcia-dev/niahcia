use crate::work::{keccak256, Hash32};
use std::collections::HashSet;

pub const STORAGE_CHALLENGE_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-CHALLENGE/V1";
pub const STORAGE_SELECT_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-SELECT/V1";
pub const STORAGE_MANIFEST_LEAF_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-MANIFEST-LEAF/V1";
pub const STORAGE_MANIFEST_NODE_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-MANIFEST-NODE/V1";
pub const STORAGE_MANIFEST_EMPTY_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-MANIFEST-EMPTY/V1";
pub const STORAGE_RANGE_LEAF_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-RANGE-LEAF/V1";
pub const STORAGE_RANGE_NODE_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-RANGE-NODE/V1";
pub const STORAGE_RANGE_EMPTY_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-RANGE-EMPTY/V1";
pub const SERVICE_EVIDENCE_LEAF_DOMAIN: &[u8] = b"NIAHCIA/SERVICE-EVIDENCE-LEAF/V1";
pub const SERVICE_EVIDENCE_NODE_DOMAIN: &[u8] = b"NIAHCIA/SERVICE-EVIDENCE-NODE/V1";
pub const SERVICE_EVIDENCE_EMPTY_DOMAIN: &[u8] = b"NIAHCIA/SERVICE-EVIDENCE-EMPTY/V1";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeSegment {
    pub chunk_index: u64,
    pub segment_index: u64,
    pub offset: u64,
    pub length: u64,
}

pub fn select_storage_segments(
    challenge_seed: Hash32,
    chunk_lengths: &[u64],
    requested_segments: usize,
    segment_size: u64,
) -> Result<Vec<ChallengeSegment>, String> {
    if chunk_lengths.is_empty() {
        return Err("cannot challenge an empty manifest".into());
    }
    if requested_segments == 0 {
        return Err("requested_segments must be non-zero".into());
    }
    if segment_size == 0 {
        return Err("segment_size must be non-zero".into());
    }
    if chunk_lengths.contains(&0) {
        return Err("chunk lengths must be non-zero".into());
    }

    let chunk_count = u64::try_from(chunk_lengths.len())
        .map_err(|_| "chunk count does not fit u64".to_string())?;
    let mut out = Vec::with_capacity(requested_segments);

    for counter in 0..requested_segments {
        let digest = storage_selection_digest(challenge_seed, counter as u64);

        let chunk_word = u64::from_be_bytes(
            digest[0..8]
                .try_into()
                .map_err(|_| "invalid selection digest".to_string())?,
        );
        let segment_word = u64::from_be_bytes(
            digest[8..16]
                .try_into()
                .map_err(|_| "invalid selection digest".to_string())?,
        );

        let chunk_index = chunk_word % chunk_count;
        let chunk_length = chunk_lengths[chunk_index as usize];
        let segment_count = chunk_length.div_ceil(segment_size);
        let segment_index = segment_word % segment_count;
        let offset = segment_index
            .checked_mul(segment_size)
            .ok_or_else(|| "segment offset overflow".to_string())?;
        let length = (chunk_length - offset).min(segment_size);

        out.push(ChallengeSegment {
            chunk_index,
            segment_index,
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

pub fn storage_manifest_leaf(
    chunk_index: u64,
    chunk_length: u64,
    chunk_hash: Hash32,
    range_root: Hash32,
) -> Hash32 {
    let mut preimage = Vec::with_capacity(STORAGE_MANIFEST_LEAF_DOMAIN.len() + 8 + 8 + 32 + 32);
    preimage.extend_from_slice(STORAGE_MANIFEST_LEAF_DOMAIN);
    preimage.extend_from_slice(&chunk_index.to_be_bytes());
    preimage.extend_from_slice(&chunk_length.to_be_bytes());
    preimage.extend_from_slice(&chunk_hash);
    preimage.extend_from_slice(&range_root);
    keccak256(&preimage)
}

pub fn storage_manifest_node(left: Hash32, right: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(STORAGE_MANIFEST_NODE_DOMAIN.len() + 64);
    preimage.extend_from_slice(STORAGE_MANIFEST_NODE_DOMAIN);
    preimage.extend_from_slice(&left);
    preimage.extend_from_slice(&right);
    keccak256(&preimage)
}

pub fn storage_manifest_root(chunks: &[(u64, Hash32, Hash32)]) -> Hash32 {
    if chunks.is_empty() {
        return keccak256(STORAGE_MANIFEST_EMPTY_DOMAIN);
    }

    let mut level: Vec<Hash32> = chunks
        .iter()
        .enumerate()
        .map(|(index, (length, hash, range_root))| {
            storage_manifest_leaf(index as u64, *length, *hash, *range_root)
        })
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
    range_root: Hash32,
    proof: &[ManifestProofStep],
) -> bool {
    let mut current = storage_manifest_leaf(chunk_index, chunk_length, chunk_hash, range_root);

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
pub struct RangeProofStep {
    pub sibling: Hash32,
    pub sibling_is_left: bool,
}

pub fn storage_range_leaf(segment_index: u64, segment: &[u8]) -> Hash32 {
    let mut preimage = Vec::with_capacity(STORAGE_RANGE_LEAF_DOMAIN.len() + 8 + 8 + segment.len());
    preimage.extend_from_slice(STORAGE_RANGE_LEAF_DOMAIN);
    preimage.extend_from_slice(&segment_index.to_be_bytes());
    preimage.extend_from_slice(&(segment.len() as u64).to_be_bytes());
    preimage.extend_from_slice(segment);
    keccak256(&preimage)
}

pub fn storage_range_node(left: Hash32, right: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(STORAGE_RANGE_NODE_DOMAIN.len() + 64);
    preimage.extend_from_slice(STORAGE_RANGE_NODE_DOMAIN);
    preimage.extend_from_slice(&left);
    preimage.extend_from_slice(&right);
    keccak256(&preimage)
}

pub fn storage_range_root(chunk: &[u8], segment_size: usize) -> Result<Hash32, String> {
    if segment_size == 0 {
        return Err("segment_size must be non-zero".into());
    }

    if chunk.is_empty() {
        return Ok(keccak256(STORAGE_RANGE_EMPTY_DOMAIN));
    }

    let mut level: Vec<Hash32> = chunk
        .chunks(segment_size)
        .enumerate()
        .map(|(index, segment)| storage_range_leaf(index as u64, segment))
        .collect();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let left = pair[0];
            let right = if pair.len() == 2 { pair[1] } else { left };
            next.push(storage_range_node(left, right));
        }
        level = next;
    }

    Ok(level[0])
}

pub fn verify_storage_range_proof(
    expected_root: Hash32,
    segment_index: u64,
    segment: &[u8],
    proof: &[RangeProofStep],
) -> bool {
    let mut current = storage_range_leaf(segment_index, segment);

    for step in proof {
        current = if step.sibling_is_left {
            storage_range_node(step.sibling, current)
        } else {
            storage_range_node(current, step.sibling)
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

pub fn service_evidence_leaf(evidence_key: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(SERVICE_EVIDENCE_LEAF_DOMAIN.len() + 32);
    preimage.extend_from_slice(SERVICE_EVIDENCE_LEAF_DOMAIN);
    preimage.extend_from_slice(&evidence_key);
    keccak256(&preimage)
}

pub fn service_evidence_node(left: Hash32, right: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(SERVICE_EVIDENCE_NODE_DOMAIN.len() + 64);
    preimage.extend_from_slice(SERVICE_EVIDENCE_NODE_DOMAIN);
    preimage.extend_from_slice(&left);
    preimage.extend_from_slice(&right);
    keccak256(&preimage)
}

pub fn service_evidence_root(evidence_keys: &[Hash32]) -> Hash32 {
    if evidence_keys.is_empty() {
        return keccak256(SERVICE_EVIDENCE_EMPTY_DOMAIN);
    }

    let mut ordered = evidence_keys.to_vec();
    ordered.sort_unstable();

    let mut level: Vec<Hash32> = ordered.into_iter().map(service_evidence_leaf).collect();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let left = pair[0];
            let right = if pair.len() == 2 { pair[1] } else { left };
            next.push(service_evidence_node(left, right));
        }
        level = next;
    }

    level[0]
}

#[derive(Debug, Clone)]
pub struct ServiceEpochAccumulator {
    pub epoch_start_height: u64,
    pub epoch_end_height: u64,
    seen_evidence: HashSet<Hash32>,
    pub challenges_passed: u64,
    pub challenges_failed: u64,
    pub deadlines_missed: u64,
    pub verified_bytes_served: u128,
    requester_ids: HashSet<Hash32>,
    challenge_block_ids: HashSet<Hash32>,
}

impl ServiceEpochAccumulator {
    pub fn new(epoch_start_height: u64, epoch_end_height: u64) -> Result<Self, String> {
        if epoch_end_height <= epoch_start_height {
            return Err("epoch_end_height must be greater than epoch_start_height".into());
        }

        Ok(Self {
            epoch_start_height,
            epoch_end_height,
            seen_evidence: HashSet::new(),
            challenges_passed: 0,
            challenges_failed: 0,
            deadlines_missed: 0,
            verified_bytes_served: 0,
            requester_ids: HashSet::new(),
            challenge_block_ids: HashSet::new(),
        })
    }

    pub fn record_success(
        &mut self,
        evidence_key: Hash32,
        requester_id: Hash32,
        challenge_block_id: Hash32,
        verified_bytes: u64,
    ) -> Result<(), String> {
        if !self.seen_evidence.insert(evidence_key) {
            return Err("duplicate service evidence".into());
        }

        self.challenges_passed = self.challenges_passed.saturating_add(1);
        self.verified_bytes_served = self
            .verified_bytes_served
            .saturating_add(u128::from(verified_bytes));
        self.requester_ids.insert(requester_id);
        self.challenge_block_ids.insert(challenge_block_id);
        Ok(())
    }

    pub fn record_failure(
        &mut self,
        evidence_key: Hash32,
        missed_deadline: bool,
    ) -> Result<(), String> {
        if !self.seen_evidence.insert(evidence_key) {
            return Err("duplicate service evidence".into());
        }

        self.challenges_failed = self.challenges_failed.saturating_add(1);
        if missed_deadline {
            self.deadlines_missed = self.deadlines_missed.saturating_add(1);
        }
        Ok(())
    }

    pub fn distinct_requester_count(&self) -> usize {
        self.requester_ids.len()
    }

    pub fn distinct_challenge_block_count(&self) -> usize {
        self.challenge_block_ids.len()
    }

    pub fn evidence_count(&self) -> usize {
        self.seen_evidence.len()
    }

    pub fn evidence_root(&self) -> Hash32 {
        let keys: Vec<Hash32> = self.seen_evidence.iter().copied().collect();
        service_evidence_root(&keys)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEligibility {
    pub eligible: bool,
    pub weight: u128,
    pub reason: &'static str,
}

/// Conservative development-only eligibility rule.
///
/// This is service-payment accounting, not consensus. It intentionally uses
/// hard minimums and no reputation multiplier.
pub fn evaluate_service_eligibility(
    epoch: &ServiceEpochAccumulator,
    min_passed: u64,
    min_distinct_requesters: usize,
    min_distinct_challenge_blocks: usize,
) -> ServiceEligibility {
    if epoch.challenges_passed < min_passed {
        return ServiceEligibility {
            eligible: false,
            weight: 0,
            reason: "insufficient successful challenges",
        };
    }

    if epoch.distinct_requester_count() < min_distinct_requesters {
        return ServiceEligibility {
            eligible: false,
            weight: 0,
            reason: "insufficient requester diversity",
        };
    }

    if epoch.distinct_challenge_block_count() < min_distinct_challenge_blocks {
        return ServiceEligibility {
            eligible: false,
            weight: 0,
            reason: "insufficient challenge-block diversity",
        };
    }

    if epoch.challenges_failed > epoch.challenges_passed {
        return ServiceEligibility {
            eligible: false,
            weight: 0,
            reason: "more failed than successful challenges",
        };
    }

    if epoch.deadlines_missed > 0 {
        return ServiceEligibility {
            eligible: false,
            weight: 0,
            reason: "missed response deadline",
        };
    }

    ServiceEligibility {
        eligible: true,
        weight: epoch
            .verified_bytes_served
            .max(u128::from(epoch.challenges_passed)),
        reason: "eligible",
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedServiceEpochReport {
    pub service_node_id: Hash32,
    pub epoch_start_height: u64,
    pub epoch_end_height: u64,
    pub challenges_passed: u64,
    pub challenges_failed: u64,
    pub deadlines_missed: u64,
    pub verified_bytes_served: u128,
    pub distinct_requester_count: u64,
    pub distinct_challenge_block_count: u64,
    pub evidence_root: Hash32,
    pub eligible: bool,
    pub eligibility_weight: u128,
}

pub fn finalize_service_epoch_report(
    service_node_id: Hash32,
    epoch: &ServiceEpochAccumulator,
    min_passed: u64,
    min_distinct_requesters: usize,
    min_distinct_challenge_blocks: usize,
) -> FinalizedServiceEpochReport {
    let eligibility = evaluate_service_eligibility(
        epoch,
        min_passed,
        min_distinct_requesters,
        min_distinct_challenge_blocks,
    );

    FinalizedServiceEpochReport {
        service_node_id,
        epoch_start_height: epoch.epoch_start_height,
        epoch_end_height: epoch.epoch_end_height,
        challenges_passed: epoch.challenges_passed,
        challenges_failed: epoch.challenges_failed,
        deadlines_missed: epoch.deadlines_missed,
        verified_bytes_served: epoch.verified_bytes_served,
        distinct_requester_count: epoch.distinct_requester_count() as u64,
        distinct_challenge_block_count: epoch.distinct_challenge_block_count() as u64,
        evidence_root: epoch.evidence_root(),
        eligible: eligibility.eligible,
        eligibility_weight: eligibility.weight,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_service_eligibility, finalize_service_epoch_report, evidence_replay_key, select_storage_ranges,
        select_storage_segments, service_evidence_root, storage_challenge_seed,
        storage_manifest_leaf, storage_manifest_node, storage_manifest_root, storage_range_leaf,
        storage_range_node, storage_range_root, verify_response_meta,
        verify_storage_manifest_proof, verify_storage_range_proof, ChallengeSegment,
        ExpectedResponseMeta, ManifestProofStep, RangeProofStep, ResponseMeta,
        ServiceEpochAccumulator,
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
    fn segment_selection_is_aligned_and_in_bounds() {
        let seed = storage_challenge_seed([0x66; 32], [0x77; 32]);
        let chunk_lengths = [4096_u64, 5000_u64, 1024_u64];
        let segments = select_storage_segments(seed, &chunk_lengths, 16, 1024).unwrap();

        assert_eq!(segments.len(), 16);

        for ChallengeSegment {
            chunk_index,
            segment_index,
            offset,
            length,
        } in segments
        {
            let chunk_len = chunk_lengths[chunk_index as usize];
            assert_eq!(offset, segment_index * 1024);
            assert!(length > 0);
            assert!(length <= 1024);
            assert!(offset + length <= chunk_len);
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
            (100_u64, [0x11; 32], [0xa1; 32]),
            (200_u64, [0x22; 32], [0xa2; 32]),
            (300_u64, [0x33; 32], [0xa3; 32]),
        ];

        let first = storage_manifest_root(&chunks);
        let second = storage_manifest_root(&chunks);

        assert_eq!(first, second);
        assert_ne!(first, storage_manifest_root(&chunks[..2]));
    }

    #[test]
    fn manifest_proof_verifies_and_rejects_tampering() {
        let leaf0 = storage_manifest_leaf(0, 100, [0x11; 32], [0xa1; 32]);
        let leaf1 = storage_manifest_leaf(1, 200, [0x22; 32], [0xa2; 32]);
        let leaf2 = storage_manifest_leaf(2, 300, [0x33; 32], [0xa3; 32]);

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
            root, 1, 200, [0x22; 32], [0xa2; 32], &proof
        ));
        assert!(!verify_storage_manifest_proof(
            root, 1, 201, [0x22; 32], [0xa2; 32], &proof
        ));
        assert!(!verify_storage_manifest_proof(
            root, 1, 200, [0x23; 32], [0xa2; 32], &proof
        ));
    }

    #[test]
    fn manifest_proof_rejects_wrong_range_root() {
        let leaf0 = storage_manifest_leaf(0, 100, [0x11; 32], [0xa1; 32]);
        let leaf1 = storage_manifest_leaf(1, 200, [0x22; 32], [0xa2; 32]);
        let root = storage_manifest_node(leaf0, leaf1);
        let proof = [ManifestProofStep {
            sibling: leaf0,
            sibling_is_left: true,
        }];

        assert!(!verify_storage_manifest_proof(
            root, 1, 200, [0x22; 32], [0xff; 32], &proof
        ));
    }

    #[test]
    fn empty_manifest_has_domain_separated_root() {
        let empty = storage_manifest_root(&[]);
        assert_ne!(empty, [0_u8; 32]);
        assert_eq!(empty, storage_manifest_root(&[]));
    }

    #[test]
    fn range_merkle_root_and_proof_verify() {
        let segment0 = b"abcdefgh";
        let segment1 = b"ijklmnop";
        let segment2 = b"qrstuvwx";

        let leaf0 = storage_range_leaf(0, segment0);
        let leaf1 = storage_range_leaf(1, segment1);
        let leaf2 = storage_range_leaf(2, segment2);

        let parent01 = storage_range_node(leaf0, leaf1);
        let parent22 = storage_range_node(leaf2, leaf2);
        let root = storage_range_node(parent01, parent22);

        assert_eq!(
            root,
            storage_range_root(b"abcdefghijklmnopqrstuvwx", 8).unwrap()
        );

        let proof = [
            RangeProofStep {
                sibling: leaf0,
                sibling_is_left: true,
            },
            RangeProofStep {
                sibling: parent22,
                sibling_is_left: false,
            },
        ];

        assert!(verify_storage_range_proof(root, 1, segment1, &proof));
        assert!(!verify_storage_range_proof(root, 1, b"ijklmnop!", &proof));
        assert!(!verify_storage_range_proof(root, 2, segment1, &proof));
    }

    #[test]
    fn range_root_rejects_zero_segment_size() {
        assert!(storage_range_root(b"data", 0).is_err());
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
    fn epoch_accumulator_rejects_replay_and_tracks_diversity() {
        let mut epoch = ServiceEpochAccumulator::new(720, 1440).unwrap();

        epoch
            .record_success([0x01; 32], [0x10; 32], [0x20; 32], 4096)
            .unwrap();
        epoch
            .record_success([0x02; 32], [0x11; 32], [0x21; 32], 1024)
            .unwrap();

        assert_eq!(epoch.challenges_passed, 2);
        assert_eq!(epoch.verified_bytes_served, 5120);
        assert_eq!(epoch.distinct_requester_count(), 2);
        assert_eq!(epoch.distinct_challenge_block_count(), 2);
        assert_eq!(epoch.evidence_count(), 2);

        assert!(epoch
            .record_success([0x01; 32], [0x12; 32], [0x22; 32], 1)
            .is_err());

        epoch.record_failure([0x03; 32], true).unwrap();
        assert_eq!(epoch.challenges_failed, 1);
        assert_eq!(epoch.deadlines_missed, 1);
        assert_eq!(epoch.evidence_count(), 3);
    }

    #[test]
    fn evidence_root_is_order_independent_and_replay_sensitive() {
        let a = [0x01; 32];
        let b = [0x02; 32];
        let c = [0x03; 32];

        let first = service_evidence_root(&[a, b, c]);
        let reordered = service_evidence_root(&[c, a, b]);
        let missing = service_evidence_root(&[a, b]);

        assert_eq!(first, reordered);
        assert_ne!(first, missing);
        assert_ne!(service_evidence_root(&[]), [0_u8; 32]);
    }

    #[test]
    fn accumulator_evidence_root_matches_unique_evidence_set() {
        let mut epoch = ServiceEpochAccumulator::new(0, 720).unwrap();
        epoch
            .record_success([0x01; 32], [0x10; 32], [0x20; 32], 100)
            .unwrap();
        epoch.record_failure([0x02; 32], false).unwrap();

        assert_eq!(
            epoch.evidence_root(),
            service_evidence_root(&[[0x01; 32], [0x02; 32]])
        );
    }

    #[test]
    fn finalized_epoch_report_binds_counters_root_and_eligibility() {
        let mut epoch = ServiceEpochAccumulator::new(720, 1440).unwrap();
        epoch
            .record_success([0x01; 32], [0x10; 32], [0x20; 32], 4096)
            .unwrap();
        epoch
            .record_success([0x02; 32], [0x11; 32], [0x21; 32], 2048)
            .unwrap();

        let report = finalize_service_epoch_report([0xaa; 32], &epoch, 2, 2, 2);

        assert_eq!(report.service_node_id, [0xaa; 32]);
        assert_eq!(report.epoch_start_height, 720);
        assert_eq!(report.epoch_end_height, 1440);
        assert_eq!(report.challenges_passed, 2);
        assert_eq!(report.challenges_failed, 0);
        assert_eq!(report.deadlines_missed, 0);
        assert_eq!(report.verified_bytes_served, 6144);
        assert_eq!(report.distinct_requester_count, 2);
        assert_eq!(report.distinct_challenge_block_count, 2);
        assert_eq!(report.evidence_root, epoch.evidence_root());
        assert!(report.eligible);
        assert_eq!(report.eligibility_weight, 6144);
    }

    #[test]
    fn conservative_eligibility_requires_success_diversity_and_no_missed_deadline() {
        let mut epoch = ServiceEpochAccumulator::new(0, 720).unwrap();
        epoch
            .record_success([0x01; 32], [0x10; 32], [0x20; 32], 4096)
            .unwrap();
        epoch
            .record_success([0x02; 32], [0x11; 32], [0x21; 32], 4096)
            .unwrap();
        epoch
            .record_success([0x03; 32], [0x12; 32], [0x22; 32], 4096)
            .unwrap();

        let eligible = evaluate_service_eligibility(&epoch, 3, 3, 3);
        assert!(eligible.eligible);
        assert_eq!(eligible.weight, 12_288);

        let not_diverse = evaluate_service_eligibility(&epoch, 3, 4, 3);
        assert!(!not_diverse.eligible);

        epoch.record_failure([0x04; 32], true).unwrap();
        let late = evaluate_service_eligibility(&epoch, 3, 3, 3);
        assert!(!late.eligible);
        assert_eq!(late.weight, 0);
    }

    #[test]
    fn invalid_epoch_bounds_are_rejected() {
        assert!(ServiceEpochAccumulator::new(100, 100).is_err());
        assert!(ServiceEpochAccumulator::new(101, 100).is_err());
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
