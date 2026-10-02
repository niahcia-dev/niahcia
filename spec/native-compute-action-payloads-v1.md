# Native Compute Action Payloads V1

Status: **CANDIDATE / pre-alpha canonical payload encoding**

## Purpose

This specification defines the exact NCE/1 objects carried in the `data` field of NativeTransaction schema V2 compute-channel actions.

Consensus derives mutable channel state from these immutable payloads. Wallets MUST NOT serialize mutable native state such as current status or settled amount into an open request and expect consensus to trust it.

## Object types

```text
0x0014  ComputeChannelOpenPayload
0x0015  ComputeChannelSettlePayload
0x0016  ComputeChannelRefundPayload
```

All three use NCE/1 schema version 1.

## ComputeChannelOpenPayloadV1

Canonical fields:

```text
1   worker_id
2   operator_id
3   channel_public_key
4   worker_payment_account
5   authorized_amount
6   expiry_height
7   claim_deadline_height
8   refund_available_height
9   service_scope_commitment
10  model_scope_commitment
11  execution_profile_scope_commitment
12  settlement_policy
```

### Semantics

The native transaction sender supplies `funding_account` implicitly through authenticated sender derivation.

The transaction `value` MUST equal `authorized_amount`.

Consensus derives:

```text
channel_id
opened_height
settled_amount = 0
state = OPEN
```

These derived values MUST NOT be supplied by the payload.

### Channel identifier

Candidate V1 derivation:

```text
channel_id =
  Keccak-256(
    "NIAHCIA/COMPUTE-CHANNEL-ID/V1" ||
    0x00 ||
    native_open_transaction_id
  )
```

The exact 32-byte native open transaction ID is the canonical SignedNativeTransactionV2 transaction identifier.

This guarantees channel uniqueness without a separate user-chosen channel nonce.

### Field requirements

- `worker_id`: canonical 32-byte worker identifier.
- `operator_id`: canonical 32-byte operator identifier.
- `channel_public_key`: canonical uncompressed secp256k1 public key, exactly 65 bytes, for V1 channel-receipt verification.
- `worker_payment_account`: exactly 20-byte native Account payload on the active network.
- `authorized_amount`: exactly 16 unsigned big-endian bytes representing integer aniah.
- height fields: canonical unsigned integers.
- scope commitments: exactly 32 bytes each.
- `settlement_policy`: canonical unsigned integer policy identifier.

V1 first-milestone settlement policy:

```text
0x00  CUMULATIVE_RECEIPT
```

Unknown policy values are invalid.

### Height ordering

Consensus MUST require:

```text
current_block_height < expiry_height
expiry_height <= claim_deadline_height
claim_deadline_height < refund_available_height
```

The opening block height becomes `opened_height`.

## ComputeChannelSettlePayloadV1

Canonical fields:

```text
1   channel_id
2   final_usage_receipt
```

Requirements:

- `channel_id`: exactly 32 bytes.
- `final_usage_receipt`: a CBOR byte string containing the complete canonical NCE/1 serialization of one ComputeUsageReceiptV1 object.

The embedded receipt bytes are validated exactly; they are not reinterpreted from JSON or an RPC structure.

The enclosing native transaction may be submitted by any fee-paying account.

## ComputeChannelRefundPayloadV1

Canonical fields:

```text
1   channel_id
```

Requirements:

- `channel_id`: exactly 32 bytes.

The authenticated transaction sender MUST equal the channel's `funding_account`.

## Channel receipt signature

For first-milestone native settlement, ComputeUsageReceiptV1 `channel_signature` uses canonical secp256k1:

```text
public key: 65-byte uncompressed SEC1
signature:  64-byte r || s
low-S required
```

The verifying key is the `channel_public_key` committed by ComputeChannelOpenPayloadV1.

The signature purpose is:

```text
SIGN/COMPUTE_USAGE_RECEIPT
```

Candidate digest:

```text
Keccak-256(
  "NIAHCIA" ||
  0x00 ||
  "SIGN/COMPUTE_USAGE_RECEIPT" ||
  0x00 ||
  network_id ||
  0x00 ||
  canonical_receipt_without_signature
)
```

Canonical receipt ID/signing vectors are required before lock.

## Settlement receipt checks performed by consensus

The chain does NOT replay or understand the AI Job.

For CUMULATIVE_RECEIPT settlement, consensus validates only settlement-relevant facts:

1. receipt is canonical;
2. receipt signature validates under channel_public_key;
3. receipt channel_id equals payload/channel state;
4. receipt worker_id/operator_id equal channel state;
5. cumulative_spent <= authorized_amount;
6. receipt expiry permits settlement;
7. channel remains OPEN;
8. current height <= claim_deadline_height;
9. receipt sequence is valid under the settlement policy;
10. checked arithmetic succeeds.

Fields such as job_id, result_commitment_id, price_offer_id, job_charge, and metering_evidence_hash are wallet/worker audit evidence. Their semantic correctness is checked off-chain before the wallet signs the receipt; ordinary consensus settlement does not re-run AI pricing or inference verification.

## Refund checks performed by consensus

For refund:

1. payload canonical;
2. channel exists;
3. channel is OPEN;
4. authenticated sender equals funding_account;
5. current height >= refund_available_height;
6. no terminal settlement/refund already exists.

## No mutable state in payload

The following are consensus-derived and MUST NOT appear in the open payload:

- channel_id;
- funding_account;
- opened_height;
- settled_amount;
- state/status;
- current receipt sequence;
- current cumulative spend.

## Invariants

1. Action payloads use NCE/1 only.
2. Open payload contains immutable requested terms only.
3. Channel ID is derived from the canonical open transaction ID.
4. Funding account is derived from the native transaction sender.
5. Open transaction value equals authorized_amount.
6. Settlement embeds exact canonical receipt bytes.
7. Refund contains only the channel identifier.
8. Mutable channel state is derived by consensus.
9. Receipt signature key is fixed at channel open.
10. Consensus does not execute or judge the underlying AI Job.

## Required vectors

Before lock, provide byte-for-byte vectors for:

1. Open payload;
2. derived channel_id;
3. valid channel public key;
4. invalid/high-S receipt signature rejection;
5. Settle payload embedding canonical receipt;
6. Refund payload;
7. invalid height ordering;
8. wrong transaction value vs authorized_amount;
9. wrong channel ID;
10. wrong worker/operator in receipt;
11. over-authorized cumulative_spent;
12. early refund;
13. settlement after claim deadline;
14. duplicate terminal transition.
