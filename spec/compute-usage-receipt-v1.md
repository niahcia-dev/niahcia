# ComputeUsageReceiptV1

Status: **CANDIDATE / pre-alpha cumulative compute settlement receipt**

## Purpose

`ComputeUsageReceiptV1` is the wallet-signed cumulative payment acknowledgement for one worker-bound `ComputeChannelV1`.

The worker does not choose the final charge. The wallet verifies the accepted pricing terms and result/metering evidence, recomputes the valid charge, and only then signs the next cumulative receipt.

## Candidate canonical fields

```text
ComputeUsageReceiptV1
- schema_version
- receipt_id
- channel_id
- authorization_id
- worker_id
- operator_id
- sequence
- previous_receipt_id
- cumulative_spent
- job_id
- result_commitment_id
- price_offer_id
- job_charge
- metering_evidence_hash
- expires_at
- channel_signature
```

## Wallet verification before signing

Before signing a new receipt, the wallet/client MUST verify at least:

1. the receipt targets the currently valid worker-bound channel;
2. the Job was signed by an identity authorized under the referenced PaymentAuthorization;
3. the accepted `price_offer_id` or quote was valid when the Job was accepted;
4. the Job's hard `max_price` was not exceeded;
5. the PaymentAuthorization limits remain satisfied;
6. the ComputeChannel has sufficient remaining authorized value;
7. the ResultCommitment binds the correct Job, worker, model, and ExecutionProfile;
8. the applicable VerificationPolicy is satisfied to the level required for payment;
9. metering evidence is valid and reproducible under ComputePricingV1;
10. `job_charge` equals the wallet's independently calculated valid charge;
11. `cumulative_spent = prior_cumulative_spent + job_charge`;
12. sequence and previous-receipt linkage are monotonic and replay-safe.

If any check fails, the wallet MUST NOT sign the receipt.

## Charge authority

The worker may propose a charge, but that proposal has no settlement authority by itself.

Settlement authority comes from the wallet/channel signature over the cumulative receipt after verification.

The worker MUST NOT be able to transform:

```text
accepted max_price = 0.05 NIAH
```

into:

```text
settlement = 0.08 NIAH
```

without a valid wallet-signed receipt, which the wallet must reject.

## Cumulative model

For a channel:

```text
receipt 1: cumulative_spent = 0.010
receipt 2: cumulative_spent = 0.027
receipt 3: cumulative_spent = 0.041
```

Only the newest valid cumulative receipt is needed to prove the maximum acknowledged spend for that channel.

A worker cannot add old cumulative receipts together.

## Result and payment separation

Receiving an AI result does not automatically authorize payment.

The wallet may receive and inspect the result before signing the corresponding usage receipt.

For STANDARD jobs, successful integrity/execution checks may be sufficient under the applicable VerificationPolicy.

For VERIFIED/HIGH-ASSURANCE jobs, the wallet waits until the required independent evidence exists before acknowledging a successful-job charge unless the policy explicitly defines staged payment.

## Streaming

The first implementation SHOULD NOT require per-token micropayment signatures.

Streaming output may be delivered before the final receipt.

The wallet signs one Job-level charge after completion/verification.

Incremental/staged receipts can be added later if large Jobs create unacceptable worker risk.

## Worker risk

A worker risks performing work before receiving the final signed usage receipt.

V1 mitigates this with:

- hard per-job limits;
- small ordinary chat Jobs;
- worker choice to reject overly large unpaid exposure;
- optional future staged billing;
- reputation/history;
- bounded channels and authorizations.

The first milestone deliberately accepts this limited exposure rather than adding complex streaming-payment machinery.

## Wallet disappearance

If the wallet disappears after receiving a valid result but before signing the Job receipt, the worker may remain unpaid for that Job under the simple V1 model.

Workers SHOULD therefore cap per-Job exposure.

A future version may add pre-authorized staged claims or escrowed Job reservations if measurements show this risk is material.

## Invariants

1. Workers cannot self-sign customer payment authority.
2. Wallets independently calculate every acknowledged Job charge.
3. Cumulative spend never decreases.
4. Cumulative spend never exceeds channel authorization.
5. Job charge never exceeds accepted Job max_price.
6. Old cumulative receipts cannot be summed for extra value.
7. Receipt linkage is replay-protected.
8. Result receipt and payment receipt are distinct objects.
9. Invalid or insufficiently verified results do not automatically earn successful-job payment.
10. First-milestone receipts are Job-level, not per-token streaming signatures.
