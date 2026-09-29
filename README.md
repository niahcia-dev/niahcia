# NIAHCIA

**AI CHAIN — reversed.**

The main repository for the NIAHCIA network.

> **Status:** pre-alpha. There is no production network or public binary release yet.

## Download

### Current release

**[Download the latest NIAHCIA release](https://github.com/niahcia/niahcia/releases/latest)**

When releases begin, this repository will be the canonical distribution point for normal users and operators.

Planned release assets:

```text
niahcia-node-linux-x86_64.tar.gz
niahcia-miner-linux-x86_64.tar.gz
niahcia-compute-linux-x86_64.tar.gz
niahcia-full-linux-x86_64.tar.gz
SHA256SUMS
```

### Which package do I need?

| Goal | Package |
|---|---|
| Run a full node | `niahcia-node-...` |
| CPU mine | `niahcia-miner-...` |
| Run an AI compute worker | `niahcia-compute-...` |
| Run the standard operator stack | `niahcia-full-...` |

For a normal installation, use a packaged release rather than building from source.

## Quick links

- [All releases](https://github.com/niahcia/niahcia/releases)
- [Download portal](https://niahcia.github.io/downloads.html)
- [Protocol specification](https://github.com/niahcia/niahcia-protocol)
- [Miner source](https://github.com/niahcia/niahcia-miner)
- [Compute worker source](https://github.com/niahcia/niahcia-compute)
- [Explorer source](https://github.com/niahcia/niahcia-explorer)
- [Web application source](https://github.com/niahcia/niahcia-web)

## What is NIAHCIA?

NIAHCIA is a permissionless blockchain and decentralized AI-agent network.

The system is split into independent roles:

- **Full nodes** validate the chain and EVM state.
- **CPU miners** secure consensus.
- **Compute workers** execute AI workloads.
- **Service nodes** provide storage and network services.

CPU mining and AI compute are intentionally separate.

## Source layout

This repository contains the reference blockchain/node implementation and its integration with the EVM execution layer.

Related components live in separate repositories so they can be developed and released independently.

```text
niahcia/niahcia              node / chain
niahcia/niahcia-protocol     protocol specifications
niahcia/niahcia-miner        CPU miner
niahcia/niahcia-compute      AI compute worker
niahcia/niahcia-explorer     explorer
niahcia/niahcia-web          main web application
```

## Development

The first implementation target is:

```text
CPU PoW consensus
      |
      v
Reth / EVM execution
```

Prototype direction:

- RandomX CPU proof of work
- approximately 30-second target block interval
- highest cumulative work fork choice
- Reth for EVM execution
- Solidity smart contracts
- Ethereum-compatible JSON-RPC

Implementation details are in:

- [Prototype 0 implementation plan](docs/implementation.md)
- [PoW ↔ Reth interface](docs/pow-reth-interface.md)
- [Repository family](docs/repository-family.md)
- [Roadmap](ROADMAP.md)

## Build from source

Source build instructions will be added once the first runnable node implementation exists.

Until then, this repository is not a usable end-user release.
