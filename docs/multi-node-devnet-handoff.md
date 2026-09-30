# v0.3.0 Multi-Node Devnet Handoff

This document defines the next implementation slice after the single-node persisted/replay devnet work.

The goal is deliberately narrow: prove that two independent NIAHCIA nodes can synchronize and independently validate the same CPU-PoW chain while each uses its own Reth execution node.

## Starting point

The reference node already has:

- canonical `BlockHeaderV1` encoding and block IDs
- RandomX PoW validation
- cumulative-work fork choice and reorg accounting
- persistent chain state
- block-to-execution-hash mappings
- persisted execution replay payloads
- atomic mined-block/execution-mapping persistence
- Reth Engine API payload validation
- restart recovery and forced execution replay smoke tests

There is currently no native NIAHCIA peer transport.

## First networking acceptance target

Run two independent stacks:

```text
Node A: NIAHCIA A <-> Reth A
             |
       NIAHCIA P2P
             |
Node B: NIAHCIA B <-> Reth B
```

Mine blocks through Node A. Node B must fetch them over the NIAHCIA peer protocol, independently validate NIAHCIA consensus, independently validate/replay the mapped execution payload through Reth B, persist the result, and reach the same canonical NIAHCIA head.

Then stop and restart Node B. It must recover and continue synchronization without copying Node A's database.

## Static devnet configuration

Add only:

```text
NIAHCIA_P2P_BIND
NIAHCIA_P2P_PEERS
```

`NIAHCIA_P2P_BIND` is the local TCP listen address.

`NIAHCIA_P2P_PEERS` is a comma-separated list of static peer socket addresses for the devnet milestone.

No peer discovery is required for v0.3.0's first slice.

## Wire framing

Use a small versioned binary frame rather than serializing Rust structs directly.

Every frame begins with:

```text
magic            4 bytes
protocol_version u16 big-endian
message_type     u16 big-endian
payload_length   u32 big-endian
payload          payload_length bytes
```

Requirements:

- network magic is fixed for the NIAHCIA devnet
- protocol version starts at 1
- reject unknown network magic
- reject unsupported protocol versions
- enforce a conservative maximum frame size before allocation
- all integer encodings are explicitly big-endian
- consensus block headers use the existing canonical `BlockHeaderV1` bytes

The exact devnet magic and numeric message IDs should be locked by protocol-vector tests when the implementation lands.

## Protocol V1 messages

Keep the first message set minimal.

### Hello

Sent immediately after connection.

Fields:

- network identifier
- protocol version
- best height, if any
- best block ID, if any
- cumulative work of best head

A peer on a different network or incompatible protocol version is disconnected.

### GetBlocks

Requests canonical blocks beginning at a height with a bounded count.

Fields:

- start height
- count

The receiver must cap `count` to the protocol maximum.

### Blocks

Returns zero or more canonical block transfer records in ascending height order.

Each transfer record contains:

- canonical `BlockHeaderV1` bytes
- execution payload hash
- persisted replay payload bytes

The sender's cumulative-work value is not trusted. The receiver derives chain work locally.

## Receive validation order

For every received block, fail closed in this order:

1. Decode bounded wire data.
2. Decode canonical `BlockHeaderV1`.
3. Verify the block ID from canonical header bytes.
4. Verify expected parent and height.
5. Verify timestamp/difficulty/target consensus rules.
6. Verify RandomX PoW locally.
7. Verify the execution replay record is bound to the advertised execution hash.
8. Submit/replay the execution payload through the receiver's own Reth.
9. Require Reth `VALID` and matching `latestValidHash`.
10. Persist replay payload, block, execution mapping, and resulting chain state.
11. If the accepted block changes the best NIAHCIA head, update local Reth forkchoice to the execution hash mapped to that head.

Never copy another peer's redb database, persisted cumulative-work value, or best-head metadata.

## Initial synchronization

After Hello:

- compare local and remote advertised best work/height only as a synchronization hint
- request bounded canonical ranges from the next locally needed height
- validate every returned block independently
- continue until no newer canonical block is advertised

The peer advertisement is not consensus truth. Local validation and cumulative-work fork choice remain authoritative.

A later slice can add block locators for efficient fork discovery. The first two-node test may synchronize a node with no existing chain or a shared canonical prefix.

## Concurrency boundary

For the first slice:

- one listener thread may accept inbound TCP connections
- one worker per peer is sufficient
- use blocking I/O with read/write timeouts
- bound frame size, block batch size, and connection resource use

Do not introduce an async runtime solely for this milestone unless the blocking implementation demonstrates a concrete limitation.

## Required tests before calling the first slice complete

Unit/protocol tests:

- locked Hello V1 vector
- locked GetBlocks V1 vector
- locked Blocks V1 vector
- malformed/truncated/oversized frame rejection
- wrong-network rejection
- unsupported-version rejection
- block batch count limit
- received block with invalid parent rejected
- received block with invalid PoW rejected
- received block with mismatched execution hash/replay payload rejected

Two-node smoke test:

1. Start Reth A and Reth B with independent data directories.
2. Start NIAHCIA A and NIAHCIA B with independent state directories.
3. Connect them through static devnet peer configuration.
4. Mine at least three blocks through A.
5. Require B to reach A's canonical NIAHCIA block ID and height.
6. Require Reth B to contain and canonicalize the execution block mapped to B's NIAHCIA tip.
7. Stop B.
8. Mine at least one additional block through A.
9. Restart B.
10. Require B to catch up without copying state from A.
11. Require both NIAHCIA nodes and both Reth nodes to agree on their respective canonical tips.

## Explicitly deferred

Do not add these while proving the first two-node milestone:

- DNS/bootstrap discovery
- DHT/discv5
- NAT traversal
- gossip mesh
- peer scoring or reputation
- persistent peer database
- complex ban management
- transaction relay
- AI-worker networking
- storage-node networking
- service-node discovery
- encrypted application-layer sessions beyond what is justified by a concrete threat model
- public-internet hardening

These are future requirements, not requirements for proving basic independent chain synchronization.

## Tomorrow's implementation order

1. Add P2P configuration fields and validation.
2. Add `p2p.rs` with bounded frame codec and protocol V1 message types.
3. Lock protocol vectors and rejection tests.
4. Add static listener/dialer and Hello exchange.
5. Add canonical range serving.
6. Add independently validated block ingestion.
7. Add synchronization loop.
8. Add the two-node smoke harness.
9. Run CI.
10. Run the two-node devnet locally and fix only failures exposed by that test.

Do not expand scope until the two-node acceptance target passes.
