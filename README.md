# NIAHCIA


NIAHCIA is a pre-alpha permissionless CPU Proof-of-Work blockchain built for decentralized AI, native smart contracts, wallet-controlled Agents, and open compute.

> **Status:** pre-alpha / devnet. There is no production network or public binary release yet.

## What is NIAHCIA?

NIAHCIA separates blockchain consensus from AI execution.

```text
CPU Proof-of-Work blockchain
  consensus / native value / smart contracts / commitments / settlement
        |
        +-- wallet-controlled Agents and encrypted chat
        +-- decentralized AI compute workers
        +-- optional storage / service providers
```

Core rules:

- **CPU Proof-of-Work is the sole chain-consensus authority.**
- The base chain must remain valid and usable even if every AI worker and optional storage/service provider disappears.
- **Smart contracts are a first-class base-chain requirement.**
- AI workers, service nodes, storage providers, websites, and schedulers receive no fork-choice or finality authority.
- AI inference stays off-chain; contracts may verify explicitly defined deterministic evidence without performing inference inside consensus.
- Wallet/client-local encrypted chat history and Agent memory are the default first persistence model.
- Prompts, responses, private memories, and private tool results are not intended to be published on-chain.

## Current architecture

### Layer 1 — CPU PoW blockchain

The active devnet reference node uses native NIAHCIA execution rather than the former Reth/EVM/Engine-API path.

Implemented foundations include:

- RandomX CPU Proof-of-Work;
- cumulative-work fork choice and reorg handling;
- the 164-byte `BlockHeaderV1`;
- native Account and Contract address kinds;
- NativeTransaction V1;
- NativeState V1 deterministic snapshots and state roots;
- native transfer execution with nonce, fee, and producer-priority-fee accounting;
- NativeBlockBody V1;
- native block/header/body/execution/state persistence;
- native mining work/submission RPC;
- shared native mempool;
- P2P Version 3 full-body transaction/block transport;
- restart-safe recovery and canonical replay.

### Native smart contracts

Smart contracts have been part of the NIAHCIA design from the beginning.

`ContractCall` and `ContractCreate` are already reserved native transaction actions, and Contract addresses are part of Address V1.

The deterministic native contract runtime is intentionally **not active yet**. Before activation it must define and vector:

- contract code representation;
- contract creation and address derivation behavior;
- canonical contract state/storage commitments;
- create/call/revert/failure semantics;
- native NIAH value movement;
- bounded deterministic memory/execution;
- gas/resource accounting;
- receipts/events where supported;
- persistence, restart, and reorg behavior;
- explicit activation/versioning.

Consensus contracts must not depend on network access, filesystem state, host wall-clock time, external AI workers, or other nondeterministic host services.

See [Native Contract Runtime V1](spec/native-contract-runtime-v1.md).

### Native compute settlement

Inactive development work extends native state with ComputeChannels for decentralized AI payment settlement.

Implemented inactive foundations include:

- NativeState V2 — accounts plus ComputeChannel state;
- NativeTransaction V2;
- canonical `ComputeChannelOpen`, `ComputeChannelSettle`, and `ComputeChannelRefund` payloads;
- signed cumulative `ComputeUsageReceiptV1`;
- deterministic Open / Settle / Refund planners;
- atomic clone-then-commit transition application;
- stale-plan and duplicate-terminal rejection;
- deterministic state-root and snapshot round trips;
- restart/reorg recovery tests;
- Versioned NativeBlockBody V2 carrying V1 and V2 signed transactions;
- inactive V2 transfer execution preserving V1 transfer semantics;
- inactive atomic versioned-block execution for V1 Transfer, V2 Transfer, and ComputeChannel actions.

The V2 compute path is **not accepted by active mempool, P2P, or mining**.

No production compute gas schedule or activation height has been assigned. The inactive block executor currently requires zero placeholder gas/fee fields for compute actions so undefined gas rules cannot accidentally become consensus behavior.

### AI compute

GPU/accelerator workers perform model inference off-chain.

The intended first compute model is:

```text
wallet / Agent
      |
      +--> discover worker
      +--> open bounded ComputeChannel
      +--> send AI jobs off-chain
      +--> receive streamed results
      +--> accumulate signed usage receipts
      |
      +--> settle final receipt on-chain
```

Workers are replaceable service providers. They do not mine by requirement and do not gain consensus authority from providing AI compute.

### Wallet-native Agents and chat

The wallet is intended to be the user's durable AI home.

NIAHCIA's candidate wallet/chat architecture separates:

- wallet spending keys;
- chat-content encryption keys;
- delegated/session capabilities;
- Agent identity and permissions;
- compute/payment authorization.

Private chat history and Agent memory are local/encrypted by default. Websites such as `niahcia.com` are intended to act as portals to wallet-controlled identity and sessions rather than owning the user's AI identity or long-term private state.

## Repository role

This repository is the canonical working repository for the reference node plus consolidated protocol, specification, and interoperability-vector material during pre-alpha development.

`niahcia-protocol` remains a synchronized protocol mirror.

Related repositories:

```text
niahcia/niahcia              canonical node / chain / protocol working repo
niahcia/niahcia-protocol     protocol mirror
niahcia/niahcia-miner        CPU miner
niahcia/niahcia-compute      AI compute worker
niahcia/niahcia-explorer     explorer
niahcia/niahcia-web          web application
niahcia/niahcia.github.io    public project site
```

## Quick links

- [Project site](https://niahcia.com/)
- [Downloads](https://niahcia.com/downloads.html)
- [GitHub Releases](https://github.com/niahcia/niahcia/releases)
- [Protocol mirror](https://github.com/niahcia/niahcia-protocol)
- [Miner](https://github.com/niahcia/niahcia-miner)
- [Compute worker](https://github.com/niahcia/niahcia-compute)
- [Explorer](https://github.com/niahcia/niahcia-explorer)
- [Web application](https://github.com/niahcia/niahcia-web)

## Releases

There is currently **no public binary release**.

When binary releases begin, this repository's [Releases](https://github.com/niahcia/niahcia/releases) page will be the canonical distribution point. Release packaging, checksums, manifests, and expected assets are documented in [RELEASES.md](RELEASES.md).

GitHub tag source archives are source snapshots; they are not packaged NIAHCIA runtime binaries.

## Build from source

Current development builds use Rust/Cargo:

```bash
git clone https://github.com/niahcia/niahcia.git
cd niahcia
cargo build --release
```

Run the current reference node with the development configuration appropriate to the active devnet work.

This is pre-alpha software. Interfaces, state formats, activation rules, and network parameters may change before public testnet/mainnet freeze.

## Development status

Start a development session with:

- [Current Work / Session Handoff](docs/CURRENT-WORK.md)
- [Specification Status Audit](docs/spec-status.md)
- [Architecture](docs/architecture.md)
- [Implementation](docs/implementation.md)
- [Roadmap](ROADMAP.md)

Important development boundaries:

- do not reinterpret locked V1 formats to smuggle in V2 behavior;
- do not activate NativeStateV2 / NativeTransactionV2 without explicit network parameters and migration vectors;
- do not invent compute gas constants before review;
- do not improvise smart-contract runtime semantics;
- keep AI inference outside consensus execution;
- keep the base chain independent of AI/storage/service availability;
- version consensus-visible changes and update interoperability vectors with them.

## Project direction

NIAHCIA's intended separation is simple:

```text
the blockchain secures ownership, value, contracts, and settlement

the wallet owns the user's AI identity, permissions, and private memory

decentralized machines supply the intelligence
```

That separation is a core design constraint, not just an implementation detail.
