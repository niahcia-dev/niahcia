# Storage Response V1

## Status

Draft proof-of-service object.

A storage response proves that a service node answered a StorageChallengeV1.

## Canonical fields

```text
StorageResponseV1
- schema_version
- response_id
- challenge_id
- commitment_id
- service_node_id
- answered_at
- range_proofs
- response_bytes_hash
- signature
```

For the segment-aligned proof path, each range proof is canonically encoded as:

```text
[
  chunk_index,
  segment_index,
  offset,
  length,
  returned_bytes,
  chunk_length,
  chunk_hash,
  range_root,
  range_proof,
  manifest_proof
]
```

Each Merkle proof is an array of two-element arrays:

```text
[ sibling_bytes32, sibling_is_left_bool ]
```

The tuple positions are permanent for `StorageResponseV1` schema version 1.

## Verification

A verifier MUST check:

1. the response matches the referenced challenge,
2. every requested range is present exactly once,
3. returned bytes hash to the claimed chunk/range commitment,
4. each chunk is included in the committed manifest root,
5. the response arrived before the challenge deadline,
6. the provider signature is valid.

## Proof level

A successful response proves availability for the sampled ranges at the challenge time.

It does not prove that every byte of the committed object was retained locally at every instant.


Manifest membership proofs MUST follow `storage-manifest-merkle-v1.md`.


For sub-chunk challenges, range membership proofs MUST follow `storage-range-merkle-v1.md`.

A response must not claim that an arbitrary byte substring is authenticated by the full chunk hash alone. It must provide the complete challenged Merkle segment or segments plus their range proofs.


The manifest leaf MUST bind the exact `range_root` used to verify the challenged segment. A valid range proof against an uncommitted or different range root is invalid.


## Response bytes hash

`response_bytes_hash` commits to the ordered returned segment bytes and their requested coordinates.

The canonical byte-hash payload is:

```text
[
  [ chunk_index, segment_index, offset, length, returned_bytes ],
  ...
]
```

encoded canonically, then:

```text
response_bytes_hash =
  keccak256(
    "NIAHCIA/STORAGE-RESPONSE-BYTES/V1"
    || canonical_returned_segment_array
  )
```

The order MUST match the challenge's ordered requested segment list.

## Canonical NCE/1 encoding

`StorageResponseV1` uses object type `0x0207` and schema version `1`.

### Response ID

The ID preimage excludes fields `2 response_id` and `9 signature`.

```text
response_id =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "ID/STORAGE_RESPONSE" || 0x00 ||
    network_id || 0x00 ||
    nce1_response_without_response_id_or_signature
  )
```

### Signing digest

The signing preimage includes `response_id` and excludes only field `9 signature`.

```text
signing_digest =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "SIGN/STORAGE_RESPONSE" || 0x00 ||
    network_id || 0x00 ||
    nce1_response_without_signature
  )
```

Signatures use fixed-width compact secp256k1 ECDSA `r[32] || s[32]`.

The supplied provider public key MUST derive the claimed `service_node_id`.

## Evidence verification

The initial reference verifier additionally requires:

- response challenge and commitment IDs match the challenge,
- `answered_at <= response_deadline`,
- response proof count equals requested segment count,
- proofs appear in the same order as requested segments,
- every returned byte length exactly matches the requested segment length,
- each returned segment authenticates to its committed `range_root`,
- each `range_root`, `chunk_length`, and `chunk_hash` authenticates to the commitment's manifest root.

Successful verification returns the exact number of bytes authenticated by the challenge.


## Locked interoperability vector

The current compatibility vector is `test-vectors/storage-response-v1.json`. Implementations SHOULD reproduce the provider identity, `range_root`, `response_bytes_hash`, NCE/1 ID preimage, `response_id`, signing preimage, signing digest, compact secp256k1 signature, and final canonical signed bytes exactly.
