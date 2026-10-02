# NIAHCIA Native Execution V1

Status: consensus format under implementation

## Scope

Native Execution V1 defines deterministic execution commitments for ordered
NIAHCIA native transactions.

The block header remains unchanged. Its `transactions_root` commits to the
ordered canonical signed native transactions. Its `execution_root` commits to
the deterministic result of executing those transactions.

## Transfer gas

A successful native Transfer V1 consumes exactly 1,000 gas.

A transaction may authorize a larger `gas_limit`, but unused gas capacity is
not charged.

## Fee calculation

For each transaction:

    max_execution_charge =
        gas_limit * max_fee_per_gas

    required_balance =
        value + max_execution_charge

The sender MUST have at least `required_balance` before execution.

The transaction is invalid when:

    max_fee_per_gas < base_fee_per_gas

Priority fee per gas is:

    min(
        max_priority_fee_per_gas,
        max_fee_per_gas - base_fee_per_gas
    )

Effective fee per gas is:

    base_fee_per_gas + priority_fee_per_gas

For a successful transaction:

    base_fee_burned =
        gas_used * base_fee_per_gas

    producer_priority_fee =
        gas_used * priority_fee_per_gas

    actual_fee =
        base_fee_burned + producer_priority_fee

    unused_fee_reserve =
        max_execution_charge - actual_fee

The base fee is burned.

The priority fee is transferred to the canonical CPU-PoW block producer and
does not create new supply.

## NativeReceiptV1

A successful transaction produces one receipt.

Fields, in commitment order:

1. transaction_id: 32 bytes
2. gas_used: unsigned 64-bit integer
3. effective_fee_per_gas: unsigned 128-bit integer
4. base_fee_burned: unsigned 128-bit integer
5. producer_priority_fee: unsigned 128-bit integer

Integer fields are encoded unsigned big-endian at their exact widths.

The receipt commitment is:

    Keccak256(
        "NIAHCIA/NATIVE-RECEIPT/V1" ||
        transaction_id ||
        gas_used_u64_be ||
        effective_fee_per_gas_u128_be ||
        base_fee_burned_u128_be ||
        producer_priority_fee_u128_be
    )

## Receipts root

For a non-empty ordered receipt list:

    receipts_root =
        Keccak256(
            "NIAHCIA/NATIVE-RECEIPTS-ROOT/V1" ||
            receipt_commitment_0 ||
            receipt_commitment_1 ||
            ...
        )

Receipt ordering is consensus-significant.

For an empty receipt list:

    receipts_root =
        Keccak256(
            "NIAHCIA/NATIVE-RECEIPTS-ROOT/V1/EMPTY"
        )

## Native execution root

The execution root commits to:

1. transactions_root: 32 bytes
2. resulting state_root: 32 bytes
3. receipts_root: 32 bytes
4. total gas_used: unsigned 64-bit integer
5. total base_fee_burned: unsigned 128-bit integer
6. total producer_priority_fee: unsigned 128-bit integer

Integer fields are unsigned big-endian at their exact widths.

The execution root is:

    Keccak256(
        "NIAHCIA/NATIVE-EXECUTION-ROOT/V1" ||
        transactions_root ||
        state_root ||
        receipts_root ||
        gas_used_u64_be ||
        base_fee_burned_u128_be ||
        producer_priority_fee_u128_be
    )

## Block execution

Transactions execute strictly in their canonical block order.

All transaction state transitions and aggregate accounting are checked.

Block execution is atomic.

If any transaction fails, or any aggregate arithmetic operation fails, the
entire block transition fails and canonical native state MUST remain exactly
as it was before block execution began.

A successful block result contains:

- transactions_root
- resulting state_root
- receipts_root
- execution_root
- aggregate gas_used
- aggregate base_fee_burned
- aggregate producer_priority_fee
- ordered transaction receipts

## Canonical persistence and independent validation

A persisted native block execution result MUST retain the complete canonical `NativeBlockExecutionResultV1`, including its ordered receipts and aggregate accounting fields. A node MUST NOT replace this data with an external execution hash, replay-journal pointer, or summary that cannot reproduce the committed execution result.

Canonical block persistence binds, atomically:

- `BlockHeaderV1`;
- the complete canonical native execution result;
- the resulting `NativeStateV1` snapshot;
- cumulative chain work and canonical-head selection.

If any commitment check or persistence step fails, none of those state changes may become visible as a committed native block transition.

A receiving peer MUST independently validate the block transition from locally trusted parent state and protocol data. Peer-supplied state snapshots are not authoritative.

## Empty blocks

An empty block:

- executes no transactions,
- consumes zero gas,
- burns zero base fee,
- pays zero transaction priority fees,
- preserves native account state,
- uses the canonical empty transaction root,
- uses the canonical empty receipts root,
- and still produces a deterministic execution root.

## Unsupported actions

Native Execution V1 currently implements Transfer execution.

ContractCall and ContractCreate MUST NOT be silently interpreted as Transfer
or accepted without their applicable native contract execution rules.

## Interoperability vector

This vector is consensus-locked by the native execution test suite.

Execution context:

    network = devnet
    initial sender balance = 30000
    base_fee_per_gas = 3
    cpu_producer = 0909090909090909090909090909090909090909

Transaction 0:

    recipient = 0202020202020202020202020202020202020202
    nonce = 0
    value = 100
    gas_limit = 1000
    max_fee_per_gas = 10
    max_priority_fee_per_gas = 4

Transaction 1:

    recipient = 0303030303030303030303030303030303030303
    nonce = 1
    value = 200
    gas_limit = 1000
    max_fee_per_gas = 10
    max_priority_fee_per_gas = 4

Expected values:

    sender =
        1a642f0e3c3af545e7acbd38b07251b3990914f1

    transaction_0_id =
        12824fa00e58c1e26bbd294669016b563a5ad033459324da9a6a521c5597d38c

    transaction_1_id =
        e491ae3db2b72ac0e9bcf2cdb1544ef0625dc6a5ef771fe45852e9d490eec305

    receipt_0_commitment =
        6ede591e53c51bae6f306997e24a4f6a7343834d8de3734edb98c5dfe40b310e

    receipt_1_commitment =
        58993e4a0fee536b051d032ec31b8e5c1ed11e8630a9615aeab3fcdc6e6ad483

    transactions_root =
        601dcd68243e382ab2697cb3732c307e232875549714f1a69a033f81c095e0ef

    state_root =
        dabce914c479e1996769e4741b7febf2390cb4d1d7b9502d9fea6e87beda5e2f

    receipts_root =
        9ea9d54bed0acf2a7c0a18a44b043f1e169163b3ba6d962d58e8cbbe10af1fea

    execution_root =
        9dad2c600bd3d554a69143c444eac49313956a3e8005d25dc381ab5f7eabb2a7

    gas_used = 2000
    base_fee_burned = 6000
    producer_priority_fee = 8000

