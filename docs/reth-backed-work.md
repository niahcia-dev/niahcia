# Reth-backed Mining Candidate

NIAHCIA v0.1.0 now has the code path that turns a real Reth execution candidate into mining work.

## Flow

```text
eth_getBlockByNumber("latest")
        |
        v
current Reth parent
        |
        v
engine_forkchoiceUpdatedV3
        |
        | payloadId
        v
engine_getPayloadV3
        |
        v
ExecutionPayloadCommitments
        |
        | Keccak-256
        v
execution_commitment
        |
        v
PowWorkTemplate
        |
        v
pow_getWork
```

Reth's current Engine API exposes V3 payload building for Cancun-era execution. NIAHCIA uses the authenticated Engine API for payload building and the normal HTTP JSON-RPC endpoint to discover the current execution head.

## Configuration

```toml
reth_engine_api = "http://127.0.0.1:8551"
reth_http_rpc = "http://127.0.0.1:8545"
reth_jwt_path = "./jwt.hex"
fee_recipient = "0x0000000000000000000000000000000000000000"
```

Reth must expose HTTP JSON-RPC with `eth` enabled on the configured public endpoint.

## Execution commitment

The development commitment includes:

- parent hash
- Reth execution payload block hash
- fee recipient
- state root
- receipts root
- block number
- gas limit
- gas used
- timestamp
- base fee

The Reth execution payload block hash itself commits to the Ethereum execution header, including the transaction root. This avoids inventing a fake transaction-root calculation in the NIAHCIA consensus layer.

## Current development assumptions

For the first Reth-backed candidate:

- `engine_forkchoiceUpdatedV3` is used.
- `engine_getPayloadV3` is used.
- withdrawals are empty.
- `prevRandao` is zeroed because NIAHCIA does not use Ethereum PoS randomness for consensus.
- `parentBeaconBlockRoot` is zeroed because NIAHCIA does not have an Ethereum beacon chain.
- the development difficulty remains 1.
- the development target remains all `ff`.

Those last two values are intentionally not final PoW consensus.

## Important

This establishes the execution-to-work plumbing. It does not yet prove a live Reth devnet accepts the NIAHCIA-specific zeroed PoS-era attributes in every fork configuration.

That live integration test is required before issue #7 is closed.
