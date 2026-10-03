# Reference Implementation Plan

## Objective

Build the smallest complete NIAHCIA system that proves a CPU-PoW blockchain can validate and settle value independently of AI infrastructure, while wallets can purchase decentralized compute without giving a worker custody of wallet funds.

## Native chain

Current implementation direction:

- RandomX CPU PoW;
- 164-byte BlockHeaderV1;
- cumulative-work fork choice;
- native transactions and deterministic native execution;
- native state roots/snapshots;
- native mining RPC;
- P2P Version 3;
- atomic persistence and restart/reorg recovery.

The former Reth/EVM execution subsystem has been removed from the active reference node.

## Smart contracts

Smart contracts are a required part of NIAHCIA's native execution architecture. NativeTransaction V1 already defines `ContractCall` and `ContractCreate`, but the active reference node deliberately rejects/does not execute their runtime semantics until the native contract runtime is specified and implemented coherently.

The runtime must define deterministic code format, contract state/storage, create/call/revert behavior, value movement, gas/resource metering, receipts/commitments, persistence/reorg behavior, canonical vectors, and activation/versioning. It must not expose nondeterministic host facilities or perform AI inference during consensus execution.

## Compute settlement

NativeStateV2 adds consensus-committed ComputeChannel state. NativeTransaction V2 defines explicit ComputeChannelOpen, ComputeChannelSettle, and ComputeChannelRefund actions.

The first implementation proves these transitions while they are inactive. Activation requires explicit network parameters, fee/gas rules, vectors, persistence/reorg coverage, and no regression to V1.

## AI compute

Workers advertise signed expiring capabilities, accept wallet-selected bounded sessions, execute through a replaceable runtime such as vLLM, stream results directly to the client, exchange signed usage evidence off-chain, and settle through ComputeChannels.

The chain does not execute AI inference.

## Wallet / Agent

The wallet is the V1 Agent home. Spending authority remains distinct from chat encryption; conversation content is encrypted by default; local memory/history is the default persistence model; portals receive bounded delegated authority.

## Optional storage/services

Decentralized storage is deferred from the first ordinary AI milestone. Service providers never gain PoW fork-choice/finality authority.

## Acceptance discipline

Each substantial implementation slice requires deterministic tests, canonical vectors when wire/consensus behavior changes, explicit negative paths, restart/reorg coverage where persistent state changes, synchronized documentation, and green formatting/build/tests/Clippy.
