# Job

## Status

**Schema V1: SUPERSEDED candidate; field meanings remain reserved.**  
**Schema V2: CURRENT CANDIDATE for the first decentralized AI milestone.**

## Purpose

`Job` is the canonical request for a unit of off-chain compute.

A Job is normally transported directly between wallet/client and worker. It is not inherently an on-chain state object.

The schema MUST NOT assume text-only inference, one execution mechanism, one verification mechanism, or decentralized storage.

## Versioning rule

Job schema V1 was defined before the wallet-local/off-chain architecture redesign.

Its assigned field IDs and meanings are not reused for different semantics.

Schema V2 keeps the same NCE object type `0x0008`, reuses only V1 fields whose meaning remains unchanged, and appends new field IDs for the redesigned architecture.

A V1 Job never silently acquires V2 semantics.

## Job schema V1

The original candidate fields remain reserved as:

```text
1   schema_version
2   job_id
3   requester
4   agent_id
5   agent_version
6   model_id
7   execution_profile_id
8   workload_type
9   input_manifest_hash
10  output_requirements_hash
11  verification_policy_id
12  payment_plan_id
13  resource_requirements
14  privacy_requirements
15  scheduling_policy
16  parent_job_id
17  root_job_id
18  created_block
19  deadline_block
20  status
21  accepted_result_id
```

Schema V1 is superseded for the first AI implementation and SHOULD NOT be newly implemented as the primary Job format.

## Job schema V2

Schema V2 uses these unchanged V1 fields where applicable:

```text
1   schema_version
2   job_id
3   requester
4   agent_id                 optional
5   agent_version            optional
6   model_id
7   execution_profile_id
8   workload_type
10  output_requirements_hash optional
11  verification_policy_id
12  payment_plan_id          optional
14  privacy_requirements     optional
16  parent_job_id            optional
17  root_job_id              optional
```

and appends:

```text
22  input_commitment
23  encrypted_input_descriptor
24  model_version
25  payment_authorization_id
26  price_offer_id
27  quote_id
28  max_price
29  payment_risk_mode
30  compute_session_id
31  submitted_height
32  assignment_deadline
33  execution_deadline
34  result_destination
35  resource_limits
36  requester_signature
```

Fields 9, 13, 15, 18, 19, 20, and 21 retain their V1 meanings but are not part of the normal V2 encoding.

Exact required/optional encoding details and ID/signature derivation require canonical vectors before V2 is locked.

## Workload types

Initial vocabulary remains:

```text
TEXT_INFERENCE
VISION_INFERENCE
AUDIO_INFERENCE
EMBEDDING
RERANKING
TOOL_EXECUTION
TRAINING
FINE_TUNING
MULTI_AGENT
CUSTOM
```

The first implementation focuses on bounded `TEXT_INFERENCE`.

## Requester

`requester` is the cryptographic identity controlling the Job.

It MAY be a temporary or rotatable wallet-controlled session/job identity.

It MUST NOT be assumed to equal the durable funding account.

`requester_signature` authenticates the complete canonical V2 Job signing payload.

## Agent fields

`agent_id` and `agent_version` are optional.

Ordinary wallet chat does not require a globally registered Agent object.

When an Agent is explicitly invoked, exact Agent/AgentVersion identity may be bound to the Job.

## Input

`input_commitment` commits to the exact canonical logical input used for the Job.

`encrypted_input_descriptor` describes how the selected worker obtains the encrypted input over AI Transport V1 or another explicitly compatible transport.

The descriptor MUST NOT imply decentralized storage.

The ordinary V2 path is direct encrypted wallet-to-worker delivery.

## Model and execution

`model_id`, `model_version`, and `execution_profile_id` identify what is requested to execute.

The ExecutionProfile defines runtime/tokenizer/output representation and other execution-relevant behavior.

## Verification

`verification_policy_id` identifies the required assurance policy.

STANDARD, VERIFIED, and HIGH_ASSURANCE behavior remains policy-driven.

Verification strength is independent from payment-risk handling.

## Payment

A chargeable Job binds:

