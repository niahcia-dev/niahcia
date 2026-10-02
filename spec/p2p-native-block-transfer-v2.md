# NIAHCIA P2P Native Block Transfer V2

## Status

Candidate — development wire protocol for the current native-execution milestone.

This document defines the current Version 2 native block-transfer behavior. It is intentionally narrower than a complete future P2P specification.

## Version transition

P2P protocol version 2 replaces the legacy block-transfer format that carried an external execution hash and replay payload.

The incompatible change is deliberate. Version 1 block-transfer framing MUST NOT be reinterpreted as Version 2.

## BlockTransferV2

A Version 2 block-transfer item contains only one canonical `BlockHeaderV1`.

For the current implementation, a block batch is encoded as:

1. an unsigned 16-bit block count;
2. exactly that many canonical 164-byte `BlockHeaderV1` values.

The transfer does not carry:

- an external execution hash;
- a replay journal;
- a peer-supplied native state snapshot;
- a trusted execution result.

## Independent validation

For each received header, a validating node MUST independently:

1. resolve the required RandomX seed from local canonical ancestry rules;
2. verify the candidate PoW and header-level consensus rules;
3. load the locally persisted parent `NativeStateV1` snapshot, or the canonical empty genesis state when validating genesis;
4. execute the transaction set defined by the active block-transfer milestone;
5. require the resulting transaction, state, receipt, and execution commitments to match the header;
6. atomically persist the validated block, complete native execution result, and resulting native state;
7. apply cumulative-work fork choice and the deterministic equal-work tie-break.

A peer-provided state snapshot is never chain authority.

## Current empty-block limitation

Version 2 currently transports headers only. Therefore the present implementation is valid only for the development milestone in which networked blocks contain the canonical empty native transaction set.

For an empty transaction set:

- transaction priority fees are zero;
- the CPU producer recipient does not affect the resulting execution commitment;
- independent reconstruction from parent state is deterministic.

This limitation MUST be removed before non-empty native transaction blocks are relayed between peers.

## Required next versioned extension

Before fee-bearing or otherwise non-empty transaction blocks become network-valid, the protocol MUST define and implement:

- canonical transaction/block-body transport;
- transaction propagation or retrieval;
- deterministic transaction ordering/inclusion rules;
- a producer-fee recipient identity that every validating peer can derive or verify;
- block reconstruction and commitment validation from the transported transaction body.

A peer MUST NOT be allowed to make a self-consistent block valid merely by supplying its own claimed post-state.

## Compatibility rule

The 164-byte `BlockHeaderV1`, CPU-PoW cumulative-work fork choice, canonical transaction commitment, and native execution commitments remain separate protocol surfaces. A future transaction-body transport extension does not silently change those locked or candidate formats.


## Successor

`spec/p2p-native-block-transfer-v3.md` defines the candidate full-body successor for non-empty native transaction blocks, including canonical transaction transport, transaction relay direction, and producer-fee-recipient validation.

V2 remains the header-only empty-block development protocol and is not silently extended.
