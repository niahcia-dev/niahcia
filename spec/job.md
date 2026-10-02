# Job

## Status

**CANDIDATE Protocol V1 object.**

## Purpose

`Job` is the generic unit of requested work.

The schema MUST NOT assume text-only inference or one execution/verification mechanism.

## Canonical fields

```text
Job
- schema_version
- job_id
- requester
- agent_id
- agent_version
- model_id
- execution_profile_id
- workload_type
- input_manifest_hash
- output_requirements_hash
- verification_policy_id
- payment_plan_id
- resource_requirements
- privacy_requirements
- scheduling_policy
- parent_job_id
- root_job_id
- created_block
- deadline_block
- status
- accepted_result_id
```

## Workload types

Initial vocabulary:

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

## Requester identity privacy

`requester` identifies the cryptographic identity authorized to create and control this Job. It MUST NOT be assumed to be the user's durable public wallet/payment address.

For privacy-preserving operation, a requester MAY be a temporary or rotatable wallet-controlled session/job identity.

Payment authorization is a separate concern. A Job may prove that valid bounded payment authority exists without making the worker-facing requester identity globally identical to the funding account.

Implementations MUST NOT infer that two Jobs belong to the same wallet merely because both are validly funded.

## Payment authorization

A chargeable Job SHOULD bind to a `PaymentAuthorizationV1` or another explicitly versioned payment authority.

The authorization proves that the requester identity may incur charges within a hard maximum without requiring the Job requester to be the durable funding account.

For channel-backed payment:

```text
funding account
  -> ComputeChannelV1
      -> PaymentAuthorizationV1
          -> Job
```

The Job's accepted price MUST NOT exceed either its own declared maximum or any applicable authorization/channel ceiling.

## Payment risk

A chargeable Job SHOULD declare or reference a ComputePaymentRiskV1 handling mode.

Payment risk is independent from VerificationPolicy.

Ordinary inexpensive interactive inference SHOULD use `SMALL`, with one completion-time usage receipt.

Higher-value one-shot Jobs MAY use `RESERVED`.

Long-running staged workloads SHOULD be deferred until staged payment and checkpoint semantics are explicitly implemented.

## Parent/child jobs

Agent-to-agent calls and compound workloads use:

- `parent_job_id` — immediate caller job
- `root_job_id` — top-level originating job

Budgets and recursion limits are enforced through policy objects rather than implicit conventions.

## Job state

User-facing lifecycle is intentionally compact:

```text
REQUESTED
EXECUTING
RESPONDED
SETTLED
```

Exceptional terminal/intermediate states include:

```text
FAILED
DISPUTED
EXPIRED
CANCELLED
```

Implementations may track more internal states for eligibility, assignment, commitment, auditing, challenge, retries, or settlement.

## Storage independence

A Job MUST NOT require decentralized storage merely to perform ordinary inference.

The committed input may be delivered directly over an authenticated encrypted job channel, referenced through a transient descriptor, or retrieved from an explicitly selected external source. `input_manifest_hash` commits to the job input representation; it does not imply that a NIAHCIA storage provider hosts that input.

Likewise, a result may be returned directly to the requester while `ResultCommitment` authenticates its integrity. Remote persistence is optional and belongs to a separate storage/service decision.

## Fast response rule

The job may reach `RESPONDED` before `SETTLED`.

Streaming output is an AI P2P data-plane operation. Blockchain settlement is asynchronous.

## Invariants

1. Large inputs/outputs must not be required on-chain.
2. The job must commit to exact input and execution requirements.
3. The accepted result must be bound to this `job_id`.
4. Payment and verification semantics are explicit through referenced policy objects.
5. Job deadlines use deterministic chain terms where on-chain enforcement is required.
6. Scheduling/worker selection must not require a permanent trusted coordinator. Ordinary paid worker selection may be performed locally by the wallet from signed eligible WorkerAdvertisements and is not a chain-consensus decision.
7. The Job schema does not assume a fixed verifier count or a network-wide 2-of-3 verification rule.
8. Requester identity and funding identity are separable; valid payment authorization MUST NOT require exposing the wallet's durable public account as the Job requester.
9. Ordinary Jobs do not require persistent Agent hosting or decentralized storage.

## Prototype status

Prototype work may initially support `TEXT_INFERENCE` with pinned Agent/Model/ExecutionProfile objects, but the former fixed redundant 2-of-3 verification assumption is superseded. Development verification behavior must reference an explicit development `VerificationPolicy` until the replacement V1 verification design is locked.
