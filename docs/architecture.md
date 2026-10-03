# NIAHCIA Architecture

## System overview

NIAHCIA separates base-chain consensus from decentralized AI services.

```text
                         NIAHCIA

Layer 1 — CPU PoW blockchain
consensus / transactions / balances / commitments / settlement
                         |
          +--------------+--------------+
          |                             |
          v                             v
wallet / Agent control          optional service protocols
          |                             |
          v                             v
AI compute workers              storage / retrieval / indexing
```

## Base-chain authority

CPU Proof-of-Work alone determines the canonical chain by cumulative valid work.

The active reference node uses native NIAHCIA execution and state. It does not require an external EVM execution engine.

Full nodes validate PoW/header rules, execute native transactions, verify state transitions and commitments, persist canonical state, and participate in P2P synchronization.

## Compute workers

Compute workers are replaceable service providers. For the first paid-compute milestone, worker discovery/selection is wallet-local from signed expiring advertisements; ordinary jobs/prompts/streaming output/usage receipts remain off-chain; one bounded ComputeChannel aggregates many jobs with one selected worker; the base chain sees channel open plus final settlement or refund.

Workers gain no consensus authority.

## Wallet / Agent layer

The wallet is the V1 Agent home/control plane. Private conversation history and Agent memory are wallet/client-local and encrypted by default. Spending keys are separate from chat-content encryption keys. Portals receive only bounded delegated capabilities.

## Storage/service layer

Decentralized storage is optional and deferred for the first ordinary AI milestone. Future storage/service providers do not vote on canonical chain state.

## Fast path and settlement path

```text
FAST PATH
wallet/client -> selected worker -> streamed response -> wallet/client

SETTLEMENT PATH
wallet opens bounded ComputeChannel
        -> off-chain usage receipts
        -> final Settle or timeout Refund
        -> native chain state transition
```
