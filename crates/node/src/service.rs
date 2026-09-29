use crate::state::{PersistedServiceSuccess, StateStore};
use crate::work::{keccak256, Hash32};
use k256::ecdsa::{
    signature::hazmat::{PrehashSigner, PrehashVerifier},
    Signature, SigningKey, VerifyingKey,
};
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
pub const SERVICE_NODE_ID_DOMAIN: &[u8] = b"NIAHCIA/SERVICE-NODE-ID/V1";
pub const STORAGE_RESPONSE_BYTES_DOMAIN: &[u8] = b"NIAHCIA/STORAGE-RESPONSE-BYTES/V1";

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
    pub verified_bytes_served: u64,
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
        self.verified_bytes_served = self.verified_bytes_served.saturating_add(verified_bytes);
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

pub fn record_verified_storage_response(
    epoch: &mut ServiceEpochAccumulator,
    network_id: &[u8],
    challenge: &StorageChallengeV1,
    challenger_public_key_sec1: &[u8],
    response: &StorageResponseV1,
    provider_public_key_sec1: &[u8],
    manifest_root: Hash32,
) -> Result<Hash32, String> {
    if challenge.challenge_height < epoch.epoch_start_height
        || challenge.challenge_height >= epoch.epoch_end_height
    {
        return Err("storage challenge height is outside service epoch".into());
    }

    verify_storage_challenge_signature(
        context.network_id,
        challenge,
        context.challenger_public_key_sec1,
    )?;
    verify_storage_response_signature(
        context.network_id,
        response,
        context.provider_public_key_sec1,
    )?;
    let verified_bytes =
        verify_storage_response_evidence(response, challenge, context.manifest_root)?;

    let evidence_key = evidence_replay_key(
        challenge.challenge_id,
        response.service_node_id,
        response.response_id,
    );
    epoch.record_success(
        evidence_key,
        challenge.challenger_id,
        challenge.challenge_block_id,
        verified_bytes,
    )?;
    Ok(evidence_key)
}

pub struct PersistentStorageResponseContext<'a> {
    pub store: &'a StateStore,
    pub network_id: &'a [u8],
    pub challenger_public_key_sec1: &'a [u8],
    pub provider_public_key_sec1: &'a [u8],
    pub manifest_root: Hash32,
}

pub fn record_verified_storage_response_persistent(
    epoch: &mut ServiceEpochAccumulator,
    challenge: &StorageChallengeV1,
    response: &StorageResponseV1,
    context: PersistentStorageResponseContext<'_>,
) -> Result<Hash32, String> {
    if epoch.seen_evidence.contains(&evidence_replay_key(
        challenge.challenge_id,
        response.service_node_id,
        response.response_id,
    )) {
        return Err("duplicate service evidence".into());
    }

    if challenge.challenge_height < epoch.epoch_start_height
        || challenge.challenge_height >= epoch.epoch_end_height
    {
        return Err("storage challenge height is outside service epoch".into());
    }

    verify_storage_challenge_signature(network_id, challenge, challenger_public_key_sec1)?;
    verify_storage_response_signature(network_id, response, provider_public_key_sec1)?;
    let verified_bytes = verify_storage_response_evidence(response, challenge, manifest_root)?;

    let evidence_key = evidence_replay_key(
        challenge.challenge_id,
        response.service_node_id,
        response.response_id,
    );
    let persisted = PersistedServiceSuccess {
        evidence_key,
        epoch_start_height: epoch.epoch_start_height,
        epoch_end_height: epoch.epoch_end_height,
        requester_id: challenge.challenger_id,
        challenge_block_id: challenge.challenge_block_id,
        verified_bytes,
    };

    if !context.store.insert_service_success(&persisted)? {
        return Err("duplicate persistent service evidence".into());
    }

    epoch.record_success(
        evidence_key,
        challenge.challenger_id,
        challenge.challenge_block_id,
        verified_bytes,
    )?;
    Ok(evidence_key)
}

pub fn load_persisted_service_epoch(
    store: &StateStore,
    epoch_start_height: u64,
    epoch_end_height: u64,
) -> Result<ServiceEpochAccumulator, String> {
    let mut epoch = ServiceEpochAccumulator::new(epoch_start_height, epoch_end_height)?;
    for success in store.load_service_successes(epoch_start_height, epoch_end_height)? {
        epoch.record_success(
            success.evidence_key,
            success.requester_id,
            success.challenge_block_id,
            success.verified_bytes,
        )?;
    }
    Ok(epoch)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEligibility {
    pub eligible: bool,
    pub weight: u64,
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
        weight: epoch.verified_bytes_served.max(epoch.challenges_passed),
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
    pub verified_bytes_served: u64,
    pub distinct_requester_count: u64,
    pub distinct_challenge_block_count: u64,
    pub evidence_root: Hash32,
    pub eligible: bool,
    pub eligibility_weight: u64,
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

pub const STORAGE_COMMITMENT_OBJECT_TYPE: u64 = 0x0205;
pub const STORAGE_COMMITMENT_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageCommitmentV1 {
    pub commitment_id: Hash32,
    pub service_node_id: Hash32,
    pub operator_id: Hash32,
    pub service_class: String,
    pub object_id: Hash32,
    pub manifest_root: Hash32,
    pub chunk_count: u64,
    pub total_bytes: u64,
    pub retained_from_block: u64,
    pub retained_until_block: u64,
    pub commitment_nonce: u64,
    pub created_block: Hash32,
    pub signature: Vec<u8>,
}

fn encode_storage_commitment_payload(
    commitment: &StorageCommitmentV1,
    include_commitment_id: bool,
    include_signature: bool,
) -> Vec<u8> {
    let mut fields = 12_u64;
    if include_commitment_id {
        fields += 1;
    }
    if include_signature {
        fields += 1;
    }

    let mut out = Vec::new();
    cbor_map_len(&mut out, fields);

    cbor_uint(&mut out, 1);
    cbor_uint(&mut out, STORAGE_COMMITMENT_SCHEMA_VERSION);

    if include_commitment_id {
        cbor_uint(&mut out, 2);
        cbor_bytes(&mut out, &commitment.commitment_id);
    }

    cbor_uint(&mut out, 3);
    cbor_bytes(&mut out, &commitment.service_node_id);
    cbor_uint(&mut out, 4);
    cbor_bytes(&mut out, &commitment.operator_id);
    cbor_uint(&mut out, 5);
    cbor_text(&mut out, &commitment.service_class);
    cbor_uint(&mut out, 6);
    cbor_bytes(&mut out, &commitment.object_id);
    cbor_uint(&mut out, 7);
    cbor_bytes(&mut out, &commitment.manifest_root);
    cbor_uint(&mut out, 8);
    cbor_uint(&mut out, commitment.chunk_count);
    cbor_uint(&mut out, 9);
    cbor_uint(&mut out, commitment.total_bytes);
    cbor_uint(&mut out, 10);
    cbor_uint(&mut out, commitment.retained_from_block);
    cbor_uint(&mut out, 11);
    cbor_uint(&mut out, commitment.retained_until_block);
    cbor_uint(&mut out, 12);
    cbor_uint(&mut out, commitment.commitment_nonce);
    cbor_uint(&mut out, 13);
    cbor_bytes(&mut out, &commitment.created_block);

    if include_signature {
        cbor_uint(&mut out, 14);
        cbor_bytes(&mut out, &commitment.signature);
    }

    out
}

pub fn storage_commitment_id_preimage(commitment: &StorageCommitmentV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_COMMITMENT_OBJECT_TYPE,
        STORAGE_COMMITMENT_SCHEMA_VERSION,
        &encode_storage_commitment_payload(commitment, false, false),
    )
}

pub fn storage_commitment_signing_preimage(commitment: &StorageCommitmentV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_COMMITMENT_OBJECT_TYPE,
        STORAGE_COMMITMENT_SCHEMA_VERSION,
        &encode_storage_commitment_payload(commitment, true, false),
    )
}

pub fn storage_commitment_canonical_bytes(commitment: &StorageCommitmentV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_COMMITMENT_OBJECT_TYPE,
        STORAGE_COMMITMENT_SCHEMA_VERSION,
        &encode_storage_commitment_payload(commitment, true, true),
    )
}

pub fn derive_storage_commitment_id(network_id: &[u8], commitment: &StorageCommitmentV1) -> Hash32 {
    generic_protocol_digest(
        b"ID/STORAGE_COMMITMENT",
        network_id,
        &storage_commitment_id_preimage(commitment),
    )
}

