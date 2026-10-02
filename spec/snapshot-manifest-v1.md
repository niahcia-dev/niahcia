# Snapshot Manifest V1

## Status

Draft service-layer specification.

A snapshot is an optimization for bootstrapping NIAHCIA execution/state data.

It is not a checkpoint and does not create trusted finality.

## Canonical fields

```text
SnapshotManifestV1
- schema_version
- network_id
- height
- block_id
- cumulative_work
- state_root
- execution_root
- chunk_size
- chunk_count
- chunk_manifest_root
- total_bytes
- created_at
- provider_service_node_id
- signature
```

## Content addressing

Snapshot chunks MUST be content-addressed.

The chunk manifest MUST commit to `chunk_index`, `chunk_hash`, and `chunk_length`.

The ordered chunk entries are committed by `chunk_manifest_root`.

## Client verification

A client restoring from a snapshot MUST independently verify:

1. provider signature,
2. all chunk hashes,
3. ordered chunk manifest root,
4. referenced NIAHCIA block ID,
5. the block's cumulative-work ancestry,
6. state root,
7. execution root,
8. network identity.

If any check fails, the snapshot is rejected.

## Multiple providers

Clients SHOULD be able to retrieve chunks from multiple independent providers.

A snapshot manifest SHOULD be usable without remaining dependent on the original publisher.

## No checkpoint authority

A snapshot provider cannot declare a height finalized.

A snapshot manifest cannot override PoW validation, cumulative-work fork choice, reorganization rules, or execution validation.

If the canonical chain later reorganizes below the snapshot anchor, the snapshot may become stale and MUST NOT be treated as authoritative.
