# Native Transaction V2 — Compute Channel Actions

Status: **CANDIDATE / pre-alpha transaction extension**

## Purpose

Native Transaction schema V2 extends the native action vocabulary with dedicated ComputeChannel settlement actions while preserving all NativeTransaction V1 meanings.

It exists so compute settlement does not depend on smart contracts or reinterpret V1 ContractCall/ContractCreate.

## Versioning

Object types remain:

```text
0x0010  NativeTransactionBody
0x0011  SignedNativeTransaction
```

Schema version distinguishes V1 from V2.

NativeTransactionBodyV2 retains the V1 payload field meanings:

```text
1   network_id
2   chain_id
3   nonce
4   action
5   target_payload
6   value
7   gas_limit
8   max_fee_per_gas
9   max_priority_fee_per_gas
10  data
```

No V1 field is repurposed.

SignedNativeTransactionV2 retains:

```text
1   body
2   public_key
3   signature
```

with the same high-level rule that the signature authenticates the exact canonical body bytes under the versioned native-transaction signing domain.

## V2 signing domain

NativeTransaction schema V2 MUST have an explicit signing/identifier domain.

The candidate signing purpose is:

```text
SIGN/NATIVE_TRANSACTION/V2
```

Candidate signing digest:

```text
Keccak-256(
  "NIAHCIA" ||
  0x00 ||
  "SIGN/NATIVE_TRANSACTION/V2" ||
  0x00 ||
  network_id ||
  0x00 ||
  canonical_unsigned_body_v2
)
```

The `network_id` committed by the signing domain MUST equal the body network_id.

The candidate transaction identifier is:

```text
tx_id_v2 =
  Keccak-256(
    "NIAHCIA/TX-ID/V2" ||
    0x00 ||
    canonical_signed_transaction_v2
  )
```

This V2 transaction ID is the value used by ComputeChannelOpen channel-ID derivation.

NativeTransaction V1 continues using its existing V1 signing and transaction-ID domains.

A V1 signed transaction MUST NOT validate as V2 merely because its payload fields happen to be structurally similar.

## V2 action vocabulary

V2 retains the V1 actions:

```text
0x00  Transfer
0x01  ContractCall
0x02  ContractCreate
```

and adds:

```text
0x10  ComputeChannelOpen
0x11  ComputeChannelSettle
0x12  ComputeChannelRefund
```

These values are CANDIDATE until canonical vectors and implementation tests lock them.

## ComputeChannelOpen

For:

```text
action = 0x10
```

requirements:

- `target_payload` is empty;
- `value` equals the exact channel `authorized_amount`;
- `data` contains canonical NCE/1 bytes of one ComputeChannelV1 opening descriptor;
- the authenticated native transaction sender MUST equal the channel `funding_account`;
- the channel must not already exist;
- channel heights/scopes/worker payout account must satisfy Native Compute Settlement V1;
- sender balance must cover `value` plus maximum native transaction fee reserve.

Execution:

1. consume native transaction nonce;
2. debit/lock `value` from sender spendable balance;
3. create OPEN ComputeChannelStateV1;
4. charge ordinary native transaction fee;
5. commit resulting state.

The locked channel value is not burned and is not paid to the CPU block producer.

## ComputeChannelSettle

For:

```text
action = 0x11
```

requirements:

- `target_payload` is empty;
- `value = 0`;
- `data` contains canonical settlement payload bytes identifying:
  - channel_id;
  - final ComputeUsageReceiptV1 bytes;
- channel exists and is OPEN;
- settlement occurs no later than `claim_deadline_height`;
- receipt validates under Native Compute Settlement V1.

The authenticated native transaction sender MUST equal the channel's committed `worker_payment_account`.

This intentionally prevents the funding wallet or another holder of an older valid cumulative receipt from front-running the worker with a lower acknowledged amount and terminally settling the channel for less than the worker's newest receipt.

Execution:

1. authenticate sender as the committed `worker_payment_account`;
2. consume worker-payment-account native nonce;
3. validate final receipt;
4. credit `cumulative_spent` to committed `worker_payment_account`;
5. credit remaining authorized value to `funding_account`;
6. mark channel SETTLED;
7. charge the submitting worker account's ordinary native transaction fee.

A second settlement is invalid.

## ComputeChannelRefund

For:

```text
action = 0x12
```

requirements:

- `target_payload` is empty;
- `value = 0`;
- `data` contains the canonical channel identifier/refund payload;
- authenticated sender MUST equal the channel `funding_account`;
- channel exists and is OPEN;
- current height is at least `refund_available_height`;
- no valid settlement has already transitioned the channel.

