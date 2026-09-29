# Local Mining RPC

The v0.1.0 node exposes a development-only local JSON-RPC interface for CPU mining work.

Default bind:

```text
127.0.0.1:9332
```

## Request current work

```bash
curl -s http://127.0.0.1:9332 \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"pow_getWork","params":[]}'
```

## Returned work

`pow_getWork` now represents an unfinished NIAHCIA `BlockHeaderV1`.

The response includes:

- generation
- template ID
- version
- NIAHCIA parent hash
- height
- timestamp
- transaction Merkle root
- execution root
- 256-bit target
- nonce range
- extra-nonce range

Difficulty is intentionally not a consensus field in the header; it is derived from target.

## Stale-work detection

The node tracks:

```text
generation + template_id
```

The template ID ignores only `nonce` and `extra_nonce`.

A change to parent, height, timestamp, transaction root, execution root, target, or version changes template identity and makes old work stale.

## Current development limitation

The first NIAHCIA parent is still a bootstrap zero hash until persistent NIAHCIA chain storage lands.

Execution data is now sourced from the local execution engine path, but live Reth devnet validation is still required before issue #7 is closed.