pub fn storage_commitment_signing_digest(
    network_id: &[u8],
    commitment: &StorageCommitmentV1,
) -> Hash32 {
    generic_protocol_digest(
        b"SIGN/STORAGE_COMMITMENT",
        network_id,
        &storage_commitment_signing_preimage(commitment),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageCommitmentParams {
    pub service_node_id: Hash32,
    pub operator_id: Hash32,
    pub service_class: String,
    pub object_id: Hash32,
    pub manifest_root: Hash32,
    pub chunk_count: u64,
    pub total_bytes: u64,
    pub retained_from_block: u64,
    pub retained_until_block: u64,
    pub commitment_nonce: u64,
    pub created_block: Hash32,
}

pub fn build_storage_commitment_v1(
    network_id: &[u8],
    params: StorageCommitmentParams,
) -> Result<StorageCommitmentV1, String> {
    if params.chunk_count == 0 {
        return Err("storage commitment chunk_count must be non-zero".into());
    }
    if params.total_bytes == 0 {
        return Err("storage commitment total_bytes must be non-zero".into());
    }
    if params.retained_until_block <= params.retained_from_block {
        return Err("storage commitment retention interval must be non-empty".into());
    }
    if params.service_class.is_empty() {
        return Err("storage commitment service_class must not be empty".into());
    }

    let mut commitment = StorageCommitmentV1 {
        commitment_id: [0_u8; 32],
        service_node_id: params.service_node_id,
        operator_id: params.operator_id,
        service_class: params.service_class,
        object_id: params.object_id,
        manifest_root: params.manifest_root,
        chunk_count: params.chunk_count,
        total_bytes: params.total_bytes,
        retained_from_block: params.retained_from_block,
        retained_until_block: params.retained_until_block,
        commitment_nonce: params.commitment_nonce,
        created_block: params.created_block,
        signature: Vec::new(),
    };
    commitment.commitment_id = derive_storage_commitment_id(network_id, &commitment);
    Ok(commitment)
}

pub fn sign_storage_commitment(
    network_id: &[u8],
    commitment: &mut StorageCommitmentV1,
    secret_key: &[u8; 32],
) -> Result<(), String> {
    let signing_key = SigningKey::from_slice(secret_key)
        .map_err(|_| "invalid secp256k1 service signing key".to_string())?;
    let public_key = signing_key.verifying_key().to_encoded_point(true);
    verify_service_node_identity(commitment.service_node_id, public_key.as_bytes())?;

    let expected_id = derive_storage_commitment_id(network_id, commitment);
    if commitment.commitment_id != expected_id {
        return Err("storage commitment_id does not match commitment contents".into());
    }

    let digest = storage_commitment_signing_digest(network_id, commitment);
    let signature: Signature = signing_key
        .sign_prehash(&digest)
        .map_err(|_| "failed to sign storage commitment".to_string())?;
    commitment.signature = signature.to_bytes().to_vec();
    Ok(())
}

pub fn verify_storage_commitment_signature(
    network_id: &[u8],
    commitment: &StorageCommitmentV1,
    public_key_sec1: &[u8],
) -> Result<(), String> {
    verify_service_node_identity(commitment.service_node_id, public_key_sec1)?;

    let expected_id = derive_storage_commitment_id(network_id, commitment);
    if commitment.commitment_id != expected_id {
        return Err("storage commitment_id does not match commitment contents".into());
    }
    if commitment.signature.len() != 64 {
        return Err("storage commitment signature must be 64-byte compact ECDSA".into());
    }

    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|_| "invalid secp256k1 service public key".to_string())?;
    let signature = Signature::from_slice(&commitment.signature)
        .map_err(|_| "invalid secp256k1 storage commitment signature".to_string())?;
    let digest = storage_commitment_signing_digest(network_id, commitment);

    verifying_key
        .verify_prehash(&digest, &signature)
        .map_err(|_| "invalid storage commitment signature".to_string())
}

pub const STORAGE_CHALLENGE_OBJECT_TYPE: u64 = 0x0206;
pub const STORAGE_CHALLENGE_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageChallengeV1 {
    pub challenge_id: Hash32,
    pub commitment_id: Hash32,
    pub challenge_block_id: Hash32,
    pub challenge_height: u64,
    pub challenge_seed: Hash32,
    pub requested_ranges: Vec<ChallengeSegment>,
    pub issued_at: u64,
    pub response_deadline: u64,
    pub challenger_id: Hash32,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageChallengeParams {
    pub commitment_id: Hash32,
    pub challenge_block_id: Hash32,
    pub challenge_height: u64,
    pub requested_ranges: Vec<ChallengeSegment>,
    pub issued_at: u64,
    pub response_deadline: u64,
    pub challenger_id: Hash32,
}

fn cbor_challenge_segments(out: &mut Vec<u8>, segments: &[ChallengeSegment]) {
    cbor_major_len(out, 4, segments.len() as u64);
    for segment in segments {
        cbor_major_len(out, 4, 4);
        cbor_uint(out, segment.chunk_index);
        cbor_uint(out, segment.segment_index);
        cbor_uint(out, segment.offset);
        cbor_uint(out, segment.length);
    }
}

fn encode_storage_challenge_payload(
    challenge: &StorageChallengeV1,
    include_challenge_id: bool,
    include_signature: bool,
) -> Vec<u8> {
    let mut fields = 9_u64;
    if include_challenge_id {
        fields += 1;
    }
    if include_signature {
        fields += 1;
    }

    let mut out = Vec::new();
    cbor_map_len(&mut out, fields);

    cbor_uint(&mut out, 1);
    cbor_uint(&mut out, STORAGE_CHALLENGE_SCHEMA_VERSION);

    if include_challenge_id {
        cbor_uint(&mut out, 2);
        cbor_bytes(&mut out, &challenge.challenge_id);
    }

    cbor_uint(&mut out, 3);
    cbor_bytes(&mut out, &challenge.commitment_id);
    cbor_uint(&mut out, 4);
    cbor_bytes(&mut out, &challenge.challenge_block_id);
    cbor_uint(&mut out, 5);
    cbor_uint(&mut out, challenge.challenge_height);
    cbor_uint(&mut out, 6);
    cbor_bytes(&mut out, &challenge.challenge_seed);
    cbor_uint(&mut out, 7);
    cbor_challenge_segments(&mut out, &challenge.requested_ranges);
    cbor_uint(&mut out, 8);
    cbor_uint(&mut out, challenge.issued_at);
    cbor_uint(&mut out, 9);
    cbor_uint(&mut out, challenge.response_deadline);
    cbor_uint(&mut out, 10);
    cbor_bytes(&mut out, &challenge.challenger_id);

    if include_signature {
        cbor_uint(&mut out, 11);
        cbor_bytes(&mut out, &challenge.signature);
    }

    out
}

pub fn storage_challenge_id_preimage(challenge: &StorageChallengeV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_CHALLENGE_OBJECT_TYPE,
        STORAGE_CHALLENGE_SCHEMA_VERSION,
        &encode_storage_challenge_payload(challenge, false, false),
    )
}

pub fn storage_challenge_signing_preimage(challenge: &StorageChallengeV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_CHALLENGE_OBJECT_TYPE,
        STORAGE_CHALLENGE_SCHEMA_VERSION,
        &encode_storage_challenge_payload(challenge, true, false),
    )
}

pub fn storage_challenge_canonical_bytes(challenge: &StorageChallengeV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_CHALLENGE_OBJECT_TYPE,
        STORAGE_CHALLENGE_SCHEMA_VERSION,
        &encode_storage_challenge_payload(challenge, true, true),
    )
}

pub fn derive_storage_challenge_id(network_id: &[u8], challenge: &StorageChallengeV1) -> Hash32 {
    generic_protocol_digest(
        b"ID/STORAGE_CHALLENGE",
        network_id,
        &storage_challenge_id_preimage(challenge),
    )
}

pub fn storage_challenge_signing_digest(
    network_id: &[u8],
    challenge: &StorageChallengeV1,
) -> Hash32 {
    generic_protocol_digest(
        b"SIGN/STORAGE_CHALLENGE",
        network_id,
        &storage_challenge_signing_preimage(challenge),
    )
}

pub fn build_storage_challenge_v1(
    network_id: &[u8],
    params: StorageChallengeParams,
) -> Result<StorageChallengeV1, String> {
    if params.requested_ranges.is_empty() {
        return Err("storage challenge must request at least one segment".into());
    }
    if params
        .requested_ranges
        .iter()
        .any(|range| range.length == 0)
    {
        return Err("storage challenge segment lengths must be non-zero".into());
    }
    if params.response_deadline <= params.issued_at {
        return Err("storage challenge deadline must be after issued_at".into());
    }

    let challenge_seed = storage_challenge_seed(params.challenge_block_id, params.commitment_id);
    let mut challenge = StorageChallengeV1 {
        challenge_id: [0_u8; 32],
        commitment_id: params.commitment_id,
        challenge_block_id: params.challenge_block_id,
        challenge_height: params.challenge_height,
        challenge_seed,
        requested_ranges: params.requested_ranges,
        issued_at: params.issued_at,
        response_deadline: params.response_deadline,
        challenger_id: params.challenger_id,
        signature: Vec::new(),
    };
    challenge.challenge_id = derive_storage_challenge_id(network_id, &challenge);
    Ok(challenge)
}

pub fn sign_storage_challenge(
    network_id: &[u8],
    challenge: &mut StorageChallengeV1,
    secret_key: &[u8; 32],
) -> Result<(), String> {
    let signing_key = SigningKey::from_slice(secret_key)
        .map_err(|_| "invalid secp256k1 challenge signing key".to_string())?;
    let public_key = signing_key.verifying_key().to_encoded_point(true);
    verify_service_node_identity(challenge.challenger_id, public_key.as_bytes())?;

    let expected_seed =
        storage_challenge_seed(challenge.challenge_block_id, challenge.commitment_id);
    if challenge.challenge_seed != expected_seed {
        return Err("storage challenge seed does not match challenge block and commitment".into());
    }

    let expected_id = derive_storage_challenge_id(network_id, challenge);
    if challenge.challenge_id != expected_id {
        return Err("storage challenge_id does not match challenge contents".into());
    }

    let digest = storage_challenge_signing_digest(network_id, challenge);
    let signature: Signature = signing_key
        .sign_prehash(&digest)
        .map_err(|_| "failed to sign storage challenge".to_string())?;
    challenge.signature = signature.to_bytes().to_vec();
    Ok(())
}

pub fn verify_storage_challenge_signature(
    network_id: &[u8],
    challenge: &StorageChallengeV1,
    public_key_sec1: &[u8],
) -> Result<(), String> {
    verify_service_node_identity(challenge.challenger_id, public_key_sec1)?;

    if challenge.requested_ranges.is_empty()
        || challenge
            .requested_ranges
            .iter()
            .any(|range| range.length == 0)
    {
        return Err("storage challenge contains invalid requested segments".into());
    }
    if challenge.response_deadline <= challenge.issued_at {
        return Err("storage challenge deadline must be after issued_at".into());
    }

    let expected_seed =
        storage_challenge_seed(challenge.challenge_block_id, challenge.commitment_id);
    if challenge.challenge_seed != expected_seed {
        return Err("storage challenge seed does not match challenge block and commitment".into());
    }

    let expected_id = derive_storage_challenge_id(network_id, challenge);
    if challenge.challenge_id != expected_id {
        return Err("storage challenge_id does not match challenge contents".into());
    }
    if challenge.signature.len() != 64 {
        return Err("storage challenge signature must be 64-byte compact ECDSA".into());
    }

    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|_| "invalid secp256k1 challenge public key".to_string())?;
    let signature = Signature::from_slice(&challenge.signature)
        .map_err(|_| "invalid secp256k1 storage challenge signature".to_string())?;
    let digest = storage_challenge_signing_digest(network_id, challenge);

    verifying_key
        .verify_prehash(&digest, &signature)
        .map_err(|_| "invalid storage challenge signature".to_string())
}

