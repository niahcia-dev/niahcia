# Architecture

## System overview

NIAHCIA separates consensus, execution, AI compute, and distributed services.

```text
                        NIAHCIA NETWORK

                     CPU PoW consensus
                            |
                            v
                       EVM execution
                            |
          +-----------------+-----------------+
          |                 |                 |
          v                 v                 v
     AI contracts      Agent registry    Service registry
          |                 |                 |
          +-----------------+-----------------+
                            |
                       P2P control
          +-----------------+-----------------+
          |                                   |
          v                                   v
   Compute workers                      Service nodes
   AI inference                         models / memory
   tools / agents                       artifacts / routing
```

## Network roles

### CPU Miner

- performs CPU-friendly PoW
- proposes blocks
- secures chain history and transaction ordering
- receives block subsidy and transaction fees

### Full Node

- validates PoW
- executes EVM transactions
- verifies state transitions
- does not need to mine or run AI

### Compute Worker

- advertises execution capabilities
- accepts AI jobs
- runs runtimes such as vLLM
- streams responses over the AI P2P network
- publishes signed result commitments
- receives AI job payments

The protocol uses the term **Compute Worker**, not GPU Miner. Future workers may use GPUs, CPUs, multi-accelerator systems, or distributed clusters.

### Service Node

A bonded service provider capable of one or more services:

- model storage and retrieval
- memory storage
- artifact or dataset storage
- routing
- indexing
- archival
- future verification support

Service nodes are paid for measurable service and are not part of base-chain fork choice.

### Agent

A persistent portable network identity composed of:

- stable agent identity
- immutable versioned definitions
- model and execution policy
- capabilities and permissions
- memory policy and state roots
- economic policy
- optional wallet/treasury
- governance controller

No physical machine is "the agent."

## Consensus / EVM boundary

The target architecture uses a custom CPU-PoW consensus client and a mature EVM execution engine.

Working Prototype 0 direction:

- RandomX CPU PoW
- ~30 second block target
- highest cumulative work fork choice
- Reth as EVM execution engine
- authenticated consensus/execution interface

The PoW client owns:

- parent selection
- cumulative work
- difficulty
- timestamps
- PoW header rules
- RandomX mining and validation
- fork choice and reorganizations

The execution engine owns:

- transaction pool
- EVM execution
- gas accounting
- receipts
- logs
- state root
- Ethereum-compatible JSON-RPC

The AI protocol must not be encoded into PoW consensus rules.

## Fast path vs settlement path

Interactive requests use two parallel paths.

```text
FAST PATH

client -> selected worker -> vLLM -> streamed tokens -> client


SETTLEMENT PATH

job commitment
      |
      v
verification / audit
      |
      v
on-chain result commitment
      |
      v
payment settlement
```

A user should not wait for multiple blocks before seeing generated tokens.

## Protocol families

### Chain P2P

- blocks
- headers
- transactions
- fork synchronization

### AI P2P

- worker advertisements
- job announcements
- assignments
- payload retrieval
- token streams
- execution receipts
- audit and challenge traffic

### Storage P2P

- manifests
- chunk discovery
- chunk transfer
- replication
- availability proofs

These may share transport and identity primitives while remaining logically independent protocols.
