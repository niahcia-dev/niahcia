# NIAHCIA

**AI CHAIN — reversed.**

This repository contains the reference implementation of the NIAHCIA network.

NIAHCIA is a permissionless decentralized blockchain and AI-agent network designed around four independent roles:

- **CPU miners** secure consensus.
- **Full nodes** validate PoW and EVM state.
- **Compute workers** execute AI workloads.
- **Service nodes** provide storage and network services.

The protocol specification lives in `niahcia-dev/niahcia-protocol`.

## Working Prototype 0 stack

- CPU PoW: RandomX
- Block target: approximately 30 seconds
- Fork choice: highest cumulative work
- EVM execution: Reth
- Smart contracts: Solidity / EVM
- AI runtime: vLLM
- Initial workload: pinned Qwen3-class ~8B text model
- Initial verification: deterministic redundant 2-of-3, with interfaces designed for later optimistic verification
- Service nodes: canonical model replication and retrieval
- AI and storage payloads: P2P and content-addressed
- Settlement: on-chain

## Architectural boundary

The PoW consensus layer must not know what an AI job is.

```text
CPU PoW consensus
        |
        v
    EVM state
        |
        +--> AI contracts / registries
        +--> service-node contracts / registries
        +--> normal Solidity applications
```

AI runtimes, models, verification policies, and storage mechanisms may evolve without changing base PoW consensus rules.

## Planned components

```text
crates/
  consensus/       CPU PoW consensus and fork choice
  pow-rpc/         consensus <-> execution interface
  node/            chain node orchestration
  p2p/             shared peer/network primitives

ai/
  worker/          compute worker daemon
  protocol/        AI P2P messages and job transport

service/
  node/            service-node daemon
  protocol/        storage/retrieval protocol

contracts/
  model-registry/
  worker-registry/
  service-registry/
  ai-jobs/

docs/
  implementation.md
```

The repository layout will evolve as the first implementation work begins.

## Status

Pre-alpha / architecture bootstrap. No production network exists yet.
