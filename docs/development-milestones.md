# Development Milestones

NIAHCIA development is organized around runnable, testable releases rather than broad architecture phases.

## v0.1.0 — First Runnable Node

Goal: prove the base execution/consensus boundary with one runnable NIAHCIA node process.

Acceptance target:

```text
niahcia node starts
↓
loads configuration
↓
connects to local Reth
↓
obtains/constructs an execution payload template
↓
builds a PoW work template
↓
exposes local mining work
```

This release does not need a public network, polished miner, AI compute, explorer, or service nodes.

Required work:

- Rust workspace/bootstrap
- node/pow daemon startup and configuration
- authenticated Reth Engine API client
- execution payload/template abstraction
- PoW header/work-template types
- local RPC for mining work
- logging and version reporting
- smoke-test documentation

## v0.2.0 — First Mined Block

Goal: mine and accept the first valid RandomX-secured NIAHCIA block.

Required work:

- RandomX integration
- nonce search
- PoW validation
- block submission path
- execution payload import
- accepted/rejected block telemetry
- deterministic development network parameters

Acceptance target:

```text
node + miner
↓
work template
↓
RandomX solution
↓
block submission
↓
block accepted
```

## v0.3.0 — Multi-Node Chain

Goal: two or more independent nodes agree on the same CPU-PoW chain.

Implementation handoff: [v0.3.0 multi-node devnet](multi-node-devnet-handoff.md)

Required work:

- chain P2P
- block/header propagation
- cumulative-work fork choice
- difficulty adjustment
- synchronization
- basic reorganization handling

Acceptance target:

- Node A and Node B follow the same canonical chain.
- A miner can connect to either node.
- Restarting one node does not destroy network continuity.

Current verified devnet result:

- independent Reth A / NIAHCIA A and Reth B / NIAHCIA B state directories
- Node A mined heights 0, 1, and 2
- Node B synchronized to the same NIAHCIA canonical tip over the native P2P protocol
- Node B was stopped, Node A mined height 3, and Node B restarted
- Node B caught up without copying NIAHCIA or Reth state
- both execution nodes reached canonical execution height 4
- reproduced with `scripts/devnet-two-node-smoke.sh`

This proves linear two-node synchronization and restart catch-up.

Additional verified fork/reorg result:

- Node A and Node B were first synchronized to the same canonical tip
- both NIAHCIA nodes were stopped and restarted without peers
- isolated Node B mined a competing block at height 4
- isolated Node A mined its own height-4 block plus height 5, creating the higher-work branch
- Node B was restarted against Node A with its competing persisted fork intact
- Node B discovered the shared ancestor, validated A's branch locally, reorganized to the higher-work chain, and advanced to next work height 6
- both independent execution backends converged to canonical execution height 6
- reproduced with `scripts/devnet-two-node-smoke.sh`

This verifies basic common-ancestor discovery and live two-node reorg convergence on the current devnet implementation.

## v0.4.0 — EVM Usable Network

Goal: ordinary Ethereum-style transactions and Solidity contracts work on the CPU-PoW network.

Required work:

- Ethereum-compatible JSON-RPC
- wallet connection
- transaction propagation
- contract deployment
- gas/base-fee handling
- receipts/logs
- chain ID/network parameters

## v0.5.0 — AI Job Prototype

Goal: first end-to-end off-chain AI execution settled through the chain.

Required work moves primarily into:

- `niahcia-protocol`
- `niahcia-compute`

The base chain must remain independent of AI availability.

## Release rule

A milestone version is not considered complete because code exists.

It is complete only when its acceptance target can be reproduced from a clean setup using documented commands.
