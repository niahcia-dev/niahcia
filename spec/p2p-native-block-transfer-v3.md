# NIAHCIA P2P Native Block Transfer V3

## Status

Candidate — next wire milestone for non-empty native transaction blocks.

## Purpose

P2P Version 3 extends the header-only Version 2 block transfer with the exact canonical native transaction body and CPU producer fee recipient required for independent validation of non-empty blocks.

The 164-byte `BlockHeaderV1` remains unchanged.

## Version boundary

P2P Version 2 remains the header-only empty-block development protocol.

P2P Version 3 is an incompatible wire version and MUST NOT reinterpret V2 block frames.

A V3 peer MUST validate a V3 block from locally trusted parent state rather than trusting peer-supplied execution/state.

## BlockTransferV3

One transferred block contains:

```text
BlockTransferV3
- header: BlockHeaderV1
- producer_fee_recipient: 20 bytes
- transaction_count: u32
- transactions[]: canonical SignedNativeTransaction bytes
```

Each transaction element is encoded as:

```text
transaction_length_u32_be
transaction_bytes[transaction_length]
```

The transaction bytes are the complete canonical NCE serialization of the applicable SignedNativeTransaction schema.

No transaction is represented by JSON, txid-only data, or a peer-local encoding.

## Batch framing

A V3 Blocks message contains:

```text
block_count_u16_be
repeated BlockTransferV3
```

Protocol bounds MUST cap:

- blocks per message;
- transactions per block;
- transaction byte length;
- total block-body bytes;
- total frame bytes.

Exact limits are implementation/network parameters and require tests before lock.

## Transaction ordering

Transaction order in `transactions[]` is consensus-significant for that block.

The sender/miner may choose any order that produces a valid deterministic native transition.

Validators MUST execute exactly the transported order.

The protocol does not require global mempool ordering consensus.

The header's existing `transactions_root` authenticates the exact canonical ordered transaction bytes using the active transaction commitment rules.

## Producer fee recipient

`producer_fee_recipient` is the native 20-byte Account payload selected by the block producer for native priority-fee credit.

Validation executes the exact block transaction sequence using:

```text
NativeExecutionContext {
    cpu_producer = producer_fee_recipient
}
```

and requires the recomputed execution/state/receipt commitments to match the header.

### Zero-priority-fee canonicality

If deterministic execution reports:

```text
total_producer_priority_fee == 0
```

then the canonical V3 body MUST encode:

```text
producer_fee_recipient = 20 zero bytes
```

A block body with a nonzero recipient and zero aggregate priority fee is non-canonical/invalid under V3.

This prevents multiple economically identical fee-recipient bodies for the same zero-fee header.

### Nonzero priority fees

If:

```text
total_producer_priority_fee > 0
```

the recipient may be any valid 20-byte Account payload selected by the producer.

Changing that recipient changes the resulting account state and therefore must cause the recomputed state/execution commitment to differ except through a cryptographic collision.

A peer cannot redirect producer fees without invalidating the header's execution commitment.

## Independent validation

For every V3 block, a node MUST:

1. validate V3 framing/canonical bounds;
2. validate the 164-byte BlockHeaderV1 and PoW;
3. load the locally persisted parent native state;
4. strictly decode every transported SignedNativeTransaction;
5. recompute the ordered transactions_root and require header equality;
6. execute the ordered transactions using the transported producer fee recipient;
7. apply the zero-priority-fee recipient canonicality rule;
8. require resulting execution_root to equal the header;
9. require the resulting state root and receipts root to match the execution result;
10. atomically persist header, ordered transaction body/reference, execution result, resulting native state, and cumulative-work/fork-choice metadata;
11. update mempool state for transactions accepted into the canonical chain.

The peer does not provide an authoritative state snapshot.

## Block body persistence

A full node MUST retain enough canonical transaction-body data to:

- serve the block to another V3 peer;
- recompute transactions_root;
- independently replay/validate block execution;
- recover correctly after restart/reorg.

A header-only persisted representation is insufficient for non-empty V3 blocks.

The canonical body may be stored directly or through an integrity-checked content-addressed internal representation, but it must be reconstructable exactly.

## Reorg behavior

When the canonical chain changes:

- transactions from newly attached blocks are removed from local mempool if present;
- valid transactions from detached blocks MAY be reconsidered for mempool admission if they are not included on the winning branch and remain valid against the new state;
- account/state rollback comes from canonical parent/branch state restoration;
- a transaction's original block position does not grant priority in the new mempool.

Mempool reinsertion is policy, not consensus.

## P2P transaction relay

V3 SHOULD add direct native transaction relay separately from block transfer.

Minimal messages:

```text
TxInvV1
- tx_ids[]

GetTxV1
- tx_ids[]

TxV1
- canonical_signed_transactions[]
```

A peer receiving a transaction:

1. applies frame/size limits;
2. rejects already-known txids;
3. strictly decodes canonical SignedNativeTransaction bytes;
4. applies NativeMempool admission policy;
5. may relay inventory to peers.

The wire does not invent a second transaction encoding.

## Relay anti-amplification

Peers SHOULD announce txids before sending large transaction bodies unless directly responding to an explicit request.

Implementations need bounded inventory counts, requested-object tracking, duplicate suppression, and per-peer resource limits.

Detailed peer scoring/ban policy is not consensus.

## Mining template interaction

A mining template contains an exact ordered transaction set and one producer fee recipient chosen locally by the mining node.

The node executes that exact template before exposing PoW work, producing:

```text
transactions_root
execution_root
```

for the header.

The miner searches nonce/extra_nonce only. It MUST NOT alter transactions or producer fee recipient without receiving a new template/generation.

Once solved, the node persists and relays the same body used to construct the solved header.

## Mempool selection

The first implementation may use a deliberately simple deterministic local template policy, for example:

- inspect mempool in txid order;
- include only transactions executable in sequence against the evolving candidate state;
- stop at configured block transaction/byte/gas limits.

This is local mining policy, not network consensus. Another producer may choose another valid order/set.

No replacement-by-fee rule is required for the first non-empty block milestone.

## Version 2 compatibility

P2P V2 nodes cannot validate non-empty V3 bodies and therefore MUST NOT participate as validating peers after the network activates non-empty V3 block requirements.

A devnet transition may be a clean protocol-version bump rather than backward-compatible multiplexing.

## Invariants

1. BlockHeaderV1 remains exactly 164 bytes.
2. V3 transports exact canonical transaction bytes.
3. Transaction order is authenticated by transactions_root.
4. Producer fee recipient is part of the canonical block body.
5. Zero total priority fee requires zero producer recipient.
6. Nonzero fee recipient is validated through deterministic state/execution recomputation.
7. Peers never trust peer-supplied post-state.
8. Non-empty block bodies are persistently reconstructable.
9. Transaction relay uses SignedNativeTransaction canonical bytes.
10. Mempool ordering/replacement is policy, not consensus.
11. Miner nonce search cannot mutate the already committed template body.

## First milestone tests

1. one valid non-empty transfer block syncs between two nodes;
2. transported tx root mismatch is rejected;
3. altered tx order with old header is rejected;
4. malformed/noncanonical transaction bytes are rejected;
5. wrong producer fee recipient on a fee-bearing block is rejected by execution-root mismatch;
6. nonzero recipient on a zero-priority-fee block is rejected;
7. restart can re-serve and replay exact non-empty block body;
8. detached-block transaction may be reconsidered for mempool;
9. attached-block transaction is removed from mempool;
10. V2 framing is not accepted as V3.
