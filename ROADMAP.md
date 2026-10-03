# NIAHCIA Roadmap

NIAHCIA is developed in small, reproducible milestones. The base chain must never depend on AI-worker or storage availability.

## Milestone 0 — Foundation

- protocol/object model and deterministic serialization;
- CPU-PoW chain identity and block-header foundation;
- repository family and development doctrine;
- threat model and interoperability-vector framework.

## Milestone 1 — Native CPU-PoW chain

Implemented or active devnet work includes RandomX CPU PoW, cumulative-work fork choice/reorg handling, NativeTransaction V1, NativeStateV1, deterministic native execution/state roots, native mining RPC, P2P Version 3, persistent block/body/execution/state storage, and restart/synchronization recovery.

Before public testnet: finalize network/genesis parameters, difficulty/timestamp rules, miner/pool interoperability, monetary constants, operational hardening, and broader devnet testing.

## Milestone 2 — Native compute settlement

- NativeStateV2 account + ComputeChannel state;
- NativeTransaction V2;
- ComputeChannelOpen / Settle / Refund;
- deterministic transitions and persistence;
- explicit activation boundary;
- reviewed native fee/gas schedule.

Compute actions remain inactive until vectors, persistence/reorg behavior, fee rules, and activation rules are complete.

## Milestone 3 — Decentralized AI compute

- signed worker advertisements;
- wallet-local worker discovery/selection;
- bounded sticky compute sessions;
- replaceable runtime execution such as vLLM;
- off-chain job transport and token streaming;
- cumulative signed usage receipts;
- ComputeChannel settlement;
- no centralized scheduler requirement.

## Milestone 4 — Wallet-native Agents

- wallet-controlled AI identity;
- chat encryption keys separated from spend keys;
- local encrypted chat history and Agent memory by default;
- bounded portal/session capabilities;
- portable Agent manifests, versions, permissions, budgets, and workflows.

## Milestone 5 — Optional service/storage network

- decentralized storage agreements;
- content-addressed data;
- replication and retrieval;
- availability/proof-of-service evidence;
- recovery and migration;
- no consensus authority for service/storage nodes.

## Milestone 6 — Public network experience

Wallet, explorer, miner, compute-worker software, niahcia.com portal, developer tools, reproducible releases, public testnet, then mainnet only after explicit protocol freeze criteria.
