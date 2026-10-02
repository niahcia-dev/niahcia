# Storage Commitment V1

## Status

Draft proof-of-service object.

A storage commitment records a service node's promise to retain retrievable content for a stated period.

## Canonical fields

```text
StorageCommitmentV1
- schema_version
- commitment_id
- service_node_id
- operator_id
- service_class
- object_id
- manifest_root
- chunk_count
- total_bytes
- retained_from_block
- retained_until_block
- commitment_nonce
- created_block
- signature
```

## Rules

- `manifest_root` MUST commit to the complete ordered chunk manifest.
- `chunk_count` and `total_bytes` MUST match the manifest.
- `service_class` MUST be one of the storage-bearing service classes defined by the service-node specification.
- the retention interval is inclusive of `retained_from_block` and exclusive of `retained_until_block`.
- `commitment_nonce` prevents accidental commitment-ID collisions for otherwise identical commitments.
- the provider signature proves authorship of the commitment, not successful service.

## Commitment ID

```text
commitment_id =
  digest(
    purpose = "ID/STORAGE_COMMITMENT",
    canonical object with commitment_id and signature excluded
  )
```

## Reward meaning

A StorageCommitmentV1 creates eligibility to prove service.

It does not by itself create a reward entitlement.


## Canonical NCE/1 encoding

`StorageCommitmentV1` uses object type `0x0205` and schema version `1`. Payload field IDs are the permanent assignments in `field-id-registry.md`.

`chunk_count`, `total_bytes`, retention heights, and `commitment_nonce` are unsigned 64-bit values in v1.

### Commitment ID

The ID preimage excludes both field `2 commitment_id` and field `14 signature`, while retaining the normal NCE/1 envelope.

```text
commitment_id =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "ID/STORAGE_COMMITMENT" || 0x00 ||
    network_id || 0x00 ||
    nce1_commitment_without_commitment_id_or_signature
  )
```

### Signing digest

The signing preimage includes `commitment_id` and excludes only field `14 signature`.

```text
signing_digest =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "SIGN/STORAGE_COMMITMENT" || 0x00 ||
    network_id || 0x00 ||
    nce1_commitment_without_signature
  )
```

The reference implementation signs this 32-byte digest with secp256k1 ECDSA and stores the signature as fixed-width compact `r[32] || s[32]`.

Verification MUST also confirm that the supplied public key derives the claimed `service_node_id` using `NIAHCIA/SERVICE-NODE-ID/V1`.

### Builder validation

The current reference builder rejects zero `chunk_count`, zero `total_bytes`, an empty `service_class`, and an empty or reversed retention interval.

Manifest totals are still independently checked against the referenced manifest during service verification; merely encoding those counts does not prove they are correct.


## Locked interoperability vector

The current compatibility vector is `test-vectors/storage-commitment-v1.json`. Implementations SHOULD reproduce the compressed public key, `service_node_id`, NCE/1 ID preimage, `commitment_id`, signing preimage, signing digest, compact secp256k1 signature, and final canonical signed bytes exactly.
