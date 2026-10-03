# Native Compute Gas and Fee Schedule V1

Status: **CANDIDATE / VECTORED / INACTIVE**

## Purpose

This specification assigns deterministic intrinsic gas to the NativeTransaction V2 ComputeChannel actions and applies the existing native transfer fee-accounting shape to those actions.

Gas is protocol accounting. It is not wall-clock runtime, host CPU time, GPU time, or AI-inference cost.

The GPU/AI service price remains part of the ComputeChannel value/receipt economy and is separate from the native transaction fee.

## Intrinsic gas

The candidate V1 schedule is:

```text
Native Transfer baseline       1,000 gas
ComputeChannelOpen             3,000 gas
ComputeChannelSettle           5,000 gas
ComputeChannelRefund           2,000 gas
```

The transfer value is the already-existing `NATIVE_TRANSFER_GAS_V1` baseline and is not changed by this specification.

The compute weights are intentionally fixed by action rather than by wall-clock measurement:

- **Open** performs transaction authentication, canonical opening-payload validation, balance/nonce checks, channel-ID derivation, locked-value accounting, and a new channel-state write.
- **Settle** is highest because it performs transaction authentication plus canonical receipt parsing, a second secp256k1 receipt-signature verification, channel validation, terminal state update, worker credit, funding refund, and nonce/fee accounting.
- **Refund** authenticates the funding account, validates timing/channel state, returns locked value, writes terminal channel state, and charges the native fee.

Canonical action payloads are tightly structured and bounded. A future protocol version may introduce a more granular gas model, but it must use an explicit successor schedule/version rather than silently changing these values.

## Gas-limit rule

For a compute action:

```text
gas_limit >= intrinsic_gas(action)
```

is required.

The actual gas charged for these V1 actions is the fixed intrinsic value. A larger gas limit only increases the maximum fee reserve; it does not increase actual gas used.

## Fee reserve

Before execution, the authenticated transaction sender must be able to cover:

```text
max_execution_charge =
    gas_limit * max_fee_per_gas

required_balance =
    transaction.value + max_execution_charge
```

using checked integer arithmetic.

For Open, `transaction.value` is the channel authorization that will be locked.

For Settle and Refund, transaction value is zero.

The pre-execution balance requirement deliberately prevents a settlement/refund sender from depending on value credited by the same transaction to fund its fee reserve.

## Effective fee

The fee calculation is identical in shape to native Transfer V1/V2:

```text
priority_headroom =
    max_fee_per_gas - base_fee_per_gas

priority_fee_per_gas =
    min(max_priority_fee_per_gas, priority_headroom)

effective_fee_per_gas =
    base_fee_per_gas + priority_fee_per_gas

base_fee_burned =
    base_fee_per_gas * intrinsic_gas

producer_priority_fee =
    priority_fee_per_gas * intrinsic_gas

actual_fee =
    base_fee_burned + producer_priority_fee

unused_fee_reserve =
    max_execution_charge - actual_fee
```

A transaction is invalid when `max_fee_per_gas < base_fee_per_gas`.

All arithmetic is checked integer arithmetic.

## Value separation

Compute service value and native transaction fees are distinct:

```text
channel authorization/value
    -> locked by Open
    -> worker payment + funding refund

native transaction fee
    -> base fee burned
    -> priority fee credited to CPU block producer
```

Compute payment is not block issuance and is not a CPU-PoW producer fee.

## Atomic execution

Compute transition plus fee accounting is atomic.

If payload validation, receipt verification, balance reserve, fee arithmetic, channel mutation, account debit/credit, or state-root generation fails, the candidate state is discarded.

The transaction nonce is consumed by the ComputeChannel transition exactly once. Fee charging does not consume a second nonce.

## Receipt / execution commitment

NativeReceiptV2 records the actual intrinsic gas and effective fee accounting for ComputeChannel actions.

NativeBlockExecutionResultV2 aggregates those values into:

- gas used;
- base fee burned;
- producer priority fee;
- receipt root;
- execution root.

The previously used zero fee/gas values were development placeholders only and are superseded by this candidate schedule in the inactive V2 block executor.

## Interoperability vector

The canonical fixture is:

`test-vectors/native-compute-gas-v1.json`

It fixes the intrinsic action values and a representative deterministic fee calculation.

Any incompatible change requires explicit protocol review and a successor gas schedule/vector before activation.

## Activation

This schedule remains inactive.

It is used by the inactive V2 execution path for development and interoperability testing, but NativeTransaction V2 / NativeStateV2 are not yet admitted to active mempool, P2P, mining, or devnet consensus.

A concrete V2 activation height must not be selected until this fee schedule and the remaining V2/runtime boundaries are accepted together.
