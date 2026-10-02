# Storage Manifest Merkle V1

## Status

Draft proof-of-service commitment format.

A storage manifest commits to an ordered list of content chunks. The manifest root is used by StorageCommitmentV1 and verified by StorageResponseV1 proofs.

## Chunk entry

Each chunk entry contains:

```text
chunk_index
chunk_length
chunk_hash
range_root
```

`chunk_index` is the zero-based position in the ordered manifest.

`chunk_hash` is the content hash of the complete chunk.

`range_root` is the intra-chunk Merkle root defined by `storage-range-merkle-v1.md`.

## Leaf hash

```text
leaf =
  keccak256(
    "NIAHCIA/STORAGE-MANIFEST-LEAF/V1"
    || chunk_index_u64_be
    || chunk_length_u64_be
    || chunk_hash
    || range_root
  )
```

## Internal node

```text
node =
  keccak256(
    "NIAHCIA/STORAGE-MANIFEST-NODE/V1"
    || left
    || right
  )
```

If a level contains an odd final node, that node is duplicated as both left and right.

## Empty manifest

```text
empty_root =
  keccak256(
    "NIAHCIA/STORAGE-MANIFEST-EMPTY/V1"
  )
```

A StorageCommitmentV1 used for actual retention service MUST NOT have zero chunks even though the empty root is defined for deterministic completeness.

## Proof format

A proof for one chunk is an ordered sequence of:

```text
ManifestProofStep
- sibling
- sibling_is_left
```

Verification starts from the exact leaf hash and folds upward.

If `sibling_is_left = true`:

```text
current = node(sibling, current)
```

otherwise:

```text
current = node(current, sibling)
```

The proof succeeds only when the final value equals the committed manifest root.

## Security properties

The manifest commits simultaneously to:

- chunk ordering,
- chunk lengths,
- chunk hashes,
- intra-chunk range roots.

Changing a chunk's bytes, length, position, or sibling path changes the root.

## Proof-of-service use

A StorageResponseV1 may return only the challenged byte range, but the verifier must also receive enough information to authenticate the containing full chunk hash against this manifest root.

How partial-range bytes are tied to the full chunk hash is a separate response-proof concern and is not solved merely by this manifest Merkle proof.
