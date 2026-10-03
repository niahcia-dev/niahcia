# NIAHCIA

**AI CHAIN — reversed.**

NIAHCIA is a pre-alpha permissionless CPU-PoW blockchain designed to support decentralized AI compute without making AI infrastructure part of consensus.

> **Status:** pre-alpha. There is no production network or public binary release yet.

## Architecture

```text
CPU PoW blockchain
  consensus / balances / transactions / commitments / settlement
        |
        +-- wallet-controlled Agents and chat
        +-- decentralized AI compute workers
        +-- optional storage/service providers
```

Core rules:

- CPU Proof-of-Work is the sole chain-consensus authority.
- AI workers, storage nodes, and service nodes receive no fork-choice or finality authority.
- The base chain must remain valid and usable if every AI worker or storage provider disappears.
- The active reference node uses native NIAHCIA transactions, execution, state, persistence, mining RPC, and P2P.
- Wallet/client-local encrypted chat history and Agent memory are the V1 default.
- NativeTransaction V2 / NativeStateV2 compute-channel work is currently inactive and not accepted by active mempool/P2P/mining.

## Current implementation

The active devnet path includes RandomX CPU PoW, the 164-byte `BlockHeaderV1`, cumulative-work fork choice, NativeTransaction V1 transfer execution, NativeStateV1 snapshots/state roots, native block-body persistence, P2P Version 3, a shared native mempool, restart-safe recovery, and mining work/submission RPC.

Inactive development work includes `ComputeChannelOpen`, `ComputeChannelSettle`, and `ComputeChannelRefund`. No compute intrinsic-gas constants or V2 activation height are currently assigned.

## Releases

There is no public binary release yet. When binary releases begin, this repository's [Releases](https://github.com/niahcia/niahcia/releases) page will be the canonical distribution point. Release policy and expected assets are documented in [RELEASES.md](RELEASES.md).

## Quick links

- [All releases](https://github.com/niahcia/niahcia/releases)
- [Project site](https://niahcia.github.io/)
- [Protocol mirror](https://github.com/niahcia/niahcia-protocol)
- [Miner](https://github.com/niahcia/niahcia-miner)
- [Compute worker](https://github.com/niahcia/niahcia-compute)
- [Explorer](https://github.com/niahcia/niahcia-explorer)
- [Web application](https://github.com/niahcia/niahcia-web)

## Development

```bash
git clone https://github.com/niahcia/niahcia.git
cd niahcia
cargo build --release
cargo run -p niahcia
```

Start new development sessions with [docs/CURRENT-WORK.md](docs/CURRENT-WORK.md). Protocol and implementation status are tracked in [docs/spec-status.md](docs/spec-status.md).

## Repository role

This repository is the canonical working repository for the reference node plus consolidated protocol/spec/test-vector material during pre-alpha development. `niahcia-protocol` remains a synchronized protocol mirror during the transition.
