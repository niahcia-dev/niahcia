# ComputePaymentRiskV1

Status: **CANDIDATE / pre-alpha job payment-risk policy**

## Purpose

`ComputePaymentRiskV1` defines when a compute Job may use a single completion receipt and when a larger Job should reserve funds or use coarse staged acknowledgements.

This is a payment-risk policy, not an AI correctness class and not a chain-consensus authority.

The central rule is:

> payment complexity should scale with economic exposure, not with every token or prompt.

## Risk classes

V1 defines three economic risk classes:

```text
SMALL
RESERVED
STAGED
```

These names describe payment handling only.

### SMALL

Intended for ordinary chat and inexpensive inference.

Characteristics:

- hard `max_price`;
- no separate per-Job escrow reservation beyond the worker-bound channel;
- worker performs the Job;
- result is returned;
- wallet verifies the result/metering;
- wallet signs one cumulative usage receipt.

This is the preferred default for interactive chat.

### RESERVED

Intended for Jobs whose maximum cost is high enough that worker exposure is undesirable, but whose execution can still settle as one completed unit.

Before execution, the wallet cryptographically reserves up to the Job's authorized maximum within the existing ComputeChannel.

The reservation:

- reduces channel funds available to other Jobs;
- is bound to one Job;
- expires;
- cannot exceed Job/authorization/channel ceilings;
- does not transfer value to the worker by itself.

After valid completion:

- actual charge is computed;
- the wallet acknowledges the actual charge;
- unused reserved amount is released.

On failure/expiry, the reservation is released according to policy.

### STAGED

Intended for long or expensive workloads where neither side should carry the entire Job exposure until the end.

The Job defines a small number of explicit stages or checkpoints.

Examples:

```text
stage 1  accepted/setup
stage 2  progress checkpoint
stage 3  final result
```

Each stage has:

- a cumulative maximum;
- objective completion evidence;
- a wallet acknowledgement;
- monotonic spending.

V1 MUST NOT require one payment signature per token, inference step, training iteration, or streamed chunk.

## Candidate JobReservationV1

```text
JobReservationV1
- schema_version
- reservation_id
- channel_id
- authorization_id
- job_id
- worker_id
- reserved_amount
- created_height
- expires_at
- sequence
- status
- channel_signature
```

A reservation is a hold against already authorized channel value. It does not create additional value or independently pay the worker.

## Candidate staged payment descriptor

```text
JobPaymentStageV1
- stage_index
- cumulative_maximum
- evidence_requirement
- stage_deadline
```

The Job or referenced PaymentPlan commits to the complete ordered stage schedule before execution begins.

A worker MUST NOT insert a new chargeable stage after acceptance.

## Choosing the risk class

The wallet chooses the payment-risk class before execution based on policy inputs such as:

- Job `max_price`;
- expected execution duration;
- workload type;
- worker policy;
- VerificationPolicy;
- whether output can be checkpointed objectively;
- remaining channel balance.

Exact production thresholds are wallet/worker policy and MUST NOT be silently hard-coded as consensus constants.

A worker MAY refuse Jobs above its one-shot exposure limit unless RESERVED or STAGED handling is used.

## Recommended initial policy

For the first milestone:

- ordinary TEXT_INFERENCE uses SMALL;
- expensive one-shot inference may use RESERVED;
- TRAINING, FINE_TUNING, and other long-running workloads are deferred until STAGED semantics are implemented and tested.

No production numeric threshold is locked.

Development wallets may use a configurable threshold such as:

```text
if max_price <= local_small_job_limit:
    SMALL
else:
    RESERVED
```

This is implementation policy, not consensus.

## Reservation rules

A valid reservation MUST:

1. reference one worker-bound ComputeChannel;
2. reference one PaymentAuthorization;
3. reference exactly one Job;
4. be signed by the channel authority;
5. fit within remaining channel value;
6. fit within remaining authorization value;
7. expire no later than the Job/channel authorization permits;
8. be replay-protected.

A channel MUST treat reserved-but-unsettled value as unavailable for new reservations/charges.

## Failure handling

SMALL:
- worker failure -> no successful-job receipt;
- wallet disappearance after result -> worker may lose that Job fee.

RESERVED:
- worker failure/timeout -> reservation releases after policy/expiry;
- wallet disappearance before final acknowledgement -> reservation does not automatically become worker payment;
- future versions may define stronger objective claim conditions if needed.

STAGED:
- worker earns only stages validly acknowledged or otherwise authorized by the committed stage policy;
- unfinished later stages do not retroactively invalidate valid earlier stages.

## Relation to verification

Payment risk and answer assurance are separate axes.

Examples:

```text
SMALL + STANDARD
SMALL + VERIFIED
RESERVED + STANDARD
RESERVED + VERIFIED
STAGED + HIGH_ASSURANCE
```

A high-cost Job is not automatically high-assurance, and a cheap Job may still request stronger verification.

## Relation to pricing

ComputePricingV1 still determines the valid charge formula and hard maximum.

ComputePaymentRiskV1 determines how economic exposure is managed while the Job is running.

Neither permits settlement above:

```text
Job.max_price
PaymentAuthorization limits
ComputeChannel remaining authorized value
```

## Relation to workload types

V1 should prioritize:

- TEXT_INFERENCE;
- EMBEDDING;
- RERANKING;
- other short bounded inference.

VISION/AUDIO inference may use SMALL or RESERVED depending on price/exposure.

TRAINING and FINE_TUNING SHOULD remain disabled for production payment until staged/checkpoint semantics, artifact commitments, interruption/restart behavior, and resource accounting are separately specified.

## Invariants

1. Payment-risk class does not affect chain consensus authority.
2. SMALL remains the default for ordinary chat.
3. Reservations hold existing authorized value; they never mint or create credit.
4. Reserved value cannot be double-allocated.
5. A reservation is not itself worker payment.
6. Stages are committed before execution and cannot be added unilaterally.
7. Per-token payment signatures are not required.
8. Payment risk and VerificationPolicy are independent dimensions.
9. Long-running workloads may be deferred rather than forcing premature payment complexity.
10. Numeric risk thresholds are policy/configuration until explicitly standardized.

## First milestone

Implement only:

```text
SMALL
```

for normal text inference.

Then add:

```text
RESERVED
```

for higher-value one-shot Jobs.

Do not implement STAGED/TRAINING/FINE_TUNING until the simpler end-to-end compute/payment path is proven.