Execution:

1. consume funding-account native nonce;
2. return the full still-locked channel amount to funding account;
3. mark channel REFUNDED;
4. charge ordinary native transaction fee.

## Canonical action payloads

`spec/native-compute-action-payloads-v1.md` defines the exact NCE/1 `data` payloads for Open, Settle, and Refund.

ComputeChannelOpen does not serialize mutable channel state. Consensus derives channel_id from the canonical open transaction ID, funding_account from the authenticated sender, opened_height from block execution height, settled_amount=0, and state=OPEN.

## Settlement payload objects

The exact settlement/refund payload encodings MUST be explicit NCE/1 objects or another canonical versioned encoding.

Implementation MUST NOT use ad-hoc JSON, ABI guessing, or implementation-private byte layouts.

Before lock, vectors must cover open, settle, refund, duplicate settle, early refund, wrong payout, invalid receipt signature, overflow, and wrong-network cases.

## Native fee separation

All three actions pay ordinary native transaction fees according to the active fee policy.

Compute value is separate:

```text
transaction fee
  -> CPU-PoW fee policy

channel value
  -> worker payment + funding-account refund
```

A worker's compute payment MUST NOT be counted as newly issued block reward.

## Mempool considerations

Admission may validate static conditions such as canonical encoding, signature, network/chain, action shape, fee reserve, and currently observable channel state.

Consensus execution remains authoritative.

A settlement/refund transaction that becomes invalid because an earlier block already settled/refunded the channel MUST be rejected/removed after state advances.

No special off-chain Job data belongs in the chain mempool.

## Replay protection

Replay protection remains anchored in:

- network_id;
- chain_id;
- authenticated sender;
- sender nonce;
- canonical signed transaction bytes.

Channel-level replay protection additionally requires:

- unique channel_id;
- one terminal channel transition;
- receipt sequence/commitment validation;
- settlement/refund state checks.

## Why settlement is worker-submitted in V1

A wallet-signed cumulative receipt proves an acknowledged amount, but an older receipt may acknowledge less than a newer receipt.

Because settlement is terminal, allowing an arbitrary account to submit any still-valid older receipt would permit a funding wallet or third party holding that receipt to settle the channel early for the lower amount.

V1 therefore requires the native Settle transaction sender to equal the channel's committed `worker_payment_account`.

A future version may restore permissionless relaying by adding a separate worker settlement authorization/signature that identifies the final receipt without giving the relayer control of the payout.

## Invariants

1. V1 transaction semantics remain unchanged.
2. Compute actions are explicit V2 actions.
3. ContractCall is not a compute settlement escape hatch.
4. Open locks existing NIAH; it does not mint.
5. Settle pays only the channel-committed worker payment account.
6. Refund can occur only after the claim/refund boundary.
7. One channel has only one terminal settlement/refund transition.
8. Compute service value and native transaction fees remain distinct.
9. Jobs/prompts/results are not transaction payload requirements.
10. All compute-action payloads are canonical and versioned.

## First implementation order

Implement in this order only after the current native transfer/P2P transaction path is ready:

1. canonical ComputeChannel opening descriptor/payload;
2. ComputeChannelOpen execution/state tests;
3. canonical settlement payload and receipt verification;
4. ComputeChannelSettle execution/state tests;
5. ComputeChannelRefund execution/state tests;
6. restart/persistence/reorg tests;
7. canonical NCE vectors;
8. P2P non-empty block propagation with these transactions.

Do not implement AI transport or worker runtime inside consensus execution.


## Activation boundary

NativeTransaction schema V2 compute actions are valid only when the active network parameters select NativeStateV2 / compute-action activation.

Before activation:

- schema V2 compute actions are consensus-invalid;
- NativeStateV1 remains authoritative.

At and after activation:

- V2 compute actions may execute under the explicitly activated rules;
- V1 Transfer semantics remain valid unless a separate transition says otherwise.

The activation height/network parameter must be committed by the network-parameter specification and covered by migration vectors.

## Gas schedule boundary

Compute-channel actions require deterministic intrinsic gas values before implementation lock.

Gas MUST be protocol accounting, not wall-clock execution time.

The exact Open/Settle/Refund gas constants are deliberately not assigned by this document yet. They require explicit review alongside canonical payload size/signature-verification cost and test vectors.

Implementations MUST NOT invent local gas values and treat them as consensus.