pub const STORAGE_RESPONSE_OBJECT_TYPE: u64 = 0x0207;
pub const STORAGE_RESPONSE_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRangeProofV1 {
    pub chunk_index: u64,
    pub segment_index: u64,
    pub offset: u64,
    pub length: u64,
    pub returned_bytes: Vec<u8>,
    pub chunk_length: u64,
    pub chunk_hash: Hash32,
    pub range_root: Hash32,
    pub range_proof: Vec<RangeProofStep>,
    pub manifest_proof: Vec<ManifestProofStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageResponseV1 {
    pub response_id: Hash32,
    pub challenge_id: Hash32,
    pub commitment_id: Hash32,
    pub service_node_id: Hash32,
    pub answered_at: u64,
    pub range_proofs: Vec<StorageRangeProofV1>,
    pub response_bytes_hash: Hash32,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageResponseParams {
    pub challenge_id: Hash32,
    pub commitment_id: Hash32,
    pub service_node_id: Hash32,
    pub answered_at: u64,
    pub range_proofs: Vec<StorageRangeProofV1>,
}

fn cbor_bool(out: &mut Vec<u8>, value: bool) {
    out.push(if value { 0xf5 } else { 0xf4 });
}

fn cbor_merkle_steps(out: &mut Vec<u8>, steps: &[(Hash32, bool)]) {
    cbor_major_len(out, 4, steps.len() as u64);
    for (sibling, sibling_is_left) in steps {
        cbor_major_len(out, 4, 2);
        cbor_bytes(out, sibling);
        cbor_bool(out, *sibling_is_left);
    }
}

fn cbor_storage_range_proofs(out: &mut Vec<u8>, proofs: &[StorageRangeProofV1]) {
    cbor_major_len(out, 4, proofs.len() as u64);
    for proof in proofs {
        cbor_major_len(out, 4, 10);
        cbor_uint(out, proof.chunk_index);
        cbor_uint(out, proof.segment_index);
        cbor_uint(out, proof.offset);
        cbor_uint(out, proof.length);
        cbor_bytes(out, &proof.returned_bytes);
        cbor_uint(out, proof.chunk_length);
        cbor_bytes(out, &proof.chunk_hash);
        cbor_bytes(out, &proof.range_root);

        let range_steps: Vec<(Hash32, bool)> = proof
            .range_proof
            .iter()
            .map(|step| (step.sibling, step.sibling_is_left))
            .collect();
        cbor_merkle_steps(out, &range_steps);

        let manifest_steps: Vec<(Hash32, bool)> = proof
            .manifest_proof
            .iter()
            .map(|step| (step.sibling, step.sibling_is_left))
            .collect();
        cbor_merkle_steps(out, &manifest_steps);
    }
}

pub fn storage_response_bytes_hash(proofs: &[StorageRangeProofV1]) -> Hash32 {
    let mut canonical = Vec::new();
    cbor_major_len(&mut canonical, 4, proofs.len() as u64);
    for proof in proofs {
        cbor_major_len(&mut canonical, 4, 5);
        cbor_uint(&mut canonical, proof.chunk_index);
        cbor_uint(&mut canonical, proof.segment_index);
        cbor_uint(&mut canonical, proof.offset);
        cbor_uint(&mut canonical, proof.length);
        cbor_bytes(&mut canonical, &proof.returned_bytes);
    }

    let mut preimage = Vec::with_capacity(STORAGE_RESPONSE_BYTES_DOMAIN.len() + canonical.len());
    preimage.extend_from_slice(STORAGE_RESPONSE_BYTES_DOMAIN);
    preimage.extend_from_slice(&canonical);
    keccak256(&preimage)
}

fn encode_storage_response_payload(
    response: &StorageResponseV1,
    include_response_id: bool,
    include_signature: bool,
) -> Vec<u8> {
    let mut fields = 7_u64;
    if include_response_id {
        fields += 1;
    }
    if include_signature {
        fields += 1;
    }

    let mut out = Vec::new();
    cbor_map_len(&mut out, fields);

    cbor_uint(&mut out, 1);
    cbor_uint(&mut out, STORAGE_RESPONSE_SCHEMA_VERSION);

    if include_response_id {
        cbor_uint(&mut out, 2);
        cbor_bytes(&mut out, &response.response_id);
    }

    cbor_uint(&mut out, 3);
    cbor_bytes(&mut out, &response.challenge_id);
    cbor_uint(&mut out, 4);
    cbor_bytes(&mut out, &response.commitment_id);
    cbor_uint(&mut out, 5);
    cbor_bytes(&mut out, &response.service_node_id);
    cbor_uint(&mut out, 6);
    cbor_uint(&mut out, response.answered_at);
    cbor_uint(&mut out, 7);
    cbor_storage_range_proofs(&mut out, &response.range_proofs);
    cbor_uint(&mut out, 8);
    cbor_bytes(&mut out, &response.response_bytes_hash);

    if include_signature {
        cbor_uint(&mut out, 9);
        cbor_bytes(&mut out, &response.signature);
    }

    out
}

pub fn storage_response_id_preimage(response: &StorageResponseV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_RESPONSE_OBJECT_TYPE,
        STORAGE_RESPONSE_SCHEMA_VERSION,
        &encode_storage_response_payload(response, false, false),
    )
}

pub fn storage_response_signing_preimage(response: &StorageResponseV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_RESPONSE_OBJECT_TYPE,
        STORAGE_RESPONSE_SCHEMA_VERSION,
        &encode_storage_response_payload(response, true, false),
    )
}

pub fn storage_response_canonical_bytes(response: &StorageResponseV1) -> Vec<u8> {
    encode_nce_envelope(
        STORAGE_RESPONSE_OBJECT_TYPE,
        STORAGE_RESPONSE_SCHEMA_VERSION,
        &encode_storage_response_payload(response, true, true),
    )
}

pub fn derive_storage_response_id(network_id: &[u8], response: &StorageResponseV1) -> Hash32 {
    generic_protocol_digest(
        b"ID/STORAGE_RESPONSE",
        network_id,
        &storage_response_id_preimage(response),
    )
}

pub fn storage_response_signing_digest(network_id: &[u8], response: &StorageResponseV1) -> Hash32 {
    generic_protocol_digest(
        b"SIGN/STORAGE_RESPONSE",
        network_id,
        &storage_response_signing_preimage(response),
    )
}

pub fn build_storage_response_v1(
    network_id: &[u8],
    params: StorageResponseParams,
) -> Result<StorageResponseV1, String> {
    if params.range_proofs.is_empty() {
        return Err("storage response must contain at least one range proof".into());
    }
    if params
        .range_proofs
        .iter()
        .any(|proof| proof.length == 0 || proof.returned_bytes.len() as u64 != proof.length)
    {
        return Err("storage response range proof length mismatch".into());
    }

    let response_bytes_hash = storage_response_bytes_hash(&params.range_proofs);
    let mut response = StorageResponseV1 {
        response_id: [0_u8; 32],
        challenge_id: params.challenge_id,
        commitment_id: params.commitment_id,
        service_node_id: params.service_node_id,
        answered_at: params.answered_at,
        range_proofs: params.range_proofs,
        response_bytes_hash,
        signature: Vec::new(),
    };
    response.response_id = derive_storage_response_id(network_id, &response);
    Ok(response)
}

pub fn sign_storage_response(
    network_id: &[u8],
    response: &mut StorageResponseV1,
    secret_key: &[u8; 32],
) -> Result<(), String> {
    let signing_key = SigningKey::from_slice(secret_key)
        .map_err(|_| "invalid secp256k1 storage response signing key".to_string())?;
    let public_key = signing_key.verifying_key().to_encoded_point(true);
    verify_service_node_identity(response.service_node_id, public_key.as_bytes())?;

    if response.response_bytes_hash != storage_response_bytes_hash(&response.range_proofs) {
        return Err("storage response bytes hash does not match returned bytes".into());
    }

    let expected_id = derive_storage_response_id(network_id, response);
    if response.response_id != expected_id {
        return Err("storage response_id does not match response contents".into());
    }

    let digest = storage_response_signing_digest(network_id, response);
    let signature: Signature = signing_key
        .sign_prehash(&digest)
        .map_err(|_| "failed to sign storage response".to_string())?;
    response.signature = signature.to_bytes().to_vec();
    Ok(())
}

pub fn verify_storage_response_signature(
    network_id: &[u8],
    response: &StorageResponseV1,
    public_key_sec1: &[u8],
) -> Result<(), String> {
    verify_service_node_identity(response.service_node_id, public_key_sec1)?;

    if response.range_proofs.is_empty()
        || response
            .range_proofs
            .iter()
            .any(|proof| proof.length == 0 || proof.returned_bytes.len() as u64 != proof.length)
    {
        return Err("storage response contains invalid range proof lengths".into());
    }

    if response.response_bytes_hash != storage_response_bytes_hash(&response.range_proofs) {
        return Err("storage response bytes hash does not match returned bytes".into());
    }

    let expected_id = derive_storage_response_id(network_id, response);
    if response.response_id != expected_id {
        return Err("storage response_id does not match response contents".into());
    }
    if response.signature.len() != 64 {
        return Err("storage response signature must be 64-byte compact ECDSA".into());
    }

    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|_| "invalid secp256k1 storage response public key".to_string())?;
    let signature = Signature::from_slice(&response.signature)
        .map_err(|_| "invalid secp256k1 storage response signature".to_string())?;
    let digest = storage_response_signing_digest(network_id, response);

    verifying_key
        .verify_prehash(&digest, &signature)
        .map_err(|_| "invalid storage response signature".to_string())
}

pub fn verify_storage_response_evidence(
    response: &StorageResponseV1,
    challenge: &StorageChallengeV1,
    manifest_root: Hash32,
) -> Result<u64, String> {
    verify_response_meta(
        &ExpectedResponseMeta {
            challenge_id: challenge.challenge_id,
            commitment_id: challenge.commitment_id,
            response_deadline: challenge.response_deadline,
        },
        &ResponseMeta {
            challenge_id: response.challenge_id,
            commitment_id: response.commitment_id,
            answered_at: response.answered_at,
        },
    )?;

    if response.range_proofs.len() != challenge.requested_ranges.len() {
        return Err("storage response proof count does not match challenge".into());
    }

    let mut verified_bytes = 0_u64;
    for (proof, requested) in response
        .range_proofs
        .iter()
        .zip(&challenge.requested_ranges)
    {
        if proof.chunk_index != requested.chunk_index
            || proof.segment_index != requested.segment_index
            || proof.offset != requested.offset
            || proof.length != requested.length
        {
            return Err("storage response proof does not match requested segment order".into());
        }

        if proof.returned_bytes.len() as u64 != proof.length {
            return Err("storage response returned byte length mismatch".into());
        }

        if !verify_storage_range_proof(
            proof.range_root,
            proof.segment_index,
            &proof.returned_bytes,
            &proof.range_proof,
        ) {
            return Err("invalid storage range proof".into());
        }

        if !verify_storage_manifest_proof(
            manifest_root,
            proof.chunk_index,
            proof.chunk_length,
            proof.chunk_hash,
            proof.range_root,
            &proof.manifest_proof,
        ) {
            return Err("invalid storage manifest proof".into());
        }

        verified_bytes = verified_bytes
            .checked_add(proof.length)
            .ok_or_else(|| "verified byte count overflow".to_string())?;
    }

    Ok(verified_bytes)
}

