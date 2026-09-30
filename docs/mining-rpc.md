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


## Submit solved work

`pow_submitWork` submits only the miner-controlled search values for the current template.

Example:

```bash
curl -s http://127.0.0.1:9332 \
  -H 'content-type: application/json' \
  --data '{
    "jsonrpc":"2.0",
    "id":2,
    "method":"pow_submitWork",
    "params":{
      "generation":0,
      "template_id":"<64-hex-template-id>",
      "nonce":123,
      "extra_nonce":0
    }
  }'
```

The node does not trust a miner-provided PoW hash. It reconstructs the current `BlockHeaderV1`, inserts the submitted `nonce` and `extra_nonce`, hashes the exact 164-byte header with its own RandomX VM and current 32-byte seed, and independently checks the unsigned big-endian hash against the header target.

Before hashing, the node requires the submitted `generation` and `template_id` to match current work. After successful verification, the block is inserted into the NIAHCIA-owned persistent chain store and the template is marked solved so duplicate submissions are rejected.

A successful result contains:

- `accepted: true`
- NIAHCIA block ID
- independently computed RandomX hash
- NIAHCIA height
- resulting cumulative work

## Current development limitation

After accepting a solution, this bootstrap node marks that template solved but does not yet automatically build and install the next Reth-backed execution template. Until template-refresh wiring lands, `pow_getWork` returns a solved-template error rather than handing miners the already-accepted block again.

Execution-engine finalization and canonical-head notification are still separate development work. The current submission path proves and persists the NIAHCIA PoW boundary; it is not yet the complete production block-import pipeline.
