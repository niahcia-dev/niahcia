# Local Mining RPC

The v0.1.0 node exposes a small **development-only** local JSON-RPC interface for CPU mining work.

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

The response includes:

- development marker
- work generation
- template ID
- version
- height
- parent hash
- execution commitment
- timestamp
- difficulty
- target
- nonce start/end range

## Stale-work detection

Internally, the node keeps:

```text
generation + template_id
```

When the active template is replaced, the generation increments and the template ID changes. A miner can therefore discard work from an older generation.

The current code has a unit test proving this behavior.

## Important v0.1.0 limitation

The current startup template is intentionally a bootstrap template:

- height is 0,
- parent hash is zero,
- execution commitment is zero,
- difficulty is 1,
- target is all `ff`.

It proves the miner-facing RPC and stale-work mechanics only.

It is **not yet chain-valid mining work** because the next integration step must populate the template from an actual Reth execution payload and NIAHCIA consensus state.

This distinction is deliberate: the RPC exists now without pretending that the chain-building path is finished.