pub const SERVICE_EPOCH_REPORT_OBJECT_TYPE: u64 = 0x0208;
pub const SERVICE_EPOCH_REPORT_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEpochReportV1 {
    pub report_id: Hash32,
    pub service_node_id: Hash32,
    pub operator_id: Hash32,
    pub epoch_start_height: u64,
    pub epoch_end_height: u64,
    pub commitments_sampled: u64,
    pub challenges_passed: u64,
    pub challenges_failed: u64,
    pub deadlines_missed: u64,
    pub verified_bytes_served: u64,
    pub distinct_requester_count: u64,
    pub distinct_challenge_block_count: u64,
    pub service_classes: Vec<String>,
    pub evidence_root: Hash32,
    pub eligibility_weight: u64,
    pub created_block: Hash32,
    pub signature: Vec<u8>,
}

fn cbor_uint(out: &mut Vec<u8>, value: u64) {
    match value {
        0..=23 => out.push(value as u8),
        24..=0xff => {
            out.push(0x18);
            out.push(value as u8);
        }
        0x100..=0xffff => {
            out.push(0x19);
            out.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(0x1a);
            out.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            out.push(0x1b);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn cbor_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    cbor_major_len(out, 2, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

fn cbor_text(out: &mut Vec<u8>, value: &str) {
    cbor_major_len(out, 3, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

fn cbor_service_classes(out: &mut Vec<u8>, values: &[String]) {
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| {
        let mut left_encoded = Vec::new();
        let mut right_encoded = Vec::new();
        cbor_text(&mut left_encoded, left);
        cbor_text(&mut right_encoded, right);
        left_encoded.cmp(&right_encoded)
    });
    ordered.dedup();

    cbor_major_len(out, 4, ordered.len() as u64);
    for value in ordered {
        cbor_text(out, &value);
    }
}

fn cbor_map_len(out: &mut Vec<u8>, len: u64) {
    cbor_major_len(out, 5, len);
}

fn cbor_major_len(out: &mut Vec<u8>, major: u8, value: u64) {
    let prefix = major << 5;
    match value {
        0..=23 => out.push(prefix | value as u8),
        24..=0xff => {
            out.push(prefix | 24);
            out.push(value as u8);
        }
        0x100..=0xffff => {
            out.push(prefix | 25);
            out.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(prefix | 26);
            out.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            out.push(prefix | 27);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn encode_service_epoch_payload(
    report: &ServiceEpochReportV1,
    include_report_id: bool,
    include_signature: bool,
) -> Vec<u8> {
    let mut fields = 16_u64;
    if include_report_id {
        fields += 1;
    }
    if include_signature {
        fields += 1;
    }

    let mut out = Vec::new();
    cbor_map_len(&mut out, fields);

    cbor_uint(&mut out, 1);
    cbor_uint(&mut out, SERVICE_EPOCH_REPORT_SCHEMA_VERSION);

    if include_report_id {
        cbor_uint(&mut out, 2);
        cbor_bytes(&mut out, &report.report_id);
    }

    cbor_uint(&mut out, 3);
    cbor_bytes(&mut out, &report.service_node_id);
    cbor_uint(&mut out, 4);
    cbor_bytes(&mut out, &report.operator_id);
    cbor_uint(&mut out, 5);
    cbor_uint(&mut out, report.epoch_start_height);
    cbor_uint(&mut out, 6);
    cbor_uint(&mut out, report.epoch_end_height);
    cbor_uint(&mut out, 7);
    cbor_uint(&mut out, report.commitments_sampled);
    cbor_uint(&mut out, 8);
    cbor_uint(&mut out, report.challenges_passed);
    cbor_uint(&mut out, 9);
    cbor_uint(&mut out, report.challenges_failed);
    cbor_uint(&mut out, 10);
    cbor_uint(&mut out, report.deadlines_missed);
    cbor_uint(&mut out, 11);
    cbor_uint(&mut out, report.verified_bytes_served);
    cbor_uint(&mut out, 12);
    cbor_uint(&mut out, report.distinct_requester_count);
    cbor_uint(&mut out, 13);
    cbor_uint(&mut out, report.distinct_challenge_block_count);
    cbor_uint(&mut out, 14);
    cbor_service_classes(&mut out, &report.service_classes);
    cbor_uint(&mut out, 15);
    cbor_bytes(&mut out, &report.evidence_root);
    cbor_uint(&mut out, 16);
    cbor_uint(&mut out, report.eligibility_weight);
    cbor_uint(&mut out, 17);
    cbor_bytes(&mut out, &report.created_block);

    if include_signature {
        cbor_uint(&mut out, 18);
        cbor_bytes(&mut out, &report.signature);
    }

    out
}

fn encode_nce_envelope(object_type: u64, schema_version: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    cbor_map_len(&mut out, 4);
    cbor_uint(&mut out, 1);
    cbor_uint(&mut out, 1);
    cbor_uint(&mut out, 2);
    cbor_uint(&mut out, object_type);
    cbor_uint(&mut out, 3);
    cbor_uint(&mut out, schema_version);
    cbor_uint(&mut out, 4);
    out.extend_from_slice(payload);
    out
}

pub fn service_epoch_report_id_preimage(report: &ServiceEpochReportV1) -> Vec<u8> {
    encode_nce_envelope(
        SERVICE_EPOCH_REPORT_OBJECT_TYPE,
        SERVICE_EPOCH_REPORT_SCHEMA_VERSION,
        &encode_service_epoch_payload(report, false, false),
    )
}

pub fn service_epoch_report_signing_preimage(report: &ServiceEpochReportV1) -> Vec<u8> {
    encode_nce_envelope(
        SERVICE_EPOCH_REPORT_OBJECT_TYPE,
        SERVICE_EPOCH_REPORT_SCHEMA_VERSION,
        &encode_service_epoch_payload(report, true, false),
    )
}

pub fn service_epoch_report_canonical_bytes(report: &ServiceEpochReportV1) -> Vec<u8> {
    encode_nce_envelope(
        SERVICE_EPOCH_REPORT_OBJECT_TYPE,
        SERVICE_EPOCH_REPORT_SCHEMA_VERSION,
        &encode_service_epoch_payload(report, true, true),
    )
}

fn generic_protocol_digest(purpose: &[u8], network_id: &[u8], canonical_bytes: &[u8]) -> Hash32 {
    let mut preimage = Vec::with_capacity(
        b"NIAHCIA".len() + 1 + purpose.len() + 1 + network_id.len() + 1 + canonical_bytes.len(),
    );
    preimage.extend_from_slice(b"NIAHCIA");
    preimage.push(0);
    preimage.extend_from_slice(purpose);
    preimage.push(0);
    preimage.extend_from_slice(network_id);
    preimage.push(0);
    preimage.extend_from_slice(canonical_bytes);
    keccak256(&preimage)
}

pub fn derive_service_epoch_report_id(network_id: &[u8], report: &ServiceEpochReportV1) -> Hash32 {
    generic_protocol_digest(
        b"ID/SERVICE_EPOCH_REPORT",
        network_id,
        &service_epoch_report_id_preimage(report),
    )
}

pub fn service_epoch_report_signing_digest(
    network_id: &[u8],
    report: &ServiceEpochReportV1,
) -> Hash32 {
    generic_protocol_digest(
        b"SIGN/SERVICE_EPOCH_REPORT",
        network_id,
        &service_epoch_report_signing_preimage(report),
    )
}

pub fn build_service_epoch_report_v1(
    network_id: &[u8],
    finalized: &FinalizedServiceEpochReport,
    operator_id: Hash32,
    commitments_sampled: u64,
    service_classes: Vec<String>,
    created_block: Hash32,
) -> ServiceEpochReportV1 {
    let mut report = ServiceEpochReportV1 {
        report_id: [0_u8; 32],
        service_node_id: finalized.service_node_id,
        operator_id,
        epoch_start_height: finalized.epoch_start_height,
        epoch_end_height: finalized.epoch_end_height,
        commitments_sampled,
        challenges_passed: finalized.challenges_passed,
        challenges_failed: finalized.challenges_failed,
        deadlines_missed: finalized.deadlines_missed,
        verified_bytes_served: finalized.verified_bytes_served,
        distinct_requester_count: finalized.distinct_requester_count,
        distinct_challenge_block_count: finalized.distinct_challenge_block_count,
        service_classes,
        evidence_root: finalized.evidence_root,
        eligibility_weight: finalized.eligibility_weight,
        created_block,
        signature: Vec::new(),
    };

    report.report_id = derive_service_epoch_report_id(network_id, &report);
    report
}

pub fn derive_service_node_id(public_key_sec1: &[u8]) -> Result<Hash32, String> {
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|_| "invalid secp256k1 service public key".to_string())?;
    let compressed = verifying_key.to_encoded_point(true);
    let mut preimage =
        Vec::with_capacity(SERVICE_NODE_ID_DOMAIN.len() + compressed.as_bytes().len());
    preimage.extend_from_slice(SERVICE_NODE_ID_DOMAIN);
    preimage.extend_from_slice(compressed.as_bytes());
    Ok(keccak256(&preimage))
}

pub fn verify_service_node_identity(
    service_node_id: Hash32,
    public_key_sec1: &[u8],
) -> Result<(), String> {
    let derived = derive_service_node_id(public_key_sec1)?;
    if derived != service_node_id {
        return Err("service_node_id does not match secp256k1 public key".into());
    }
    Ok(())
}

pub fn sign_service_epoch_report(
    network_id: &[u8],
    report: &mut ServiceEpochReportV1,
    secret_key: &[u8; 32],
) -> Result<(), String> {
    let signing_key = SigningKey::from_slice(secret_key)
        .map_err(|_| "invalid secp256k1 service signing key".to_string())?;
    let public_key = signing_key.verifying_key().to_encoded_point(true);
    verify_service_node_identity(report.service_node_id, public_key.as_bytes())?;

    let expected_id = derive_service_epoch_report_id(network_id, report);
    if report.report_id != expected_id {
        return Err("service epoch report_id does not match report contents".into());
    }

    let digest = service_epoch_report_signing_digest(network_id, report);
    let signature: Signature = signing_key
        .sign_prehash(&digest)
        .map_err(|_| "failed to sign service epoch report".to_string())?;

    report.signature = signature.to_bytes().to_vec();
    Ok(())
}

pub fn verify_service_epoch_report_signature(
    network_id: &[u8],
    report: &ServiceEpochReportV1,
    public_key_sec1: &[u8],
) -> Result<(), String> {
    verify_service_node_identity(report.service_node_id, public_key_sec1)?;

    let expected_id = derive_service_epoch_report_id(network_id, report);
    if report.report_id != expected_id {
        return Err("service epoch report_id does not match report contents".into());
    }

    if report.signature.len() != 64 {
        return Err("service epoch report signature must be 64-byte compact ECDSA".into());
    }

    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|_| "invalid secp256k1 service public key".to_string())?;
    let signature = Signature::from_slice(&report.signature)
        .map_err(|_| "invalid secp256k1 service signature".to_string())?;
    let digest = service_epoch_report_signing_digest(network_id, report);

    verifying_key
        .verify_prehash(&digest, &signature)
        .map_err(|_| "invalid service epoch report signature".to_string())
}

pub fn attach_service_epoch_signature(
    report: &mut ServiceEpochReportV1,
    signature: Vec<u8>,
) -> Result<(), String> {
    if signature.is_empty() {
        return Err("service epoch report signature must not be empty".into());
    }

    report.signature = signature;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        attach_service_epoch_signature, build_service_epoch_report_v1, build_storage_challenge_v1,
        build_storage_commitment_v1, build_storage_response_v1, derive_service_epoch_report_id,
        derive_service_node_id, derive_storage_challenge_id, derive_storage_commitment_id,
        derive_storage_response_id, evaluate_service_eligibility, evidence_replay_key,
        finalize_service_epoch_report, load_persisted_service_epoch,
        record_verified_storage_response, record_verified_storage_response_persistent,
        select_storage_ranges, select_storage_segments, service_epoch_report_canonical_bytes,
        service_epoch_report_signing_digest, service_epoch_report_signing_preimage,
        service_evidence_root, sign_service_epoch_report, sign_storage_challenge,
        sign_storage_commitment, sign_storage_response, storage_challenge_canonical_bytes,
        storage_challenge_id_preimage, storage_challenge_seed, storage_challenge_signing_digest,
        storage_challenge_signing_preimage, storage_commitment_canonical_bytes,
        storage_commitment_id_preimage, storage_commitment_signing_digest,
        storage_commitment_signing_preimage, storage_manifest_leaf, storage_manifest_node,
        storage_manifest_root, storage_range_leaf, storage_range_node, storage_range_root,
        storage_response_bytes_hash, storage_response_canonical_bytes,
        storage_response_id_preimage, storage_response_signing_digest,
        storage_response_signing_preimage, verify_response_meta,
        verify_service_epoch_report_signature, verify_storage_challenge_signature,
        verify_storage_commitment_signature, verify_storage_manifest_proof,
        verify_storage_range_proof, verify_storage_response_evidence,
        verify_storage_response_signature, ChallengeSegment, ExpectedResponseMeta,
        ManifestProofStep, PersistentStorageResponseContext, RangeProofStep, ResponseMeta,
        ServiceEpochAccumulator, ServiceEpochReportV1, StorageChallengeParams,
        StorageCommitmentParams, StorageRangeProofV1, StorageResponseParams,
    };

    #[test]
    fn storage_commitment_is_canonical_signed_and_network_bound() {
        let secret = [0x09_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        let service_node_id = derive_service_node_id(public_key.as_bytes()).unwrap();

        let mut commitment = build_storage_commitment_v1(
            b"niahcia-dev",
            StorageCommitmentParams {
                service_node_id,
                operator_id: [0xbb; 32],
                service_class: "MODEL_STORAGE".into(),
                object_id: [0x44; 32],
                manifest_root: [0x55; 32],
                chunk_count: 4,
                total_bytes: 16_384,
                retained_from_block: 100,
                retained_until_block: 820,
                commitment_nonce: 7,
                created_block: [0x66; 32],
            },
        )
        .unwrap();

        assert_eq!(
            commitment.commitment_id,
            derive_storage_commitment_id(b"niahcia-dev", &commitment)
        );
        sign_storage_commitment(b"niahcia-dev", &mut commitment, &secret).unwrap();
        assert_eq!(commitment.signature.len(), 64);
        verify_storage_commitment_signature(b"niahcia-dev", &commitment, public_key.as_bytes())
            .unwrap();

        let mut tampered = commitment.clone();
        tampered.total_bytes += 1;
        assert!(verify_storage_commitment_signature(
            b"niahcia-dev",
            &tampered,
            public_key.as_bytes(),
        )
        .is_err());
        assert!(verify_storage_commitment_signature(
            b"other-network",
            &commitment,
            public_key.as_bytes(),
        )
        .is_err());
    }

    #[test]
    fn storage_commitment_rejects_invalid_retention_and_empty_content() {
        let node_id = [0x11; 32];
        assert!(build_storage_commitment_v1(
            b"niahcia-dev",
            StorageCommitmentParams {
                service_node_id: node_id,
                operator_id: [0x22; 32],
                service_class: "MODEL_STORAGE".into(),
                object_id: [0x33; 32],
                manifest_root: [0x44; 32],
                chunk_count: 0,
                total_bytes: 1,
                retained_from_block: 0,
                retained_until_block: 720,
                commitment_nonce: 0,
                created_block: [0x55; 32],
            },
        )
        .is_err());
        assert!(build_storage_commitment_v1(
            b"niahcia-dev",
            StorageCommitmentParams {
                service_node_id: node_id,
                operator_id: [0x22; 32],
                service_class: "MODEL_STORAGE".into(),
                object_id: [0x33; 32],
                manifest_root: [0x44; 32],
                chunk_count: 1,
                total_bytes: 0,
                retained_from_block: 0,
                retained_until_block: 720,
                commitment_nonce: 0,
                created_block: [0x55; 32],
            },
        )
        .is_err());
        assert!(build_storage_commitment_v1(
            b"niahcia-dev",
            StorageCommitmentParams {
                service_node_id: node_id,
                operator_id: [0x22; 32],
                service_class: "MODEL_STORAGE".into(),
                object_id: [0x33; 32],
                manifest_root: [0x44; 32],
                chunk_count: 1,
                total_bytes: 1,
                retained_from_block: 720,
                retained_until_block: 720,
                commitment_nonce: 0,
                created_block: [0x55; 32],
            },
        )
        .is_err());
    }
    #[test]
    fn persistent_verified_response_survives_restart_and_rebuilds_epoch() {
        use crate::state::StateStore;
        use std::time::{SystemTime, UNIX_EPOCH};

        let challenger_secret = [0x0a_u8; 32];
        let challenger_key = k256::ecdsa::SigningKey::from_slice(&challenger_secret).unwrap();
        let challenger_public = challenger_key.verifying_key().to_encoded_point(true);
        let challenger_id = derive_service_node_id(challenger_public.as_bytes()).unwrap();

        let provider_secret = [0x0b_u8; 32];
        let provider_key = k256::ecdsa::SigningKey::from_slice(&provider_secret).unwrap();
        let provider_public = provider_key.verifying_key().to_encoded_point(true);
        let service_node_id = derive_service_node_id(provider_public.as_bytes()).unwrap();

        let returned_bytes = b"abcdefgh".to_vec();
        let range_root = storage_range_leaf(0, &returned_bytes);
        let chunk_hash = [0x70; 32];
        let manifest_root = storage_manifest_leaf(0, 8, chunk_hash, range_root);

        let mut challenge = build_storage_challenge_v1(
            b"niahcia-dev",
            StorageChallengeParams {
                commitment_id: [0x31; 32],
                challenge_block_id: [0x42; 32],
                challenge_height: 100,
                requested_ranges: vec![ChallengeSegment {
                    chunk_index: 0,
                    segment_index: 0,
                    offset: 0,
                    length: 8,
                }],
                issued_at: 1000,
                response_deadline: 1030,
                challenger_id,
            },
        )
        .unwrap();
        sign_storage_challenge(b"niahcia-dev", &mut challenge, &challenger_secret).unwrap();

        let mut response = build_storage_response_v1(
            b"niahcia-dev",
            StorageResponseParams {
                challenge_id: challenge.challenge_id,
                commitment_id: challenge.commitment_id,
                service_node_id,
                answered_at: 1020,
                range_proofs: vec![StorageRangeProofV1 {
                    chunk_index: 0,
                    segment_index: 0,
                    offset: 0,
                    length: 8,
                    returned_bytes,
                    chunk_length: 8,
                    chunk_hash,
                    range_root,
                    range_proof: Vec::new(),
                    manifest_proof: Vec::new(),
                }],
            },
        )
        .unwrap();
        sign_storage_response(b"niahcia-dev", &mut response, &provider_secret).unwrap();

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "niahcia-service-epoch-{}-{nonce}.redb",
            std::process::id()
        ));

        {
            let store = StateStore::open(&path).unwrap();
            let mut epoch = ServiceEpochAccumulator::new(0, 720).unwrap();
            record_verified_storage_response_persistent(
                &mut epoch,
                &challenge,
                &response,
                PersistentStorageResponseContext {
                    store: &store,
                    network_id: b"niahcia-dev",
                    challenger_public_key_sec1: challenger_public.as_bytes(),
                    provider_public_key_sec1: provider_public.as_bytes(),
                    manifest_root,
                },
            )
            .unwrap();
            assert_eq!(epoch.challenges_passed, 1);
            assert_eq!(epoch.verified_bytes_served, 8);
        }

        {
            let store = StateStore::open(&path).unwrap();
            let mut restored = load_persisted_service_epoch(&store, 0, 720).unwrap();
            assert_eq!(restored.challenges_passed, 1);
            assert_eq!(restored.verified_bytes_served, 8);
            assert_eq!(restored.distinct_requester_count(), 1);
            assert_eq!(restored.distinct_challenge_block_count(), 1);
            assert!(record_verified_storage_response_persistent(
                &mut restored,
                &challenge,
                &response,
                PersistentStorageResponseContext {
                    store: &store,
                    network_id: b"niahcia-dev",
                    challenger_public_key_sec1: challenger_public.as_bytes(),
                    provider_public_key_sec1: provider_public.as_bytes(),
                    manifest_root,
                },
            )
            .is_err());
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn verified_storage_response_records_once_in_epoch() {
        let challenger_secret = [0x0a_u8; 32];
        let challenger_key = k256::ecdsa::SigningKey::from_slice(&challenger_secret).unwrap();
        let challenger_public = challenger_key.verifying_key().to_encoded_point(true);
        let challenger_id = derive_service_node_id(challenger_public.as_bytes()).unwrap();

        let provider_secret = [0x0b_u8; 32];
        let provider_key = k256::ecdsa::SigningKey::from_slice(&provider_secret).unwrap();
        let provider_public = provider_key.verifying_key().to_encoded_point(true);
        let service_node_id = derive_service_node_id(provider_public.as_bytes()).unwrap();

        let returned_bytes = b"abcdefgh".to_vec();
        let range_root = storage_range_leaf(0, &returned_bytes);
        let chunk_hash = [0x70; 32];
        let manifest_root = storage_manifest_leaf(0, 8, chunk_hash, range_root);

        let mut challenge = build_storage_challenge_v1(
            b"niahcia-dev",
            StorageChallengeParams {
                commitment_id: [0x31; 32],
                challenge_block_id: [0x42; 32],
                challenge_height: 100,
                requested_ranges: vec![ChallengeSegment {
                    chunk_index: 0,
                    segment_index: 0,
                    offset: 0,
                    length: 8,
                }],
                issued_at: 1000,
                response_deadline: 1030,
                challenger_id,
            },
        )
        .unwrap();
        sign_storage_challenge(b"niahcia-dev", &mut challenge, &challenger_secret).unwrap();

        let mut response = build_storage_response_v1(
            b"niahcia-dev",
            StorageResponseParams {
                challenge_id: challenge.challenge_id,
                commitment_id: challenge.commitment_id,
                service_node_id,
                answered_at: 1020,
                range_proofs: vec![StorageRangeProofV1 {
                    chunk_index: 0,
                    segment_index: 0,
                    offset: 0,
                    length: 8,
                    returned_bytes,
                    chunk_length: 8,
                    chunk_hash,
                    range_root,
                    range_proof: Vec::new(),
                    manifest_proof: Vec::new(),
                }],
            },
        )
        .unwrap();
        sign_storage_response(b"niahcia-dev", &mut response, &provider_secret).unwrap();

        let mut epoch = ServiceEpochAccumulator::new(0, 720).unwrap();
        let evidence_key = record_verified_storage_response(
            &mut epoch,
            b"niahcia-dev",
            &challenge,
            challenger_public.as_bytes(),
            &response,
            provider_public.as_bytes(),
            manifest_root,
        )
        .unwrap();

        assert_eq!(
            evidence_key,
            evidence_replay_key(
                challenge.challenge_id,
                response.service_node_id,
                response.response_id
            )
        );
        assert_eq!(epoch.challenges_passed, 1);
        assert_eq!(epoch.verified_bytes_served, 8);
        assert_eq!(epoch.distinct_requester_count(), 1);
        assert_eq!(epoch.distinct_challenge_block_count(), 1);
        assert!(record_verified_storage_response(
            &mut epoch,
            b"niahcia-dev",
            &challenge,
            challenger_public.as_bytes(),
            &response,
            provider_public.as_bytes(),
            manifest_root,
        )
        .is_err());
    }

    #[test]
    fn locked_storage_response_vector_matches() {
        let secret = [0x0b_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        assert_eq!(
            hex::encode(public_key.as_bytes()),
            "02552c630b64b54bf50210c9e253d38bd4949c72e22873500f6285c2bede312a84"
        );

        let service_node_id = derive_service_node_id(public_key.as_bytes()).unwrap();
        assert_eq!(
            hex::encode(service_node_id),
            "2c76f8506cecbb516a45739348d43314356ef2361078aa6359ee980ce8535bbc"
        );

        let returned_bytes = b"abcdefgh".to_vec();
        let range_root = storage_range_leaf(0, &returned_bytes);
        assert_eq!(
            hex::encode(range_root),
            "2dd606a3ac32e28b80868f0f738e3cc5bdd3725e6f4262c2dae150a72b0fe915"
        );

        let range_proof = StorageRangeProofV1 {
            chunk_index: 0,
            segment_index: 0,
            offset: 0,
            length: 8,
            returned_bytes,
            chunk_length: 8,
            chunk_hash: [0x70; 32],
            range_root,
            range_proof: Vec::new(),
            manifest_proof: Vec::new(),
        };

        assert_eq!(
            hex::encode(storage_response_bytes_hash(std::slice::from_ref(
                &range_proof
            ))),
            "9989bfe991cba8260667c0142441ceeb95d9c4b5873578bd3226ca716e326f7b"
        );

        let mut response = build_storage_response_v1(
            b"niahcia-dev",
            StorageResponseParams {
                challenge_id: hex::decode(
                    "5e623e834459f2f73f8a324fdf35c003c9dbf79322dbd68a1e1b56ea25974763",
                )
                .unwrap()
                .try_into()
                .unwrap(),
                commitment_id: hex::decode(
                    "bdc05540f553573acc7b89d9888e4291ecf3953d620959d14eccafae64d9cbfa",
                )
                .unwrap()
                .try_into()
                .unwrap(),
                service_node_id,
                answered_at: 1_000_020,
                range_proofs: vec![range_proof],
            },
        )
        .unwrap();

        assert_eq!(
            hex::encode(storage_response_id_preimage(&response)),
            "a4010102190207030104a701010358205e623e834459f2f73f8a324fdf35c003c9dbf79322dbd68a1e1b56ea25974763045820bdc05540f553573acc7b89d9888e4291ecf3953d620959d14eccafae64d9cbfa0558202c76f8506cecbb516a45739348d43314356ef2361078aa6359ee980ce8535bbc061a000f425407818a00000008486162636465666768085820707070707070707070707070707070707070707070707070707070707070707058202dd606a3ac32e28b80868f0f738e3cc5bdd3725e6f4262c2dae150a72b0fe91580800858209989bfe991cba8260667c0142441ceeb95d9c4b5873578bd3226ca716e326f7b"
        );
        assert_eq!(
            hex::encode(response.response_id),
            "f8b0b62a72e22b67358b6ae605c1576bdb9d62ba760be997856302b1a759aa8d"
        );
        assert_eq!(
            hex::encode(storage_response_signing_preimage(&response)),
            "a4010102190207030104a80101025820f8b0b62a72e22b67358b6ae605c1576bdb9d62ba760be997856302b1a759aa8d0358205e623e834459f2f73f8a324fdf35c003c9dbf79322dbd68a1e1b56ea25974763045820bdc05540f553573acc7b89d9888e4291ecf3953d620959d14eccafae64d9cbfa0558202c76f8506cecbb516a45739348d43314356ef2361078aa6359ee980ce8535bbc061a000f425407818a00000008486162636465666768085820707070707070707070707070707070707070707070707070707070707070707058202dd606a3ac32e28b80868f0f738e3cc5bdd3725e6f4262c2dae150a72b0fe91580800858209989bfe991cba8260667c0142441ceeb95d9c4b5873578bd3226ca716e326f7b"
        );
        assert_eq!(
            hex::encode(storage_response_signing_digest(b"niahcia-dev", &response)),
            "beb001fb4d35c564d6d92829917d69aa69a94bb5fc022d0bde98e1d61e983f25"
        );

        sign_storage_response(b"niahcia-dev", &mut response, &secret).unwrap();
        assert_eq!(
            hex::encode(&response.signature),
            "12de1a44a77590fe4eb87f65dcc7d6128866c0ff8b91335678278fef4fead0436625f25f8411ac4c3d4f9ee9d78a3fa0fcd54b535f3c777a86faa6acd4993cf3"
        );
        assert_eq!(
            hex::encode(storage_response_canonical_bytes(&response)),
            "a4010102190207030104a90101025820f8b0b62a72e22b67358b6ae605c1576bdb9d62ba760be997856302b1a759aa8d0358205e623e834459f2f73f8a324fdf35c003c9dbf79322dbd68a1e1b56ea25974763045820bdc05540f553573acc7b89d9888e4291ecf3953d620959d14eccafae64d9cbfa0558202c76f8506cecbb516a45739348d43314356ef2361078aa6359ee980ce8535bbc061a000f425407818a00000008486162636465666768085820707070707070707070707070707070707070707070707070707070707070707058202dd606a3ac32e28b80868f0f738e3cc5bdd3725e6f4262c2dae150a72b0fe91580800858209989bfe991cba8260667c0142441ceeb95d9c4b5873578bd3226ca716e326f7b09584012de1a44a77590fe4eb87f65dcc7d6128866c0ff8b91335678278fef4fead0436625f25f8411ac4c3d4f9ee9d78a3fa0fcd54b535f3c777a86faa6acd4993cf3"
        );
        verify_storage_response_signature(b"niahcia-dev", &response, public_key.as_bytes())
            .unwrap();
    }

    #[test]
    fn storage_response_is_signed_network_bound_and_verifies_evidence() {
        let provider_secret = [0x0b_u8; 32];
        let provider_key = k256::ecdsa::SigningKey::from_slice(&provider_secret).unwrap();
        let provider_public = provider_key.verifying_key().to_encoded_point(true);
        let service_node_id = derive_service_node_id(provider_public.as_bytes()).unwrap();

        let segment0 = b"abcdefgh";
        let segment1 = b"ijklmnop";
        let leaf0 = storage_range_leaf(0, segment0);
        let leaf1 = storage_range_leaf(1, segment1);
        let range_root = storage_range_node(leaf0, leaf1);
        let chunk_hash = [0x70; 32];
        let manifest_root = storage_manifest_leaf(0, 16, chunk_hash, range_root);

        let challenge = build_storage_challenge_v1(
            b"niahcia-dev",
            StorageChallengeParams {
                commitment_id: [0x31; 32],
                challenge_block_id: [0x42; 32],
                challenge_height: 900,
                requested_ranges: vec![ChallengeSegment {
                    chunk_index: 0,
                    segment_index: 1,
                    offset: 8,
                    length: 8,
                }],
                issued_at: 100,
                response_deadline: 130,
                challenger_id: [0x53; 32],
            },
        )
        .unwrap();

        let mut response = build_storage_response_v1(
            b"niahcia-dev",
            StorageResponseParams {
                challenge_id: challenge.challenge_id,
                commitment_id: challenge.commitment_id,
                service_node_id,
                answered_at: 120,
                range_proofs: vec![StorageRangeProofV1 {
                    chunk_index: 0,
                    segment_index: 1,
                    offset: 8,
                    length: 8,
                    returned_bytes: segment1.to_vec(),
                    chunk_length: 16,
                    chunk_hash,
                    range_root,
                    range_proof: vec![RangeProofStep {
                        sibling: leaf0,
                        sibling_is_left: true,
                    }],
                    manifest_proof: Vec::new(),
                }],
            },
        )
        .unwrap();

        assert_eq!(
            response.response_id,
            derive_storage_response_id(b"niahcia-dev", &response)
        );
        sign_storage_response(b"niahcia-dev", &mut response, &provider_secret).unwrap();
        verify_storage_response_signature(b"niahcia-dev", &response, provider_public.as_bytes())
            .unwrap();
        assert_eq!(
            verify_storage_response_evidence(&response, &challenge, manifest_root).unwrap(),
            8
        );

        let mut tampered = response.clone();
        tampered.range_proofs[0].returned_bytes[0] ^= 1;
        assert!(verify_storage_response_signature(
            b"niahcia-dev",
            &tampered,
            provider_public.as_bytes(),
        )
        .is_err());
        assert!(verify_storage_response_signature(
            b"other-network",
            &response,
            provider_public.as_bytes(),
        )
        .is_err());
    }

    #[test]
    fn storage_response_rejects_late_and_wrong_segment_evidence() {
        let segment = b"abcdefgh";
        let range_root = storage_range_leaf(0, segment);
        let chunk_hash = [0x70; 32];
        let manifest_root = storage_manifest_leaf(0, 8, chunk_hash, range_root);
        let challenge = build_storage_challenge_v1(
            b"niahcia-dev",
            StorageChallengeParams {
                commitment_id: [0x31; 32],
                challenge_block_id: [0x42; 32],
                challenge_height: 900,
                requested_ranges: vec![ChallengeSegment {
                    chunk_index: 0,
                    segment_index: 0,
                    offset: 0,
                    length: 8,
                }],
                issued_at: 100,
                response_deadline: 130,
                challenger_id: [0x53; 32],
            },
        )
        .unwrap();

        let mut response = build_storage_response_v1(
            b"niahcia-dev",
            StorageResponseParams {
                challenge_id: challenge.challenge_id,
                commitment_id: challenge.commitment_id,
                service_node_id: [0x60; 32],
                answered_at: 131,
                range_proofs: vec![StorageRangeProofV1 {
                    chunk_index: 0,
                    segment_index: 0,
                    offset: 0,
                    length: 8,
                    returned_bytes: segment.to_vec(),
                    chunk_length: 8,
                    chunk_hash,
                    range_root,
                    range_proof: Vec::new(),
                    manifest_proof: Vec::new(),
                }],
            },
        )
        .unwrap();

        assert!(verify_storage_response_evidence(&response, &challenge, manifest_root).is_err());
        response.answered_at = 120;
        response.range_proofs[0].offset = 1;
        assert!(verify_storage_response_evidence(&response, &challenge, manifest_root).is_err());
    }

    #[test]
    fn locked_storage_challenge_vector_matches() {
        let secret = [0x0a_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        assert_eq!(
            hex::encode(public_key.as_bytes()),
            "03f76a39d05686e34a4420897e359371836145dd3973e3982568b60f8433adde6e"
        );
        let challenger_id = derive_service_node_id(public_key.as_bytes()).unwrap();
        assert_eq!(
            hex::encode(challenger_id),
            "a53e0f71a08bab3aa14045adc7a535c61b2bc4c87af311547ce620f7fba5f283"
        );

        let mut challenge = build_storage_challenge_v1(
            b"niahcia-dev",
            StorageChallengeParams {
                commitment_id: [0x31; 32],
                challenge_block_id: [0x42; 32],
                challenge_height: 900,
                requested_ranges: vec![
                    ChallengeSegment {
                        chunk_index: 2,
                        segment_index: 3,
                        offset: 12_288,
                        length: 4096,
                    },
                    ChallengeSegment {
                        chunk_index: 5,
                        segment_index: 0,
                        offset: 0,
                        length: 1024,
                    },
                ],
                issued_at: 1_000_000,
                response_deadline: 1_000_030,
                challenger_id,
            },
        )
        .unwrap();

        assert_eq!(
            hex::encode(challenge.challenge_seed),
            "a772492e1fae81dd7610230ae2d288eed165a807719edf5daffbf8d2d6513313"
        );
        assert_eq!(
            hex::encode(storage_challenge_id_preimage(&challenge)),
            "a4010102190206030104a901010358203131313131313131313131313131313131313131313131313131313131313131045820424242424242424242424242424242424242424242424242424242424242424205190384065820a772492e1fae81dd7610230ae2d288eed165a807719edf5daffbf8d2d6513313078284020319300019100084050000190400081a000f4240091a000f425e0a5820a53e0f71a08bab3aa14045adc7a535c61b2bc4c87af311547ce620f7fba5f283"
        );
        assert_eq!(
            hex::encode(challenge.challenge_id),
            "5e623e834459f2f73f8a324fdf35c003c9dbf79322dbd68a1e1b56ea25974763"
        );
        assert_eq!(
            hex::encode(storage_challenge_signing_preimage(&challenge)),
            "a4010102190206030104aa01010258205e623e834459f2f73f8a324fdf35c003c9dbf79322dbd68a1e1b56ea259747630358203131313131313131313131313131313131313131313131313131313131313131045820424242424242424242424242424242424242424242424242424242424242424205190384065820a772492e1fae81dd7610230ae2d288eed165a807719edf5daffbf8d2d6513313078284020319300019100084050000190400081a000f4240091a000f425e0a5820a53e0f71a08bab3aa14045adc7a535c61b2bc4c87af311547ce620f7fba5f283"
        );
        assert_eq!(
            hex::encode(storage_challenge_signing_digest(b"niahcia-dev", &challenge)),
            "913ae6ddaf54688427d14f1e584e318a823f4873ac8fb8bfe02d5a3f0c0c51e6"
        );

        sign_storage_challenge(b"niahcia-dev", &mut challenge, &secret).unwrap();
        assert_eq!(
            hex::encode(&challenge.signature),
            "fa68cd03c27aa69e3ba6a444079a0f86397dc24e6b83a04ec8ce4c9275704ea074382503bca56323bf6cc4e544e0a38c8f399a73351e704f13fdba87f8041dbd"
        );
        assert_eq!(
            hex::encode(storage_challenge_canonical_bytes(&challenge)),
            "a4010102190206030104ab01010258205e623e834459f2f73f8a324fdf35c003c9dbf79322dbd68a1e1b56ea259747630358203131313131313131313131313131313131313131313131313131313131313131045820424242424242424242424242424242424242424242424242424242424242424205190384065820a772492e1fae81dd7610230ae2d288eed165a807719edf5daffbf8d2d6513313078284020319300019100084050000190400081a000f4240091a000f425e0a5820a53e0f71a08bab3aa14045adc7a535c61b2bc4c87af311547ce620f7fba5f2830b5840fa68cd03c27aa69e3ba6a444079a0f86397dc24e6b83a04ec8ce4c9275704ea074382503bca56323bf6cc4e544e0a38c8f399a73351e704f13fdba87f8041dbd"
        );
        verify_storage_challenge_signature(b"niahcia-dev", &challenge, public_key.as_bytes())
            .unwrap();
    }
    #[test]
    fn storage_challenge_is_canonical_signed_seed_bound_and_network_bound() {
        let secret = [0x0a_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        let challenger_id = derive_service_node_id(public_key.as_bytes()).unwrap();
        let commitment_id = [0x31; 32];
        let challenge_block_id = [0x42; 32];

        let mut challenge = build_storage_challenge_v1(
            b"niahcia-dev",
            StorageChallengeParams {
                commitment_id,
                challenge_block_id,
                challenge_height: 900,
                requested_ranges: vec![
                    ChallengeSegment {
                        chunk_index: 2,
                        segment_index: 3,
                        offset: 12_288,
                        length: 4096,
                    },
                    ChallengeSegment {
                        chunk_index: 5,
                        segment_index: 0,
                        offset: 0,
                        length: 1024,
                    },
                ],
                issued_at: 1_000_000,
                response_deadline: 1_000_030,
                challenger_id,
            },
        )
        .unwrap();

        assert_eq!(
            challenge.challenge_seed,
            storage_challenge_seed(challenge_block_id, commitment_id)
        );
        assert_eq!(
            challenge.challenge_id,
            derive_storage_challenge_id(b"niahcia-dev", &challenge)
        );
        sign_storage_challenge(b"niahcia-dev", &mut challenge, &secret).unwrap();
        verify_storage_challenge_signature(b"niahcia-dev", &challenge, public_key.as_bytes())
            .unwrap();

        let mut bad_seed = challenge.clone();
        bad_seed.challenge_seed[0] ^= 1;
        assert!(verify_storage_challenge_signature(
            b"niahcia-dev",
            &bad_seed,
            public_key.as_bytes(),
        )
        .is_err());
        assert!(verify_storage_challenge_signature(
            b"other-network",
            &challenge,
            public_key.as_bytes(),
        )
        .is_err());
    }

    #[test]
    fn storage_challenge_rejects_empty_ranges_and_bad_deadline() {
        let base = StorageChallengeParams {
            commitment_id: [0x31; 32],
            challenge_block_id: [0x42; 32],
            challenge_height: 900,
            requested_ranges: Vec::new(),
            issued_at: 100,
            response_deadline: 130,
            challenger_id: [0x53; 32],
        };
        assert!(build_storage_challenge_v1(b"niahcia-dev", base.clone()).is_err());

        let mut bad_deadline = base;
        bad_deadline.requested_ranges.push(ChallengeSegment {
            chunk_index: 0,
            segment_index: 0,
            offset: 0,
            length: 1024,
        });
        bad_deadline.response_deadline = bad_deadline.issued_at;
        assert!(build_storage_challenge_v1(b"niahcia-dev", bad_deadline).is_err());
    }
    #[test]
    fn locked_storage_commitment_vector_matches() {
        let secret = [0x09_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        assert_eq!(
            hex::encode(public_key.as_bytes()),
            "0256b328b30c8bf5839e24058747879408bdb36241dc9c2e7c619faa12b2920967"
        );

        let service_node_id = derive_service_node_id(public_key.as_bytes()).unwrap();
        assert_eq!(
            hex::encode(service_node_id),
            "514f4f5fd2c5331dd8a69a46bb4996f3ad8c418bb9e53af973da50ea68e81332"
        );

        let mut commitment = build_storage_commitment_v1(
            b"niahcia-dev",
            StorageCommitmentParams {
                service_node_id,
                operator_id: [0xbb; 32],
                service_class: "MODEL_STORAGE".into(),
                object_id: [0x44; 32],
                manifest_root: [0x55; 32],
                chunk_count: 4,
                total_bytes: 16_384,
                retained_from_block: 100,
                retained_until_block: 820,
                commitment_nonce: 7,
                created_block: [0x66; 32],
            },
        )
        .unwrap();

        assert_eq!(
            hex::encode(storage_commitment_id_preimage(&commitment)),
            "a4010102190205030104ac0101035820514f4f5fd2c5331dd8a69a46bb4996f3ad8c418bb9e53af973da50ea68e81332045820bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb056d4d4f44454c5f53544f52414745065820444444444444444444444444444444444444444444444444444444444444444407582055555555555555555555555555555555555555555555555555555555555555550804091940000a18640b1903340c070d58206666666666666666666666666666666666666666666666666666666666666666"
        );
        assert_eq!(
            hex::encode(commitment.commitment_id),
            "bdc05540f553573acc7b89d9888e4291ecf3953d620959d14eccafae64d9cbfa"
        );
        assert_eq!(
            hex::encode(storage_commitment_signing_preimage(&commitment)),
            "a4010102190205030104ad0101025820bdc05540f553573acc7b89d9888e4291ecf3953d620959d14eccafae64d9cbfa035820514f4f5fd2c5331dd8a69a46bb4996f3ad8c418bb9e53af973da50ea68e81332045820bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb056d4d4f44454c5f53544f52414745065820444444444444444444444444444444444444444444444444444444444444444407582055555555555555555555555555555555555555555555555555555555555555550804091940000a18640b1903340c070d58206666666666666666666666666666666666666666666666666666666666666666"
        );
        assert_eq!(
            hex::encode(storage_commitment_signing_digest(
                b"niahcia-dev",
                &commitment
            )),
            "1375af6584f574d354dac170543bdc09d90cf1392672eec18e17cb7a7c870227"
        );

        sign_storage_commitment(b"niahcia-dev", &mut commitment, &secret).unwrap();
        assert_eq!(
            hex::encode(&commitment.signature),
            "4ef89fa55521225507f3942e4fb4ae61391fd32050f29ba3e93f369654f0b4bf39659a691e9afcd0954a5979c77fd707a55fae71136d5056416ba7ff6840bfbd"
        );
        assert_eq!(
            hex::encode(storage_commitment_canonical_bytes(&commitment)),
            "a4010102190205030104ae0101025820bdc05540f553573acc7b89d9888e4291ecf3953d620959d14eccafae64d9cbfa035820514f4f5fd2c5331dd8a69a46bb4996f3ad8c418bb9e53af973da50ea68e81332045820bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb056d4d4f44454c5f53544f52414745065820444444444444444444444444444444444444444444444444444444444444444407582055555555555555555555555555555555555555555555555555555555555555550804091940000a18640b1903340c070d582066666666666666666666666666666666666666666666666666666666666666660e58404ef89fa55521225507f3942e4fb4ae61391fd32050f29ba3e93f369654f0b4bf39659a691e9afcd0954a5979c77fd707a55fae71136d5056416ba7ff6840bfbd"
        );
        verify_storage_commitment_signature(b"niahcia-dev", &commitment, public_key.as_bytes())
            .unwrap();
    }
    #[test]
    fn locked_service_epoch_vector_matches() {
        let secret = [0x07_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        assert_eq!(
            hex::encode(public_key.as_bytes()),
            "02989c0b76cb563971fdc9bef31ec06c3560f3249d6ee9e5d83c57625596e05f6f"
        );

        let service_node_id = derive_service_node_id(public_key.as_bytes()).unwrap();
        assert_eq!(
            hex::encode(service_node_id),
            "d070a1e364585c32ce042e933504a05b74913ff8d3bef399d6b9b9abf3fb3d8b"
        );

        let mut epoch = ServiceEpochAccumulator::new(0, 720).unwrap();
        epoch
            .record_success([0x01; 32], [0x10; 32], [0x20; 32], 4096)
            .unwrap();
        epoch
            .record_success([0x02; 32], [0x11; 32], [0x21; 32], 2048)
            .unwrap();
        assert_eq!(
            hex::encode(epoch.evidence_root()),
            "b510c5e894da2eec30586ac3109f551f16af2123e34e7429302df5252c1d54d7"
        );

        let finalized = finalize_service_epoch_report(service_node_id, &epoch, 2, 2, 2);
        let mut report = build_service_epoch_report_v1(
            b"niahcia-dev",
            &finalized,
            [0xbb; 32],
            2,
            vec!["ARCHIVE".into(), "MODEL_STORAGE".into()],
            [0xcc; 32],
        );

        assert_eq!(
            hex::encode(report.report_id),
            "4111ebc6d0283609830f9666ac017def52f977d5e170b2960acd9432a4809597"
        );
        assert_eq!(
            hex::encode(service_epoch_report_signing_digest(b"niahcia-dev", &report)),
            "61a0eb64c68188f8144a5a73bf41868f50c7afd58f2e63f61ea04582f0dcf552"
        );

        sign_service_epoch_report(b"niahcia-dev", &mut report, &secret).unwrap();
        assert_eq!(
            hex::encode(&report.signature),
            "40077b047eb28cdc5903d2e22aad96a56f0c21506a0a5a395a78763ea24135512df31821934df7d3b3a5dabff9314fe568d56f8620f8d20dddf59e018e797cc3"
        );
        verify_service_epoch_report_signature(b"niahcia-dev", &report, public_key.as_bytes())
            .unwrap();
    }
    #[test]
    fn secp256k1_service_epoch_signature_verifies_and_rejects_tampering() {
        let mut epoch = ServiceEpochAccumulator::new(0, 720).unwrap();
        epoch
            .record_success([0x01; 32], [0x10; 32], [0x20; 32], 4096)
            .unwrap();
        epoch
            .record_success([0x02; 32], [0x11; 32], [0x21; 32], 2048)
            .unwrap();

        let secret = [0x07_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        let service_node_id = derive_service_node_id(public_key.as_bytes()).unwrap();

        let finalized = finalize_service_epoch_report(service_node_id, &epoch, 2, 2, 2);
        let mut report = build_service_epoch_report_v1(
            b"niahcia-dev",
            &finalized,
            [0xbb; 32],
            2,
            vec!["ARCHIVE".into(), "MODEL_STORAGE".into()],
            [0xcc; 32],
        );

        sign_service_epoch_report(b"niahcia-dev", &mut report, &secret).unwrap();
        assert_eq!(report.signature.len(), 64);
        verify_service_epoch_report_signature(b"niahcia-dev", &report, public_key.as_bytes())
            .unwrap();

        let mut tampered = report.clone();
        tampered.verified_bytes_served += 1;
        assert!(verify_service_epoch_report_signature(
            b"niahcia-dev",
            &tampered,
            public_key.as_bytes(),
        )
        .is_err());

        assert!(verify_service_epoch_report_signature(
            b"other-network",
            &report,
            public_key.as_bytes(),
        )
        .is_err());
    }
    #[test]
    fn service_node_id_binds_to_public_key() {
        let secret = [0x08_u8; 32];
        let signing_key = k256::ecdsa::SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        let node_id = derive_service_node_id(public_key.as_bytes()).unwrap();

        assert_ne!(node_id, [0_u8; 32]);
        assert!(super::verify_service_node_identity(node_id, public_key.as_bytes()).is_ok());
        assert!(super::verify_service_node_identity([0xff; 32], public_key.as_bytes()).is_err());
    }
    #[test]
    fn service_epoch_report_builder_derives_id_and_signature_is_not_in_id() {
        let mut epoch = ServiceEpochAccumulator::new(0, 720).unwrap();
        epoch
            .record_success([0x01; 32], [0x10; 32], [0x20; 32], 4096)
            .unwrap();
        epoch
            .record_success([0x02; 32], [0x11; 32], [0x21; 32], 2048)
            .unwrap();

        let finalized = finalize_service_epoch_report([0xaa; 32], &epoch, 2, 2, 2);
        let mut report = build_service_epoch_report_v1(
            b"niahcia-dev",
            &finalized,
            [0xbb; 32],
            2,
            vec!["ARCHIVE".into(), "MODEL_STORAGE".into()],
            [0xcc; 32],
        );

        assert_ne!(report.report_id, [0_u8; 32]);
        assert_eq!(
            report.report_id,
            derive_service_epoch_report_id(b"niahcia-dev", &report)
        );

        let id_before_signature = report.report_id;
        attach_service_epoch_signature(&mut report, vec![0x30, 0x44, 0x01]).unwrap();

        assert_eq!(
            id_before_signature,
            derive_service_epoch_report_id(b"niahcia-dev", &report)
        );
        assert!(!report.signature.is_empty());
    }

    #[test]
    fn empty_service_epoch_signature_is_rejected() {
        let mut report = ServiceEpochReportV1 {
            report_id: [0; 32],
            service_node_id: [0; 32],
            operator_id: [0; 32],
            epoch_start_height: 0,
            epoch_end_height: 1,
            commitments_sampled: 0,
            challenges_passed: 0,
            challenges_failed: 0,
            deadlines_missed: 0,
            verified_bytes_served: 0,
            distinct_requester_count: 0,
            distinct_challenge_block_count: 0,
            service_classes: Vec::new(),
            evidence_root: [0; 32],
            eligibility_weight: 0,
            created_block: [0; 32],
            signature: Vec::new(),
        };
        assert!(attach_service_epoch_signature(&mut report, Vec::new()).is_err());
    }
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
    fn service_epoch_report_nce_is_deterministic_and_domain_bound() {
        let mut report = ServiceEpochReportV1 {
            report_id: [0_u8; 32],
            service_node_id: [0x11; 32],
            operator_id: [0x22; 32],
            epoch_start_height: 720,
            epoch_end_height: 1440,
            commitments_sampled: 8,
            challenges_passed: 7,
            challenges_failed: 1,
            deadlines_missed: 0,
            verified_bytes_served: 28_672,
            distinct_requester_count: 4,
            distinct_challenge_block_count: 7,
            service_classes: vec!["ARCHIVE".into(), "MODEL_STORAGE".into()],
            evidence_root: [0x33; 32],
            eligibility_weight: 28_672,
            created_block: [0x44; 32],
            signature: vec![],
        };

        let id = derive_service_epoch_report_id(b"devnet/prototype0", &report);
        assert_ne!(id, [0_u8; 32]);
        report.report_id = id;

        let signing_preimage = service_epoch_report_signing_preimage(&report);
        let signing_digest = service_epoch_report_signing_digest(b"devnet/prototype0", &report);
        let other_network_digest =
            service_epoch_report_signing_digest(b"testnet/prototype0", &report);

        assert_eq!(
            signing_preimage,
            service_epoch_report_signing_preimage(&report)
        );
        assert_ne!(signing_digest, other_network_digest);

        report.signature = vec![0xaa; 65];
        let canonical = service_epoch_report_canonical_bytes(&report);
        assert_ne!(canonical, signing_preimage);

        let mut changed = report.clone();
        changed.verified_bytes_served += 1;
        assert_ne!(
            derive_service_epoch_report_id(b"devnet/prototype0", &changed),
            report.report_id
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
