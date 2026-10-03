# Native Compute Settlement V1

Status: **CANDIDATE / pre-alpha native compute settlement boundary**

## Purpose

Native Compute Settlement V1 defines the minimum base-chain state required to support the first NIAHCIA AI milestone.

The central rule is:

> the base chain settles bounded compute value; it does not execute AI Jobs or track their ordinary lifecycle.

## First-milestone chain state

The first end-to-end AI milestone requires only one new compute-specific native state family:

```text
ComputeChannelStateV1
```

Worker discovery, worker advertisements, JobV2, AI Transport, ResultCommitmentV2, pricing offers, PaymentAuthorizationV1, and ordinary ComputeUsageReceiptV1 exchange remain off-chain.

Operator/worker registration and service bonds are deliberately deferred from the first settlement milestone.

## ComputeChannelStateV1

Candidate state:

```text
ComputeChannelStateV1
- channel_id
- funding_account
- worker_id
- operator_id
- channel_public_key
- worker_payment_account
- authorized_amount
- settled_amount
- opened_height
- expiry_height
- claim_deadline_height
- refund_available_height
- service_scope_commitment
- model_scope_commitment
- execution_profile_scope_commitment
- settlement_policy
- state
```

Candidate states:

```text
OPEN
SETTLED
REFUNDED
```

No per-Job state is stored in this native object.

## Channel open

A channel-open transition:

1. authenticates the funding account through the normal native transaction signature/nonce domain;
2. validates channel parameters;
3. debits/locks exactly `authorized_amount` from spendable account balance;
4. creates one unique OPEN ComputeChannelStateV1;
5. records the worker/operator and channel public key used for off-chain receipts;
6. records an expiry height;
7. prevents the locked value from being spent elsewhere.

Opening a channel does not create currency.

## Channel settlement

A channel-settle transition accepts the newest valid cumulative ComputeUsageReceiptV1 permitted by the settlement protocol.

It validates at least:

- channel exists and is OPEN;
- receipt channel_id matches;
- worker/operator match the channel;
- channel signature is valid;
- authenticated Settle transaction sender equals the committed worker_payment_account;
- receipt sequence/finality rules are satisfied;
- cumulative_spent <= authorized_amount;
- cumulative_spent is not below any already accepted settlement state;
- receipt is not expired under the applicable rules.

The transition then:

```text
worker payment = cumulative_spent -> worker_payment_account
wallet refund  = authorized_amount - cumulative_spent -> funding_account
```

and moves the channel to SETTLED.

A settled channel cannot be settled again.

### Older-receipt front-running

A funding wallet necessarily possesses the receipts it signed. If arbitrary accounts could submit Settle, the wallet could submit an older, lower cumulative receipt before the worker submits the newest receipt and terminally underpay the worker.

For V1, Settle is therefore worker-authenticated: the native transaction sender MUST equal the channel's committed `worker_payment_account`.

## Claim/refund race prevention

`expiry_height` stops new Job use but does not immediately unlock funds.

The worker may submit the latest valid receipt through `claim_deadline_height`.

Refund is not valid before `refund_available_height`.

This prevents a funding wallet from waiting for the Job/session to end and racing a worker's already signed final receipt with an immediate refund.

## Expiry refund

If an OPEN channel reaches `refund_available_height` without a valid settlement claim, the funding authority may reclaim the unused locked value according to the settlement policy.

The simple first-milestone policy is:

```text
no accepted final receipt at expiry
    -> full refund to funding account
    -> REFUNDED
```

If a valid final receipt has already been accepted by the native settlement transition, normal settlement applies instead.

More complex challenge periods are deferred.

## Cooperative close

The first implementation MAY support explicit cooperative close before expiry when wallet and worker agree on the final cumulative receipt.

This uses the same settlement arithmetic.

No generalized state-channel dispute protocol is required.

## Worker/operator identity in V1 settlement

For the first milestone, `worker_id` and `operator_id` are identifiers committed by the channel and authenticated by the off-chain worker protocol.

The native chain does not need a complete worker registry merely to pay the worker designated by the funding account.

This is safe for ordinary purchaser-funded compute because the user chooses the worker and locks the user's own funds.

Future bonded/high-assurance protocols may require chain-registered Worker/Operator identity.

## Bonds

Service bonds are NOT required for the first compute settlement milestone.

They may later be introduced for:

