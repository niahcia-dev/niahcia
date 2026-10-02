# ComputePricingV1

Status: **CANDIDATE / pre-alpha compute pricing and metering**

## Purpose

`ComputePricingV1` defines how a worker advertises a signed, expiring price schedule and how a wallet determines a hard maximum charge before execution.

The central rule is:

> the wallet authorizes a maximum before execution; final settlement may be lower based on measured usage, but never higher.

## Price advertisement

A worker advertisement MAY reference one or more signed pricing offers.

Candidate fields:

```text
ComputePriceOfferV1
- schema_version
- offer_id
- worker_id
- operator_id
- service_type
- model_id
- execution_profile_id
- billing_mode
- base_fee
- input_unit_price
- output_unit_price
- time_unit_price
- minimum_charge
- maximum_charge
- valid_from
- expires_at
- offer_nonce
- signature
```

All monetary fields use integer `aniah`.

## Billing modes

V1 should support a small explicit set:

```text
FIXED
TOKEN_METERED
TIME_METERED
HYBRID
```

`FIXED`
- one agreed charge for a Job.

`TOKEN_METERED`
- charge derived from canonical input/output token counts under the referenced model/tokenizer/profile.

`TIME_METERED`
- charge derived from a canonical billable execution duration defined by the applicable runtime receipt format.

`HYBRID`
- base fee plus one or more metered components.

A billing mode MUST define its measurable units precisely enough that wallet and worker can independently recompute the charge.

## Pre-execution maximum

Before a wallet sends a chargeable Job, it calculates or receives a quoted upper bound.

Conceptually:

```text
estimated/declared limits
  -> pricing offer
  -> computed job maximum
  -> compare with PaymentAuthorization.max_per_job
  -> authorize Job
```

The Job MUST bind the exact `offer_id` and a `max_price`.

The worker MUST reject the Job before execution if it cannot accept those terms.

After acceptance, the worker MUST NOT charge more than the Job's `max_price`, even if actual resource usage is higher than expected.

## Metering

For TOKEN_METERED billing, input/output token counts must use the tokenizer and canonical representation defined by the referenced Model/ExecutionProfile.

For TIME_METERED billing, wall-clock time reported solely by the worker is not sufficient for high-assurance accounting unless the applicable receipt/verification policy defines trustworthy measurement evidence.

For the first milestone, TOKEN_METERED or FIXED billing is preferred because the unit is easier for wallet and worker to reproduce.

## Charge calculation

Example TOKEN_METERED formula:

```text
charge =
  base_fee
  + input_tokens  * input_unit_price
  + output_tokens * output_unit_price
```

Then:

```text
final_charge =
  clamp(charge, minimum_charge, job.max_price)
```

If the uncapped calculated charge exceeds `job.max_price`, the worker receives at most `job.max_price`.

Implementations MUST use checked integer arithmetic.

## Output limits

To avoid forcing a worker to continue generating unpaid output after the authorized maximum is reached, a Job SHOULD include an output/resource limit compatible with pricing.

For token generation this may include:

```text
max_output_tokens
```

The wallet can choose that limit so the worst-case valid charge remains below `max_price`.

Workers SHOULD stop before producing usage that cannot be charged under the accepted authorization.

## Price changes

Workers may publish new offers at any time.

A new offer does not alter already accepted Jobs or an existing ComputeSession unless the wallet explicitly accepts the new offer.

The wallet may end/reselect a session when:

- the current offer expires;
- the worker withdraws the offer for future Jobs;
- a replacement offer exceeds wallet policy;
- price changes make another worker preferable.

Existing accepted usage remains governed by the offer/job terms valid at acceptance.

## Signed quote option

For dynamic pricing, the worker MAY issue a short-lived signed Job quote:

```text
ComputeQuoteV1
- quote_id
- offer_id
- job_parameters_hash
- quoted_maximum
- expires_at
- worker_signature
```

The wallet then binds `quote_id` and `quoted_maximum` into the Job.

The first implementation does not require dynamic quotes when a standing offer is sufficient.

## Privacy

Pricing offers are public service advertisements and need not reveal the user's identity.

A worker learns the accepted Job/session identity and the terms used for that Job. It does not need the wallet's unrelated balances or other session spending history.

## Failure rules

- rejected before execution: no compute charge;
- worker timeout before valid result: no successful-job charge;
- wallet cancels before worker acceptance: no charge;
- worker accepts then exceeds its own resource estimate: still bounded by `max_price`;
- result invalid under VerificationPolicy: successful-result payment is not authorized;
- partial/streaming payment is deferred unless explicitly defined by a future policy.

## Invariants

1. Every chargeable Job has a hard maximum known before execution.
2. A worker cannot unilaterally increase an accepted Job's price.
3. Final charge never exceeds Job max, PaymentAuthorization max, or remaining ComputeChannel balance.
4. Price offers are signed and expire.
5. Metering units are defined by versioned model/profile/runtime rules, not free-form worker claims.
6. Price changes apply only to future accepted Jobs unless explicitly renegotiated.
7. Checked integer `aniah` arithmetic is mandatory.
8. Failed or invalid work does not silently become a successful-job charge.
9. Ordinary pricing grants no consensus authority.
10. The wallet may reselect another worker rather than accept a new price.

## First milestone

Use either:

```text
FIXED
```

or:

```text
TOKEN_METERED
base_fee
input-token price
output-token price
max_output_tokens
hard max_price
```

Prefer TOKEN_METERED once canonical tokenizer/output accounting is available. Avoid time billing until trustworthy runtime metering is specified.
