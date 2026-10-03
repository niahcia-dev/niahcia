# Economic Architecture

NIAHCIA separates compensation for distinct network services.

## CPU security economy

CPU miners may receive protocol issuance and native transaction fees according to the active monetary/fee policy.

Their work secures consensus and transaction ordering. AI workers and service providers do not receive consensus authority through payment or collateral.

## Compute economy

Compute workers are paid from user/Agent-authorized value, initially through bounded ComputeChannels.

The compute service payment is separate from the native transaction fee used to open/settle/refund a channel.

Compute settlement does not mint value.

## Service economy

Future service/storage providers may receive payment for measurable storage, retrieval, routing, indexing, or availability work.

Collateral by itself must not create an automatic reward entitlement or consensus privilege.

## Agent economy

Agents may use bounded budgets and application-level pricing such as free/subsidized use, metered compute, session budgets, or Agent-to-Agent child-job budgets.

## Payment-plan flexibility

The broader object model may support separate buckets for executor, verifier, storage, child-Agent, creator, protocol, and refund policies.

No global hard-coded percentage split should be assumed unless explicitly specified by activated network policy.

## Current milestone

The first paid-compute milestone focuses on exact value conservation:

```text
funding account
  -> locked ComputeChannel value
  -> worker payment + wallet refund
```

Mainnet issuance and fee policy remain separate consensus work.