```text
payment_authorization_id
price_offer_id
quote_id (optional)
max_price
payment_risk_mode
```

The worker MUST accept or reject those terms before execution.

No result may authorize settlement above `max_price` or the applicable PaymentAuthorization/ComputeChannel ceilings.

`payment_plan_id` is optional for the simple first milestone when no multi-party allocation beyond the primary worker is required. More complex verification/creator/routing economics may reference a PaymentPlan.

## ComputeSession

`compute_session_id` binds the Job to the current selected worker session.

A replacement worker uses a new ComputeSession and ordinarily a new worker-bound ComputeChannel.

The wallet may reconstruct a replacement Job from local context without migrating durable memory from the previous worker.

## Timing

`submitted_height` provides a chain-relative reference point without requiring the Job itself to be published on-chain.

`assignment_deadline` bounds worker acceptance/assignment.

`execution_deadline` bounds completion under the accepted Job.

These values may use chain heights or another explicitly versioned deterministic time representation defined before V2 lock.

## Result destination

`result_destination` identifies the authenticated return path or encryption destination for the Job result.

For the first milestone it may bind to the active AI Transport V1 ComputeSession.

It MUST NOT be assumed to be a public wallet address.

## Resource limits

`resource_limits` commits the user-authorized execution ceiling required to bound cost and execution behavior.

For token inference this should include at least the applicable maximum output-token limit when TOKEN_METERED pricing is used.

A worker SHOULD stop before producing usage outside the authorized resource/max-price envelope.

## Privacy

`privacy_requirements` describes the requested execution privacy class.

The Job contains commitments and delivery metadata, not the user's complete durable conversation or wallet-local memory.

Only selected context required for the Job is delivered to the worker.

## Parent/child Jobs

`parent_job_id` and `root_job_id` remain available for future compound or Agent-to-Agent execution.

The first chat milestone does not require recursive Jobs.

Any future child Job must remain within explicit Capability and PaymentAuthorization delegation bounds.

## Lifecycle boundary

Job Lifecycle Boundary V1 controls publication behavior.

For ordinary SMALL/STANDARD inference:

```text
create JobV2
  -> send over AI Transport
  -> worker accepts
  -> execute/stream
  -> ResultCommitment
  -> wallet verifies
  -> ComputeUsageReceipt
```

This entire sequence may remain off-chain.

The base chain normally sees ComputeChannel funding and eventual settlement, not every Job.

## Job state

Schema V2 deliberately omits the V1 `status` and `accepted_result_id` fields.

REQUESTED/EXECUTING/RESPONDED/FAILED/etc. are derived logical states from authenticated transport/evidence rather than mutable fields that require canonical Job mutation.

A JobV2 object is immutable after signing.

## Storage independence

A JobV2 MUST NOT require decentralized storage merely to execute ordinary inference.

Inputs may be delivered directly over AI Transport V1 and results may return directly over the same logical ComputeSession.

## Invariants

1. JobV2 is immutable after requester signature.
2. Requester identity and funding identity are separable.
3. Ordinary JobV2 execution is off-chain by default.
4. Input plaintext is not public chain data.
5. JobV2 does not require decentralized storage.
6. JobV2 binds exact model/version/profile requirements.
7. Chargeable Jobs bind hard price/payment authorization.
8. Payment-risk and VerificationPolicy are independent.
9. Result delivery need not expose a wallet address.
10. Mutable lifecycle status is derived externally rather than rewriting the signed Job.
11. V1 field meanings remain reserved and are not silently repurposed.
12. A fixed 2-of-3 verification assumption is not part of JobV2.

## First milestone

The first JobV2 implementation should support:

```text
TEXT_INFERENCE
STANDARD verification
SMALL payment risk
direct AI Transport delivery
one selected ComputeSession
one worker-bound ComputeChannel
TOKEN_METERED or FIXED signed pricing
wallet-local context/memory
off-chain ResultCommitment
off-chain ComputeUsageReceipt
eventual channel settlement
```

TRAINING, FINE_TUNING, recursive Agent execution, staged payment, and mandatory on-chain Job publication are outside the first milestone.
