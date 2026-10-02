# ComputeChannelV1

Status: **CANDIDATE / pre-alpha bounded compute-payment channel**

## Purpose

`ComputeChannelV1` allows a wallet to authorize a bounded pool of NIAH for many off-chain AI compute jobs without requiring one base-chain transaction per prompt.

It is intentionally narrower than a generalized state-channel or payment-routing protocol.

The central rule is:

> lock or authorize value once, issue many bounded signed usage receipts off-chain, then settle the aggregate result on-chain.

## Non-goals

ComputeChannelV1 does not define:

- multi-hop payment routing;
- arbitrary bilateral smart-contract state;
- generalized channel networks;
- HTLCs;
- channel factories;
- cross-chain settlement;
- credit created without locked or otherwise enforceable value.

Those may be considered separately in the future and are not required for the first NIAHCIA AI milestone.

## Candidate canonical fields

```text
ComputeChannelV1
- schema_version
- channel_id
- funding_account
- channel_public_key
- worker_id
- operator_id
- authorized_amount
- service_scope
- model_scope
- execution_profile_scope
- opened_height
- expiry_height
- settlement_policy
- sequence
- status
```

Exact field IDs and settlement transaction encoding remain candidate until vectors are defined.

## Funding model

A channel MUST NOT create spendable value.

The chain must be able to establish an enforceable maximum amount associated with the channel, either through explicit lock/escrow or another separately specified native mechanism with equivalent safety.

For V1 the preferred direction is simple explicit lock/escrow.

```text
wallet account
   |
   | lock up to X aniah
   v
ComputeChannelV1
```

The authorized amount is a hard ceiling, not an estimate.

## Channel key

`channel_public_key` is distinct from the wallet's ordinary spending key.

The wallet may keep the funding account relationship private from compute workers where the settlement proof format permits, but full chain observers may still be able to correlate channel funding with the funding account. V1 MUST NOT claim perfect unlinkability.

Compromise of the channel signing key MUST NOT authorize spending above the value and scope already committed to the channel.

## Worker-bound channel

A V1 ComputeChannel is bound to one selected `worker_id` and its `operator_id`.

The channel may fund many Jobs for that worker during one compute session, but another worker requires a different channel.

This means:

```text
wallet
  -> select Worker A
  -> open Channel A
  -> many Jobs for Worker A
  -> settle/expire Channel A

worker failure or reselection
  -> select Worker B
  -> open Channel B
```

This deliberately avoids a multi-party shared channel in which several workers hold independent claims against the same value pool.

The wallet-local Agent/chat state remains independent of the selected worker, so changing workers does not require migrating conversation memory or Agent ownership.

## Usage receipts

Each accepted compute charge is represented off-chain by a signed cumulative or monotonic usage receipt.

Preferred V1 direction:

```text
ComputeUsageReceiptV1
- channel_id
- sequence
- cumulative_spent
- job_id
- worker_id
- result_commitment_id
- authorization_id
- receipt_expiry
- channel_signature
```

A later receipt for the same channel supersedes an earlier receipt only when its sequence is greater and its cumulative amount is greater than or equal to the prior valid amount.

A receipt is valid only for the worker/operator bound to the channel.

This keeps settlement bounded to the latest valid cumulative state rather than requiring every prompt to be published on-chain.

## Settlement

V1 should support a simple lifecycle:

```text
OPEN
  -> ACTIVE
  -> CLOSING
  -> SETTLED

OPEN/ACTIVE
  -> EXPIRED
  -> SETTLED
```

Settlement MUST NOT exceed `authorized_amount`.

Unused value returns to the funding account according to the settlement policy.

The first implementation SHOULD prefer cooperative or expiry-based close. Complex adversarial dispute machinery should be added only when a concrete attack requires it.

## Worker payment

A worker accepts a Job only when it can validate the accompanying `PaymentAuthorizationV1` and the channel can cover the maximum permitted charge.

After successful execution/verification, the wallet signs a usage receipt binding the charge to the Job and ResultCommitment.

Workers may aggregate many receipts before on-chain settlement.

V1 does not require a worker to accept channel payment; direct on-chain payment may remain available for testing or low-frequency use.

## Privacy boundary

The worker-facing Job identity need not be the channel funding account.

A channel may authorize a temporary requester/session identity through `PaymentAuthorizationV1`.

The chain may still reveal economic linkage when the channel is opened or settled. ComputeChannelV1 reduces prompt-level linkage and transaction volume; it is not a complete anonymity system.

## Failure handling

- worker offline before execution: no usage receipt, no compute charge;
- execution timeout: no charge unless a separately defined policy authorizes a partial objectively measurable charge;
- invalid ResultCommitment: no successful-job charge;
- wallet/client disappears: expiry permits eventual recovery of unused locked value;
- worker disappears after returning a valid result but before settlement: its signed/accepted receipt remains usable under the settlement policy;
- channel key compromise: attacker remains bounded by channel amount, service scope, authorization limits, and expiry.

## Invariants

1. A channel never creates value.
2. Total valid settlement never exceeds the channel's enforceable authorized amount.
3. Receipt sequences are monotonic and replay-protected.
4. Old receipts cannot increase settlement after a newer valid cumulative receipt.
5. Channel authority is narrower than wallet root authority.
6. Compute payment grants no PoW consensus authority.
7. A channel is not required for base-chain transfers.
8. One channel may fund many Jobs without placing every Job on-chain.
9. Funding identity and Job requester identity are not required to be identical.
10. Channel expiry must allow eventual recovery of unused value without a permanent third-party coordinator.
11. Each V1 channel is bound to exactly one worker/operator.
12. Worker reselection requires a distinct channel; channels are not multi-worker shared balances.

## First-milestone simplification

The first implementation should support:

- one wallet-funded channel;
- exactly one bound worker/operator per channel;
- one native currency: NIAH/aniah;
- direct worker payments only;
- no routing;
- no credit;
- monotonic cumulative receipts;
- fixed expiry;
- simple cooperative/expiry settlement;
- hard value ceiling.

A V1 channel MUST NOT represent a shared spend pool claimable by multiple unrelated workers. Binding the channel to one worker/operator avoids multi-party settlement races and keeps cumulative receipts sufficient for safe settlement.

That is sufficient to prove low-overhead AI micropayments without turning the compute milestone into a generalized payment-network project.


## Aggregation boundary

ComputeChannel settlement is the normal chain aggregation point for many ordinary Jobs.

The chain does not need individual Job creation, stream, response, or usage-receipt transactions.

The newest valid cumulative usage receipt summarizes the maximum acknowledged channel spend at settlement time.


## Native settlement boundary

Native Compute Settlement V1 defines the minimal chain state for channel funding, settlement, and refund.

The first milestone does not require Jobs, ResultCommitments, PaymentAuthorizations, WorkerAdvertisements, or ordinary usage-receipt issuance to be published on-chain.

Worker/operator chain registration and service bonds are also deferred from the first ordinary paid-compute settlement milestone.
