# PaymentAuthorizationV1

Status: **CANDIDATE / pre-alpha bounded compute spending delegation**

## Purpose

`PaymentAuthorizationV1` allows a wallet or ComputeChannel authority to delegate narrowly bounded AI-compute spending power to a temporary requester, chat session, Job identity, or Agent.

It is not a transfer of wallet ownership.

## Candidate canonical fields

```text
PaymentAuthorizationV1
- schema_version
- authorization_id
- channel_id
- authorized_subject
- service_type
- model_scope
- execution_profile_scope
- max_per_job
- max_total
- max_jobs
- valid_from
- expires_at
- delegation_depth
- parent_authorization_id
- nonce
- signature
```

Exact field IDs and enum encodings remain candidate until vectors are defined.

## Authorized subject

`authorized_subject` is the cryptographic identity permitted to create chargeable Jobs under this authorization.

It MAY be a temporary wallet-controlled chat/session/job identity and MUST NOT be assumed to equal the funding account.

## Hard bounds

The authorization defines cryptographically enforceable ceilings.

A conforming verifier MUST reject any Job/receipt that would exceed any applicable bound, including:

- `max_per_job`;
- `max_total`;
- `max_jobs`;
- expiry;
- service type;
- model scope;
- execution-profile scope;
- parent authorization;
- delegation depth.

No user interface, Agent prompt, or worker claim may override these limits.

## Delegation

A child authorization MUST be equal to or narrower than its parent.

Delegation may reduce:

- remaining amount;
- per-job maximum;
- job count;
- service/model/profile scope;
- validity interval;
- delegation depth.

Delegation MUST NOT increase authority.

A value of zero delegation depth prohibits further delegation.

## Relationship to Capability

`Capability` remains the general permission primitive.

PaymentAuthorizationV1 is a specialized economic authorization optimized for compute payment and channel settlement.

An Agent may require both:

```text
CapabilityV1
  says: this Agent may invoke compute

PaymentAuthorizationV1
  says: this identity may spend at most X for that compute
```

Neither implies unrestricted wallet signing authority.

## Relationship to PaymentPlan

`PaymentPlan` describes how a Job's permitted payment may be allocated or settled.

`PaymentAuthorizationV1` answers a different question:

> what value is this requester cryptographically allowed to spend?

A Job may therefore reference both a PaymentAuthorization and a PaymentPlan.

## Relationship to Job

A chargeable Job binds to one valid authorization.

The Job requester signature proves control of `authorized_subject`.

The payment authorization proves that subject has bounded authority to incur compute charges.

This separates requester identity from funding identity.

## Replay protection

An authorization MUST bind:

- its parent/channel;
- subject;
- scope;
- validity interval;
- nonce;
- authorization identifier.

Consumed/settled usage MUST be accounted so that replaying the same Job or receipt cannot spend the same authorized amount twice.

## Invariants

1. Authorization cannot exceed the parent/channel value ceiling.
2. Child delegation cannot broaden parent authority.
3. The authorized subject need not be the funding account.
4. A worker cannot increase the permitted price after authorization.
5. An Agent cannot self-expand its spending limits.
6. Expired or revoked authorization cannot create new valid charges.
7. Compromise of a delegated key is bounded by that delegation.
8. Payment authorization grants no chain-consensus authority.
9. Natural-language instructions cannot override canonical limits.

## First-milestone simplification

The first implementation should support one non-delegating authorization per chat/session identity:

```text
channel
  -> session authorization
       -> Jobs
```

with:

- one compute service type;
- explicit model/profile scope;
- hard per-job maximum;
- hard total maximum;
- maximum job count;
- fixed expiry;
- delegation_depth = 0.

Recursive Agent delegation can be added later after the basic payment path is proven.


## Pricing interaction

A PaymentAuthorization bounds what a subject may spend; it does not let the worker choose an arbitrary charge.

Each chargeable Job binds an accepted `offer_id` or quote plus a hard `max_price`.

The effective payment ceiling is the minimum of:

```text
Job.max_price
PaymentAuthorization.max_per_job
PaymentAuthorization.remaining_total
ComputeChannel.remaining_authorized_amount
```

Any calculated or claimed charge above that effective ceiling is invalid.
