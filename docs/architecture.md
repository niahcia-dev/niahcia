# NIAHCIA Architecture

## System overview

NIAHCIA separates base-chain consensus from decentralized AI services.

```text
                         NIAHCIA

Layer 1 — CPU PoW blockchain
consensus / native value / smart contracts / commitments / settlement
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

Full nodes validate PoW/header rules, execute native transactions and the active deterministic smart-contract runtime, verify state transitions and commitments, persist canonical state, and participate in P2P synchronization.

## Smart contracts

Smart contracts are a core NIAHCIA base-chain capability. NativeTransaction V1 already reserves `ContractCall` and `ContractCreate`; those actions are intentionally not executable until a separately versioned native contract-runtime specification defines deterministic code, state/storage, gas, call/create, failure/revert, receipt, and persistence semantics.

The contract runtime is part of deterministic blockchain execution and remains subordinate to CPU-PoW consensus. It must not depend on external AI workers, storage providers, websites, network access, filesystem access, wall-clock time, or other nondeterministic host services.

AI inference remains off-chain. Contracts may eventually verify authenticated commitments, receipts, signatures, or other versioned evidence produced by off-chain services without executing inference inside consensus.

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
