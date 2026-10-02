# NativeBlockBodyV1

Status: **CANDIDATE / pre-alpha canonical non-header block body**

## Purpose

`NativeBlockBodyV1` is the canonical non-header data required to independently validate a native NIAHCIA block with transactions.

It complements the fixed 164-byte `BlockHeaderV1`.

## Object type

```text
0x0017  NativeBlockBody
```

Schema version:

```text
1
```

## Canonical fields

```text
1   producer_fee_recipient
2   transactions
```

`producer_fee_recipient` is exactly 20 bytes.

`transactions` is an ordered array of byte strings. Each element contains the complete canonical NCE serialization of one SignedNativeTransaction object.

Array order is consensus-significant.

## Why the header remains unchanged

The header already commits:

- ordered transaction content through `transactions_root`;
- deterministic execution through `execution_root`.

NativeBlockBodyV1 supplies the exact bytes needed to recompute those commitments.

The body does not add another consensus authority.

## Validation

A validator:

1. strictly decodes NativeBlockBodyV1;
2. strictly decodes every embedded signed transaction;
3. recomputes the ordered transaction commitment;
4. executes the exact transactions against parent native state using `producer_fee_recipient`;
5. applies the zero-priority-fee canonical recipient rule from P2P V3;
6. requires all header commitments to match.

## Producer fee recipient

If aggregate producer priority fees are zero, the canonical body MUST use twenty zero bytes.

If aggregate producer priority fees are nonzero, the body recipient is the address used during deterministic execution.

## Empty body

For an empty transaction set with zero producer priority fees, the one canonical body is:

```text
producer_fee_recipient = 20 zero bytes
transactions = []
```

P2P V2 did not carry this body because it reconstructed the same empty execution implicitly.

P2P V3 carries/derives the V1 body explicitly.

## Persistence

A full node validating P2P V3 blocks MUST persist exact NativeBlockBodyV1 canonical bytes or an internal representation capable of reproducing them byte-for-byte.

Header-only persistence is insufficient once non-empty blocks are valid.

## Block identity

`BlockHeaderV1.block_id` remains the canonical block identifier.

NativeBlockBodyV1 does not create a second competing block ID.

A body that does not reproduce the header commitments is invalid for that block header.

## Invariants

1. BlockHeaderV1 remains 164 bytes.
2. Body transactions are exact canonical signed transaction bytes.
3. Transaction order is preserved.
4. Body recipient is used in native execution.
5. Zero-priority-fee blocks have a unique zero recipient body.
6. Body bytes are persistently reconstructable.
7. Peer-supplied body never overrides header commitments.
