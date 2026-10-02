# MemoryDescriptor

## Purpose

`MemoryDescriptor` identifies persistent or ephemeral agent state without requiring the state itself to live on-chain.

## Canonical fields

```text
MemoryDescriptor
- schema_version
- memory_id
- scope
- owner
- agent_id
- session_id
- root_hash
- storage_manifest_hash
- encryption_policy
- read_policy
- write_policy
- replication_policy
- version
- previous_root
- updated_block
```

## Scopes

```text
SESSION
USER
AGENT_GLOBAL
SHARED
PRIVATE
ARCHIVE
```

## State commitment

`root_hash` commits to the canonical logical memory state.

The underlying blobs/chunks live on service nodes or compatible content-addressed storage.

## Updates

Memory updates use optimistic concurrency:

```text
old_root
   |
execution/update
   v
new_root
```

An update is valid only if the expected `old_root` still matches the current descriptor state.

This prevents concurrent workers from silently overwriting one another.

## Replication

`replication_policy` may specify:

- minimum replicas
- desired replicas
- service classes
- geographic/operator diversity
- retention period
- payment/budget reference

## Privacy

`encryption_policy` defines whether stored memory is plaintext, client-encrypted, worker-readable, or subject to a future privacy mechanism.

Content addressing does not imply confidentiality.

## Invariants

1. Memory blobs are not authoritative without matching the committed root.
2. State updates are append/audit friendly.
3. Global/shared memory writes require stricter authority than ordinary session memory.
4. Storage providers cannot redefine memory state by serving different bytes with invalid hashes.

## Prototype 0

Prototype 0 initially needs SESSION memory semantics; other scopes remain valid protocol concepts.
