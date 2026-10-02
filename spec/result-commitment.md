# ResultCommitment

## Purpose

`ResultCommitment` is the signed binding between a Job, an executor, the exact execution context, and the produced output/artifacts.

## Canonical fields

```text
ResultCommitment
- schema_version
- commitment_id
- job_id
- worker_id
- operator_id
- model_id
- execution_profile_id
- input_hash
- output_hash
- token_or_artifact_hash
- runtime_receipt_hash
- output_manifest_hash
- output_location
- started_at
- completed_at
- submitted_block
- nonce_or_salt_commitment
- signature
```

## Canonical output

For token-generating inference, verification SHOULD commit to canonical token IDs or another explicitly versioned binary output representation rather than rendered JSON/text formatting.

The canonical representation is defined by the ExecutionProfile output format.

## Commit/reveal

Where a VerificationPolicy requires independent redundant execution, workers first publish a blinded commitment such as:

```text
H(canonical_result || salt)
```

and reveal only after the commit phase closes.

This prevents later workers from simply copying an earlier revealed result.

## Output storage

Large outputs remain off-chain.

`output_manifest_hash` and `output_location` identify retrievable content, while hashes provide integrity.

## Runtime receipt

`runtime_receipt_hash` may commit to execution metadata such as:

- runtime/build
- model/profile
- resource measurements
- proof/attestation data
- intermediate checkpoints

Receipt formats are versioned separately.

## Invariants

1. The commitment MUST bind to one `job_id`.
2. Model and ExecutionProfile identity MUST be explicit.
3. Signatures MUST cover the canonical commitment payload.
4. Output location is not trusted without hash verification.
5. A revealed result MUST match its prior blinded commitment when commit/reveal applies.

## Prototype 0

Prototype 0 compares independently generated canonical token-sequence commitments under the REDUNDANT verification policy.


## Metering evidence

When payment is metered, the ResultCommitment or its referenced runtime receipt MUST commit to the canonical usage evidence required by ComputePricingV1.

For TOKEN_METERED inference this should include reproducible input/output token counts or the canonical token sequence from which those counts are derived.

The worker's signed ResultCommitment authenticates its claim; the wallet/verifier must still recompute or validate the applicable metering evidence before signing a payment receipt.