- objective fraud penalties;
- verifier accountability;
- Sybil-cost policy;
- high-assurance service admission.

A bond must never grant CPU-PoW consensus authority.

Bond design is separate from basic user-funded channel settlement.

## Native transaction integration

Current NativeTransaction V1 defines only:

```text
Transfer
ContractCall
ContractCreate
```

Native Compute Settlement V1 MUST NOT silently reinterpret any of those actions.

The first implementation must use one of these explicit version-safe paths:

1. define NativeTransaction schema/action version 2 with dedicated compute-channel actions; or
2. define a separately versioned native system-action envelope referenced by a future NativeTransaction version.

Using the unimplemented ContractCall placeholder as an undocumented compute-channel escape hatch is forbidden.

## Candidate future native actions

The preferred direction is a future version with explicit actions conceptually equivalent to:

```text
ComputeChannelOpen
ComputeChannelSettle
ComputeChannelRefund
```

Exact numeric action values are NOT assigned here.

They require NativeTransaction versioning, canonical field/action encoding, gas rules, execution semantics, and interoperability vectors.

## Gas and fees

Opening, settling, or refunding a ComputeChannel is a native state transition and should pay the ordinary CPU-PoW transaction fee applicable to that native action.

The compute service payment itself is distinct from the transaction fee:

```text
native transaction fee
  -> pays CPU-PoW block execution/production policy

compute channel value
  -> pays selected compute worker
```

No compute payment is minted by settlement.

## Supply accounting

Compute settlement only moves already existing NIAH.

```text
funding account
  -> locked channel value
  -> worker payment + wallet refund
```

The transition must preserve value exactly, excluding separately accounted native transaction fees/burns.

## Chain independence from AI

If all compute workers disappear:

- open channels may expire/refund;
- ordinary NIAH transfers continue;
- CPU PoW continues;
- chain validation continues.

Compute-worker availability is not a precondition for chain validity.

## Off-chain evidence retained locally

Wallet and worker retain signed evidence until channel settlement and any applicable settlement window ends:

- JobV2;
- Job acceptance;
- price offer;
- ResultCommitmentV2;
- metering evidence;
- ComputeUsageReceiptV1.

The chain normally sees only the final receipt required for settlement, not the full conversation or Job transcript.

## Invariants

1. ComputeChannel settlement never mints value.
2. Locked channel value cannot be double-spent.
3. Settlement never exceeds authorized_amount.
4. One channel can settle at most once.
5. Refund/settlement arithmetic is checked integer aniah arithmetic.
6. Jobs/prompts/results are not required on-chain.
7. Worker/operator chain registration is not required for the first ordinary paid compute milestone.
8. Bonds are deferred from the first milestone.
9. Existing NativeTransaction V1 actions are not reinterpreted.
10. Dedicated native compute actions require explicit transaction versioning and vectors.
11. CPU PoW remains the sole chain-consensus authority.
12. Chain validity does not depend on AI-worker availability.

## First milestone acceptance test

The first native AI-payment test should prove:

1. wallet has spendable NIAH;
2. wallet opens a worker-bound ComputeChannel with 1 NIAH;
3. native state locks exactly 1 NIAH;
4. several Jobs execute entirely off-chain;
5. wallet signs monotonically increasing ComputeUsageReceiptV1 values;
6. final cumulative receipt is 0.37 NIAH;
7. one settlement transition credits 0.37 NIAH to the worker destination;
8. 0.63 NIAH returns to the funding account;
9. second settlement attempt is rejected;
10. ordinary transfers and CPU-PoW chain progress remain independent of compute availability.


## Native State V2 integration

`spec/native-state-v2-compute-channels.md` defines the consensus state-root, snapshot, persistence, activation, and reorg rules for ComputeChannelStateV1.

Compute-channel state MUST be part of the canonical native state snapshot/root. It MUST NOT be maintained as an uncommitted auxiliary side table.

NativeStateV1 remains accounts-only and unchanged; compute actions require explicit NativeStateV2 activation.


## Native transaction gas and fee accounting

ComputeChannel Open / Settle / Refund use the deterministic candidate intrinsic-gas and fee rules defined in `spec/native-compute-gas-v1.md`.

The native transaction fee is separate from channel authorization and compute-service payment. Base fee is burned and priority fee is credited to the CPU block producer. The ComputeChannel state transition and its native fee accounting must commit atomically.
