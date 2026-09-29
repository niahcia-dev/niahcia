# Prototype 0 Implementation Plan

## Objective

Build the smallest complete NIAHCIA network that proves:

1. CPU-secured EVM chain operation across independent nodes.
2. Decentralized AI job execution without a central scheduler.
3. Independent compute-worker settlement.
4. Independent service-node model distribution.
5. End-to-end failure tolerance when individual miners, workers, or service nodes disappear.

## Workstream A — PoW + EVM

Deliverables:

- RandomX integration
- PoW block header and validation rules
- ~30 second target block time
- difficulty adjustment
- highest cumulative work fork choice
- PoW consensus daemon
- authenticated interface to Reth
- block template creation
- mined payload submission
- reorganization handling
- Ethereum-compatible JSON-RPC exposed by the execution node

The PoW daemon owns consensus. Reth owns EVM execution and state.

## Workstream B — Smart contracts

Initial contracts:

- ModelRegistry
- WorkerRegistry
- ServiceRegistry
- AIJobs

Prototype requirements:

- model and execution-profile registration
- worker registration and test bonds
- service-node registration and test bonds
- AI job escrow
- deterministic worker assignment hooks
- result commitment submission
- commit/reveal verification
- final result commitment
- compute and service settlement

## Workstream C — Compute worker

The first worker daemon must:

- create/load a network identity
- register supported execution profiles
- advertise availability over P2P
- receive assignments
- retrieve the canonical input payload
- verify model/runtime/profile hashes
- execute via vLLM
- stream tokens to the requester
- construct canonical token-sequence commitments
- submit signed result commitments
- expose observed performance metrics

## Workstream D — Service node

The first service node must:

- register its identity
- ingest the canonical model manifest
- store model chunks
- advertise available manifests
- serve chunks over P2P
- verify content hashes
- respond to basic availability challenges
- report retrieval accounting for test settlement

## Workstream E — P2P

Logical protocols:

### Chain P2P
Blocks, headers, transactions, synchronization.

### AI P2P
Worker advertisements, jobs, assignments, input retrieval, streaming, results.

### Storage P2P
Model manifests, chunk discovery, chunk transfer, availability.

Where practical, identity and transport primitives should be shared without merging protocol semantics.

## Acceptance topology

```text
Chain:
  Node A — miner/full node
  Node B — miner/full node
  Node C — miner/full node

Compute:
  Worker D
  Worker E
  Worker F

Service:
  Service G
  Service H
```

A user submits a job, receives a streamed response, verification finalizes, and payments settle while CPU block rewards remain economically separate.

## Failure tests

- Stop one miner: chain continues.
- Stop one compute worker: job can still succeed or reassign.
- Stop one service node: canonical model remains retrievable.
- Stop the original bootstrap/developer node: established peers continue.
- Restart participants in a different order: network state recovers correctly.

## Explicit non-goals for the first milestone

These are deferred implementation tasks, not excluded protocol capabilities:

- production monetary policy
- distributed training
- private inference
- zkML
- autonomous agent spending
- persistent global agent memory
- production slashing
- multi-GPU distributed inference
- polished web UI
