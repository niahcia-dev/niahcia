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
