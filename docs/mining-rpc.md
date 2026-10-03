# Local Mining RPC

The pre-alpha node exposes a development JSON-RPC interface for CPU mining work.

Default bind:

```text
127.0.0.1:9332
```

## Request current work

```bash
curl -s http://127.0.0.1:9332   -H 'content-type: application/json'   --data '{"jsonrpc":"2.0","id":1,"method":"pow_getWork","params":{}}'
```

`pow_getWork` returns the active native mining template, including generation/template identity, parent, height, timestamp, transaction root, execution root, target, and RandomX seed information.

## Stale-work detection

The node binds work to the current template identity. Changes to parent/state/transactions/fee recipient or other committed template fields invalidate stale work.

## Submit solved work

`pow_submitWork` submits miner-controlled search values for the current template. The node independently reconstructs and validates the candidate; it does not trust a miner-provided PoW hash.

Successful native blocks are persisted through the NIAHCIA-owned block/body/execution/state path and trigger the next template.

## Status

This RPC is a development interface. Stock miner/pool interoperability remains an explicit future compatibility target and may require a separately specified pool-facing protocol without changing consensus semantics.
