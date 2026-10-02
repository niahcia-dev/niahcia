# ResultCommitment

## Status

**Schema V1: legacy candidate layout retained.**  
**Schema V2: CURRENT CANDIDATE for ordinary off-chain compute results.**

## Purpose

`ResultCommitment` is the worker-signed binding between one Job, the exact execution context, and the produced result/evidence.

A ResultCommitment is valid as a canonical signed off-chain object. Routine publication on-chain or storage-node persistence is not required.

## Versioning rule

Schema V1 field IDs and meanings remain reserved.

Schema V2 reuses only unchanged V1 fields and appends new IDs for the redesigned off-chain architecture.

## Schema V1

```text
1   schema_version
2   commitment_id
3   job_id
4   worker_id
5   operator_id
6   model_id
7   execution_profile_id
8   input_hash
9   output_hash
10  token_or_artifact_hash
11  runtime_receipt_hash
12  output_manifest_hash
13  output_location
14  started_at
15  completed_at
16  submitted_block
17  nonce_or_salt_commitment
18  signature
```

V1 remains interpretable but is superseded for the first AI implementation.

## Schema V2

V2 reuses unchanged fields:

```text
1   schema_version
2   commitment_id
3   job_id
4   worker_id
5   operator_id
6   model_id
7   execution_profile_id
9   output_hash
10  token_or_artifact_hash
11  runtime_receipt_hash
14  started_at
15  completed_at
17  nonce_or_salt_commitment
18  signature
```

and appends:

```text
19  model_version
20  input_commitment
21  canonical_output_commitment
22  metering_evidence_hash
23  verification_evidence_root
24  compute_session_id
25  completion_sequence
```

V1 fields 8, 12, 13, and 16 retain their V1 meanings but are omitted from normal V2 encoding.

Exact required/optional rules and signing vectors remain candidate.

## Job binding

A ResultCommitmentV2 MUST bind exactly one immutable JobV2 through `job_id`.

The worker MUST NOT substitute a different:

- requester request;
- model/version;
- ExecutionProfile;
- input commitment;
- ComputeSession.

## Input commitment

`input_commitment` MUST equal the JobV2 input commitment for the execution being claimed.

The worker does not need to publish plaintext input.

## Output commitments

`output_hash` commits to the returned output bytes under the applicable output format.

`canonical_output_commitment` commits to the canonical result representation defined by the ExecutionProfile.

For token inference, `token_or_artifact_hash` SHOULD commit to canonical token IDs or another explicitly versioned representation suitable for comparison and metering.

Rendered text formatting is not automatically the canonical verification representation.

## Model and profile

`model_id`, `model_version`, and `execution_profile_id` make the claimed execution environment explicit.

A result produced under a different model/version/profile does not satisfy the Job merely because the human-readable answer appears similar.

## Metering

`metering_evidence_hash` commits the usage evidence required by ComputePricingV1.

For TOKEN_METERED inference this SHOULD cover the canonical input/output token accounting needed for wallet-side recomputation.

The worker signature authenticates the claim; it does not force the wallet to accept an incorrect charge.

## Verification evidence

`verification_evidence_root` is optional for STANDARD execution when no additional evidence is required.

VERIFIED/HIGH_ASSURANCE policies may bind verifier commitments, proof/attestation roots, challenge evidence, or other policy-defined material.

VerificationPolicy determines what evidence is sufficient.

## ComputeSession binding

`compute_session_id` binds the result to the active worker session under which the Job was accepted.

This prevents a result from one session/context being replayed ambiguously into another.

## Completion sequence

`completion_sequence` provides an authenticated ordering point for final results within a ComputeSession.

It is distinct from stream chunk sequence numbers.

Duplicate identical delivery may be tolerated; conflicting ResultCommitments for the same Job/session completion claim require protocol handling and may constitute objective evidence.

## Output delivery

ResultCommitmentV2 does not require an `output_location`.

For ordinary chat:

```text
worker
  -> AI Transport V1
  -> encrypted streamed/final result
  -> wallet
```

Large artifacts may optionally reference separate manifests/storage through another versioned object, but storage is not built into the ordinary V2 result requirement.

## Runtime receipt

`runtime_receipt_hash` may commit to execution metadata such as:

- runtime/build identity;
- resource measurements;
- tokenizer/profile evidence;
- proof/attestation data;
- checkpoint metadata where applicable.

The receipt format is versioned separately.

## Publication boundary

STANDARD ResultCommitmentV2 objects remain off-chain by default.

VERIFIED results may remain off-chain when the VerificationPolicy permits.

HIGH_ASSURANCE policies may explicitly require chain-visible commitment/evidence.

`submitted_block` is therefore not part of normal V2 encoding.

## Payment boundary

ResultCommitmentV2 is evidence used by the wallet before signing ComputeUsageReceiptV1.

It is not itself payment authority.

The wallet verifies:

```text
Job
+ ResultCommitment
+ pricing offer
+ metering evidence
+ VerificationPolicy
```

before acknowledging the charge.

## Privacy

ResultCommitmentV2 contains commitments and execution identity, not plaintext prompts, complete conversation history, wallet-local memory, or the user's master payment identity.

## Invariants

1. One ResultCommitment binds one Job.
2. Worker/operator/model/version/profile identity is explicit.
3. Input commitment must match the Job.
4. Signatures cover the canonical commitment payload.
5. V2 does not require storage-node output persistence.
6. V2 does not require routine on-chain publication.
7. Metering evidence is independently validated before payment.
8. Result evidence does not itself authorize spending.
9. VerificationPolicy controls assurance evidence requirements.
10. V1 field meanings remain reserved and are never silently repurposed.

## First milestone

For SMALL/STANDARD TEXT_INFERENCE:

```text
JobV2
  -> worker execution
  -> streamed output
  -> ResultCommitmentV2
  -> wallet verifies output/token/metering commitments
  -> ComputeUsageReceiptV1
```

All result exchange remains off-chain.
