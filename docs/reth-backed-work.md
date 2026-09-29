# Reth-backed Mining Candidate

NIAHCIA uses Reth as an **execution engine**, not as a source of blockchain identity or consensus.

The architectural rule is:

> **Reth calculates EVM execution results. NIAHCIA defines the block.**

## Flow

```text
Reth execution engine
        |
        | executes candidate transactions
        v
state_root
receipts_root
ordered raw transactions
gas data
base fee
        |
        v
NIAHCIA-native execution commitment
        |
        v
NIAHCIA PowWorkTemplate
        |
        v
NIAHCIA block hash / RandomX work
```

The Reth execution payload block hash is retained only as **diagnostic metadata**. It is not included in the NIAHCIA execution commitment and does not define NIAHCIA block identity.

## Two different parent relationships

During development it is important not to confuse these:

```text
NIAHCIA parent hash
    = previous NIAHCIA block

execution parent hash
    = execution-engine parent used by Reth
```

They serve different purposes.

The current first-block bootstrap uses an all-zero NIAHCIA parent until persistent NIAHCIA chain state is implemented. The Reth execution parent remains available separately in the execution metadata.

## NIAHCIA transaction commitment

NIAHCIA does not depend on Reth's Ethereum execution block hash to indirectly commit to transactions.

Instead it creates its own deterministic transaction commitment from the ordered raw transaction bytes returned in the execution payload:

```text
keccak256(
    "NIAHCIA/TRANSACTIONS/V1"
    || transaction_count
    || len(tx0) || tx0
    || len(tx1) || tx1
    || ...
)
```

This is intentionally a NIAHCIA-native commitment.

It is not Ethereum's Merkle-Patricia transaction trie root.

The EVM still interprets the transactions using Ethereum-compatible transaction semantics, but NIAHCIA owns the outer block commitment.

## Execution commitment

The current development execution commitment contains:

- execution parent hash
- fee recipient
- EVM state root
- EVM receipts root
- NIAHCIA transaction commitment
- execution block number
- gas limit
- gas used
- timestamp
- base fee

It explicitly does **not** contain:

- Ethereum mainnet block identity
- Ethereum beacon-chain identity
- Reth's execution payload block hash as a consensus field
- Ethereum validator/finality data

## Independence requirement

A future execution engine should be replaceable.

Conceptually:

```text
NIAHCIA + Reth
NIAHCIA + another compatible EVM engine
```

should be able to produce the same NIAHCIA consensus commitments from the same ordered transactions and execution results.

No Ethereum network connection is required.

## Configuration

```toml
reth_engine_api = "http://127.0.0.1:8551"
reth_http_rpc = "http://127.0.0.1:8545"
reth_jwt_path = "./jwt.hex"
fee_recipient = "0x0000000000000000000000000000000000000000"
```

These endpoints are for the operator's own local Reth process.

## Current development assumptions

The current integration still uses Reth's Engine API V3 as the transport for requesting execution payloads. This is an implementation interface, not NIAHCIA consensus.

For the first development candidate:

- withdrawals are empty,
- `prevRandao` is zeroed,
- `parentBeaconBlockRoot` is zeroed,
- development difficulty remains 1,
- development target remains all `ff`.

A live local integration test is still required before issue #7 is closed.
