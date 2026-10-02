# Economic Architecture

NIAHCIA deliberately separates compensation for different network services.

## CPU security economy

CPU miners receive:

- protocol block subsidy
- transaction gas fees

Their work secures consensus, transaction ordering, EVM state, AI settlement, and service-node state.

## Compute economy

Compute workers receive payment from AI workloads.

Sources may include:

- direct user job fees
- agent budgets
- smart-contract job escrow
- temporary bootstrap incentives if explicitly enabled by network policy

Compute rewards are not a fixed percentage of the CPU block subsidy.

## Service economy

Service nodes receive payment for measurable services such as:

- model persistence
- retrieval bandwidth
- memory persistence
- artifact storage
- routing
- availability commitments

Owning collateral by itself does not create a right to rewards.

## Agent economy

Agents may define application-level pricing, including:

- free/subsidized use
- fixed request price
- metered compute price
- subscription or session budgets at the application layer
- agent-to-agent child-job budgets

## Generic PaymentPlan

The protocol object model should support flexible payment buckets:

```text
PaymentPlan
├── max_total
├── escrow
├── executor_budget
├── verifier_budget
├── storage_budget
├── child_agent_budget
├── creator_fee
├── protocol_fee
├── refund_policy
└── settlement_policy
```

No global hard-coded percentage split should be assumed by the protocol specification.

## Prototype 0

Prototype 0 uses test currency only. Its purpose is to verify accounting and settlement correctness, not finalize mainnet monetary policy.
