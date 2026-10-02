# PaymentPlan

## Purpose

`PaymentPlan` describes job-level economic constraints and settlement destinations without hard-coding a universal reward split.

## Canonical fields

```text
PaymentPlan
- schema_version
- payment_plan_id
- currency
- max_total
- escrow_amount
- executor_budget
- verifier_budget
- storage_budget
- routing_budget
- child_agent_budget
- creator_fee
- protocol_fee
- refund_policy
- settlement_policy
- expiry_policy
- plan_hash
```

## Principles

CPU block rewards are not represented by job PaymentPlans.

PaymentPlan governs application/service economics such as:

- compute execution
- verification
- storage/retrieval
- routing
- child-agent calls
- creator revenue
- optional protocol fees

## Budgets

Budgets are ceilings unless the settlement policy explicitly defines fixed allocations.

Unused budget SHOULD be refundable according to `refund_policy`.

## Child-agent calls

`child_agent_budget` caps aggregate downstream agent invocation unless a stricter policy applies.

A Job/Agent policy SHOULD additionally limit:

- maximum call depth
- maximum child job count
- maximum per-child spend

## Settlement

Settlement policy may support:

- fixed price
- metered price
- standing worker price
- auction/bid result
- session/channel accounting
- custom contract logic

## Invariants

1. Total settlement MUST NOT exceed authorized escrow/budget.
2. Downstream child jobs cannot create authority to spend beyond the parent budget.
3. CPU mining economics remain separate.
4. Payment accounting MUST be auditable from signed/on-chain records.
5. Failed/expired jobs follow explicit refund rules.

## Prototype 0

Prototype 0 uses test currency and simple compute/storage escrow. Production issuance and fee percentages are intentionally undecided.


## Relationship to compute channels

ComputeChannelV1 and PaymentAuthorizationV1 separate payment authority from Job pricing/allocation.

Conceptually:

```text
funding account
  -> ComputeChannelV1
      -> PaymentAuthorizationV1
          -> Job
              -> PaymentPlan
                  -> accepted usage receipt / settlement
```

`ComputeChannelV1` establishes an enforceable spending ceiling across many Jobs.

`PaymentAuthorizationV1` delegates a narrower portion of that authority to a requester/session/Agent identity.

`PaymentPlan` describes how one Job's permitted spend is allocated and settled among eligible participants.

A PaymentPlan MUST NOT expand the amount or scope authorized by its parent PaymentAuthorization/ComputeChannel.


## Relationship to ComputePricingV1

ComputePricingV1 defines the accepted service price and metering formula.

PaymentPlan does not authorize spending beyond that price or beyond PaymentAuthorization/ComputeChannel ceilings.

For an ordinary primary-worker Job:

```text
signed price offer
  -> Job max_price
  -> measured valid usage
  -> calculated final charge
  -> PaymentPlan settlement allocation
```

The final allocated settlement MUST NOT exceed the accepted final charge.
