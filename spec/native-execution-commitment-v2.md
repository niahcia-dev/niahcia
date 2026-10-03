# Native Execution Commitment V2

Status: **CANDIDATE / REVIEW REQUIRED / INACTIVE**

## Purpose

Native Execution Commitment V2 is the successor execution-result boundary for a versioned NativeBlockBodyV2 and NativeStateV2.

It does **not** change the locked 164-byte BlockHeaderV1 and it does not activate NativeTransaction V2, ComputeChannel actions, or smart-contract execution.

The active devnet continues to use Native Execution V1 until an explicit activation boundary exists.

## Receipt V2

Each ordered receipt commits:

1. signed native transaction schema version;
2. native action number;
3. transaction ID;
4. post-transaction NativeStateV2 root;
5. gas used;
6. effective fee per gas;
7. base fee burned;
8. producer priority fee.

The receipt commitment domain is:

```text
NIAHCIA/NATIVE-RECEIPT/V2
```

Receipt V2 intentionally includes the post-transaction state root so the ordered receipt chain commits the exact state reached after each accepted transaction.

## Compute gas placeholder rule

ComputeChannel gas/fee constants are not defined.

For the current inactive execution proof only, ComputeChannel receipts carry zero for:

- gas used;
- effective fee per gas;
- base fee burned;
- producer priority fee.

The inactive V2 block executor separately requires zero transaction gas/fee placeholder fields for ComputeChannel actions.

**Zero is not a production gas schedule.** It is an explicit non-activation placeholder that prevents undefined fee behavior from being mistaken for consensus policy.

## Receipts root

The ordered receipt commitments are hashed under:

```text
NIAHCIA/NATIVE-RECEIPTS-ROOT/V2
```

The empty root uses:

```text
NIAHCIA/NATIVE-RECEIPTS-ROOT/V2/EMPTY
```

## Execution root

The V2 execution root commits, in order:

```text
transactions_root
state_root
receipts_root
gas_used
base_fee_burned
producer_priority_fee
```

under:

```text
NIAHCIA/NATIVE-EXECUTION-ROOT/V2
```

The aggregate state root is the final NativeStateV2 root. For non-empty results it must equal the post-state root in the final receipt.

## Canonical result serialization

The inactive canonical result uses:

```text
u8   version = 2
[32] transactions_root
[32] state_root
[32] receipts_root
[32] execution_root
u64  gas_used
u128 base_fee_burned
u128 producer_priority_fee
u64  receipt_count
receipt_count * NativeReceiptV2
```

All integer fields are big-endian.

Decoding must recompute receipt root, aggregate gas/fee totals, final receipt state root, and execution root. Mismatch is invalid.

## Locked interoperability vector

The canonical vector is stored at:

`test-vectors/native-execution-v2.json`

and is enforced by the Rust test `locked_execution_v2_interoperability_vector`.

The locked vector fixes:

- both Receipt V2 canonical encodings;
- both receipt commitments;
- the ordered receipts root;
- the execution root;
- the complete 451-byte canonical NativeBlockExecutionResultV2 encoding.

Any incompatible change to those bytes or commitments requires an explicit successor version and replacement vectors. The vector lock does not activate V2 execution.

## Activation boundary

This candidate structure is not admitted to active block persistence, P2P, mining, or mempool behavior.

Before activation it still requires:

- explicit block/execution version activation parameters;
- persistence/restart/reorg integration;
- defined ComputeChannel gas/fee rules;
- migration behavior from Native Execution V1;
- review alongside the required native smart-contract runtime.

Smart-contract receipts may extend or succeed this boundary once ContractCall/ContractCreate runtime semantics are specified. This document does not invent those semantics.
