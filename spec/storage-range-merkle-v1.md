# Storage Range Merkle V1

## Status

Draft proof-of-service sub-chunk proof format.

This tree lets a StorageResponseV1 prove that a challenged byte segment belongs to a committed chunk without transferring the entire chunk.

## Segmenting

A chunk is split into fixed-size segments.

The segment size is defined by the service profile and MUST be non-zero.

The final segment may be shorter.

Each segment has a zero-based `segment_index`.

## Leaf hash

```text
leaf =
  keccak256(
    "NIAHCIA/STORAGE-RANGE-LEAF/V1"
    || segment_index_u64_be
    || segment_length_u64_be
    || segment_bytes
  )
```

## Internal node

```text
node =
  keccak256(
    "NIAHCIA/STORAGE-RANGE-NODE/V1"
    || left
    || right
  )
```

Odd final nodes are duplicated.

## Empty chunk

```text
empty_range_root =
  keccak256(
    "NIAHCIA/STORAGE-RANGE-EMPTY/V1"
  )
```

## Chunk commitment

For proof-of-service manifests, the per-chunk commitment SHOULD include both:

```text
chunk_hash
range_root
```

where:

- `chunk_hash` authenticates the exact full chunk bytes,
- `range_root` authenticates independently provable sub-chunk segments.

The manifest leaf specification SHOULD commit to both values before Protocol v1 is frozen.

## Proof format

A range proof is an ordered sequence of:

```text
RangeProofStep
- sibling
- sibling_is_left
```

Verification begins with the exact challenged segment leaf and folds upward using the same left/right rule as other NIAHCIA Merkle proofs.

## Challenge alignment

The initial implementation proves full Merkle segments, not arbitrary unaligned byte substrings.

Therefore the challenge selector should ultimately resolve a requested byte range into one or more complete segment indexes.

This avoids pretending that an arbitrary substring by itself proves membership in the full chunk.

## Security note

This two-level structure is intentional:

```text
manifest_root
    ↓
chunk commitment
    ↓
range_root
    ↓
challenged segment bytes
```

A provider must answer with bytes that authenticate to the committed range root and a chunk commitment that authenticates to the storage manifest.
