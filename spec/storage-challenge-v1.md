# Storage Challenge V1

## Status

Draft proof-of-service object.

A storage challenge selects unpredictable data from an existing StorageCommitmentV1.

## Canonical fields

```text
StorageChallengeV1
- schema_version
- challenge_id
- commitment_id
- challenge_block_id
- challenge_height
- challenge_seed
- requested_ranges
- issued_at
- response_deadline
- challenger_id
- signature
```

For the segment-aligned proof path, each requested range is canonically encoded as the four-element NCE/1 array:

```text
[ chunk_index, segment_index, offset, length ]
```

All four values are unsigned integers. `length` MUST be non-zero. The array form is intentional: positions are permanent for `StorageChallengeV1` schema version 1 and avoid introducing unregistered nested map keys.

## Seed derivation

The deterministic challenge seed is:

```text
challenge_seed =
  keccak256(
    "NIAHCIA/STORAGE-CHALLENGE/V1"
    || challenge_block_id
    || commitment_id
  )
```

Requested chunk indexes and byte ranges are derived from the seed by the deterministic selection algorithm defined by the service profile.

The provider MUST NOT be able to know the challenged ranges before the referenced NIAHCIA challenge block exists.

## Deadline

A challenge includes a protocol/service-profile deadline.

Deadlines SHOULD be long enough to tolerate ordinary network variance but short enough that repeatedly proxy-fetching arbitrary missing data is economically unattractive.

## Authority

A challenge is a service measurement object.

It does not affect chain validity or fork choice.


## Deterministic range selection

The initial selector is deterministic and implementation-independent.

For each requested range number `counter = 0..N-1`:

```text
selector_digest =
  keccak256(
    "NIAHCIA/STORAGE-SELECT/V1"
    || challenge_seed
    || counter_u64_be
  )

chunk_word  = unsigned_be_u64(selector_digest[0..8])
offset_word = unsigned_be_u64(selector_digest[8..16])

chunk_index = chunk_word mod chunk_count

chunk_length = manifest.chunk_lengths[chunk_index]
length       = min(chunk_length, max_range_length)
max_offset   = chunk_length - length

offset =
  0,                               if max_offset == 0
  offset_word mod (max_offset+1),  otherwise
```

All chunks in a challengeable manifest MUST have non-zero length.

The ordered `chunk_lengths` list is part of the manifest commitment.

This selector intentionally samples both chunk identity and an internal byte range, reducing the usefulness of keeping only small challenge caches.

The initial selector uses modulo reduction. This is acceptable for service-layer sampling because it does not affect PoW consensus or chain validity. A later protocol revision may adopt rejection sampling if stronger statistical uniformity is required.


## Segment-aligned selector

The service implementation SHOULD challenge complete intra-chunk Merkle segments rather than arbitrary byte substrings.

Given a fixed `segment_size`, derive for each requested sample:

```text
selector_digest =
  keccak256(
    "NIAHCIA/STORAGE-SELECT/V1"
    || challenge_seed
    || counter_u64_be
  )

chunk_word   = BE_U64(selector_digest[0..8])
segment_word = BE_U64(selector_digest[8..16])

chunk_index   = chunk_word mod chunk_count
chunk_length  = manifest.chunk_lengths[chunk_index]
segment_count = ceil(chunk_length / segment_size)
segment_index = segment_word mod segment_count

offset = segment_index * segment_size
length = min(segment_size, chunk_length - offset)
```

The challenged response then returns the complete selected segment plus its `storage-range-merkle-v1.md` proof.

This segment-aligned selector supersedes arbitrary unaligned byte-range sampling for the proof path that is intended to settle service rewards.


## Canonical NCE/1 encoding

`StorageChallengeV1` uses object type `0x0206` and schema version `1`.

The reference implementation uses the segment-aligned `requested_ranges` representation described above.

The builder derives `challenge_seed` directly from `challenge_block_id` and `commitment_id`; callers do not supply an arbitrary seed.

### Challenge ID

The ID preimage excludes fields `2 challenge_id` and `11 signature`.

```text
challenge_id =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "ID/STORAGE_CHALLENGE" || 0x00 ||
    network_id || 0x00 ||
    nce1_challenge_without_challenge_id_or_signature
  )
```

### Signing digest

The signing preimage includes `challenge_id` and excludes only field `11 signature`.

```text
signing_digest =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "SIGN/STORAGE_CHALLENGE" || 0x00 ||
    network_id || 0x00 ||
    nce1_challenge_without_signature
  )
```

The initial signed challenge path treats `challenger_id` as a service identity derived from the challenger's compressed secp256k1 public key via `NIAHCIA/SERVICE-NODE-ID/V1`.

Signatures use fixed-width compact secp256k1 ECDSA `r[32] || s[32]`.

Verification MUST independently recompute the challenge seed, challenge ID, network-bound signing digest, challenger identity binding, non-empty requested segment set, non-zero segment lengths, and a deadline strictly later than `issued_at`.

Protocol-derived unsigned challenge scheduling may be specified separately later; it MUST NOT silently overload this signed peer-challenge encoding.


## Locked interoperability vector

The current compatibility vector is `test-vectors/storage-challenge-v1.json`. Implementations SHOULD reproduce the challenge seed, challenger identity, NCE/1 ID preimage, `challenge_id`, signing preimage, signing digest, compact secp256k1 signature, and final canonical signed bytes exactly.
