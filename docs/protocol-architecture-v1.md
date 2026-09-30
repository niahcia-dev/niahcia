# NIAHCIA Protocol Architecture V1

> **Protocol source of truth:** the authoritative architecture map now lives in [`niahcia/niahcia-protocol`](https://github.com/niahcia/niahcia-protocol/blob/main/docs/protocol-architecture-v1.md).

This repository is the Rust reference implementation of NIAHCIA. Its `docs/` directory is for implementation-facing material such as building/running the node, devnet operations, RPC behavior, implementation milestones, debugging, and implementation-specific design notes.

Normative protocol definitions, candidate protocol designs, canonical encodings, improvement proposals, and cross-implementation test vectors belong in `niahcia/niahcia-protocol`.

## Documentation rule

When an implementation change alters protocol behavior:

1. update the affected specification in `niahcia-protocol`;
2. update/add interoperability vectors when the change is consensus- or wire-critical;
3. update this implementation and its implementation-facing documentation;
4. explicitly mark or remove superseded behavior rather than leaving contradictory rules.

Implementation code does not silently redefine a locked protocol specification. If the implementation and specification disagree, the discrepancy must be resolved explicitly.

## Current implementation architecture

The reference node currently implements or is developing the following major boundaries:

```text
NIAHCIA consensus daemon
        |
        +-- RandomX PoW / cumulative-work chain selection
        +-- chain P2P / synchronization / reorg handling
        +-- native RPC and mining RPC
        |
        +-- authenticated Reth Engine API boundary
                    |
                    +-- EVM execution and state
```

AI compute, agent, verification, and service/storage protocols are defined at the protocol level and will be integrated into the reference implementation incrementally. They do not become chain-consensus authorities merely because this implementation hosts components for them.

## Current interoperability note

Stock common RandomX miner compatibility—especially stock XMRig through pool-facing interoperability—is an explicit protocol objective. The exact XMRig-compatible PoW preimage is still under review and is **not locked by this implementation document**.

See the authoritative protocol architecture and future mining-interoperability specification in `niahcia-protocol` before treating a development mining blob as a production contract.
