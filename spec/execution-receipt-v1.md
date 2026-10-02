# ExecutionReceiptV1

Status: **CANDIDATE / pre-alpha auditability contract**

## Purpose

`ExecutionReceiptV1` is a canonical, privacy-aware record of what execution was requested, which exact protocol resources governed it, who claimed execution, what result was committed, how it was verified, and how settlement was determined.

It is intended to make machine work auditable without requiring private prompts, outputs, memory, or model payloads to be published on-chain.

An execution receipt is evidence. It does not by itself make a result correct; correctness is determined by the applicable `VerificationPolicy` and protocol state transition.

## Candidate canonical fields

```text
ExecutionReceiptV1
- schema_version
- job_id
- requester_id
- agent_id
- agent_version
- agent_manifest_hash
- worker_id
- operator_id
- model_id
- model_version_hash
- execution_profile_id
- input_commitment
- result_commitment_id
- verification_policy_id
- verification_outcome
- verification_evidence_root
- privacy_policy_root
- resource_usage_commitment
- payment_plan_id
- settlement_commitment
- started_at
- completed_at
- finalized_block
```

Fields that are not applicable must use the canonical explicit empty/null representation defined by the serialization specification rather than being ambiguously omitted when omission would alter the receipt hash.

## Binding requirements

A receipt MUST bind to an exact `Job`, exact execution-critical AgentVersion/manifest where an agent initiated or governed the work, exact Model/ExecutionProfile, and exact ResultCommitment.

A receipt MUST NOT claim only a mutable display name such as a model nickname or agent label.

## Input and result privacy

`input_commitment` commits to the canonical job input or authorized encrypted/private input representation. The receipt need not reveal plaintext input.

`result_commitment_id` references the canonical ResultCommitment. The receipt need not reveal plaintext result when the Job/privacy policy restricts it.

The receipt therefore supports auditability of *which committed work* occurred without requiring universal public disclosure of its contents.

## Verification

`verification_policy_id` identifies the exact policy applied.

`verification_outcome` records the canonical finalized outcome under that policy.

`verification_evidence_root` commits to the evidence set used to reach that outcome where the policy requires external evidence.

A worker signature alone is not equivalent to successful verification.

## Resource usage

`resource_usage_commitment` may commit to canonical metering evidence required by the PaymentPlan or VerificationPolicy, such as declared execution duration, token/accounting units, accelerator time, storage/retrieval work, or other versioned metrics.

This field MUST NOT become a license for arbitrary unverifiable billing. Each compensable metric requires explicit semantics in the applicable payment/service specification.

## Settlement

`payment_plan_id` identifies the applicable payment rules.

`settlement_commitment` commits to the finalized economic outcome, including the protocol-defined recipients/amounts where applicable.

The receipt does not itself authorize funds beyond the Job/PaymentPlan/chain state.

## Timing

`started_at` and `completed_at` are execution evidence fields and must use the time semantics defined by the applicable Job/ExecutionProfile. They are not automatically consensus timestamps.

`finalized_block` identifies the canonical chain point at which the receipt's finalized protocol outcome became established, where applicable.

## Uses

Execution receipts are intended to support:

- requester audit trails;
- agent lineage/history;
- dispute/challenge evidence;
- service accounting;
- debugging and observability;
- cross-host migration/history reconstruction;
- reputation inputs where a future reputation protocol explicitly permits them;
- independent verification that settlement corresponds to the committed job/result/policy.

They must not silently create a centralized reputation authority.

## Security invariants

1. Receipt identity must be derived from canonical serialization and an explicit domain.
2. A receipt must bind exact versions/identifiers, not mutable names.
3. Private input/output must remain hidden when policy requires it; commitments are not permission to disclose plaintext.
4. A receipt cannot upgrade an unverified result into a verified result.
5. A receipt cannot grant capabilities that the Job/Agent did not possess.
6. Settlement fields must match canonical settlement state or be rejected as non-authoritative evidence.
7. Unknown receipt versions must not be interpreted as V1.

## Consensus boundary

Not every receipt byte must necessarily be stored directly on-chain. The chain may commit to a receipt identifier/root and finalized settlement state while the full canonical receipt/evidence remains content-addressed and retrievable through the service/storage network.

The exact inclusion/commitment mechanism is not locked by this candidate.

## Required vectors before lock

1. canonical receipt serialization;
2. receipt ID/hash derivation;
3. public input/result example;
4. private input/result commitment example;
5. exact AgentVersion/manifest binding;
6. exact Model/ExecutionProfile binding;
7. verification evidence-root binding;
8. settlement commitment binding;
9. substitution/mismatch rejection cases;
10. cross-implementation receipt verification fixture.
