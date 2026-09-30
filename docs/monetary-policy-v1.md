# NIAHCIA Monetary Policy V1

Status: **pre-alpha architecture lock; numerical emission constants intentionally pending simulation**

This document defines the monetary-policy architecture for NIAHCIA. It deliberately separates consensus-security issuance from AI-compute and storage/service economics.

## Principles

1. CPU proof-of-work is a real consensus-security service and MUST receive a protocol-defined block subsidy while the subsidy schedule is active.
2. GPU AI execution is not consensus proof-of-work. AI workers MUST NOT receive a percentage of the CPU block subsidy merely for existing.
3. Storage/service nodes are not a second consensus authority. Service rewards MUST NOT grant consensus weight or a percentage of the CPU block subsidy merely for registration.
4. CPU consensus rewards, GPU AI-compute rewards, and storage/service rewards are distinct economic schedules with independently auditable accounting.
5. Monetary-policy arithmetic MUST use the integer monetary rules defined by `monetary-and-fee-primitives-v1.md`.
6. Consensus issuance MUST be deterministic from canonical chain state. No oracle, fiat price, GPU utilization measurement, administrator, website, or off-chain scheduler may alter the CPU block subsidy.

## Target block interval

The current V1 design target is approximately **30 seconds per canonical CPU-PoW block**.

Emission constants MUST be specified in blocks, not wall-clock dates. Documentation MAY provide approximate calendar durations using the 30-second target, but consensus MUST never depend on wall-clock calendar dates for reward transitions.

If a later protocol version changes the target block interval, it MUST explicitly define whether and how the emission curve is preserved.

## Three economic lanes

### Lane A — CPU consensus security

A canonical PoW block may create the deterministic CPU block subsidy defined by the active emission schedule. The block producer also receives whatever transaction-fee portion the active fee policy assigns to block production.

CPU subsidy eligibility derives only from producing a valid canonical PoW block. Hashrate, miner identity, hardware count, geographic location, AI participation, storage participation, or service-node registration MUST NOT independently mint additional CPU-consensus issuance.

### Lane B — GPU AI compute

GPU/AI workers are paid for accepted AI jobs under the Job/Payment/Verification protocol. The primary long-run source of AI-worker compensation SHOULD be job-funded payment rather than an automatic percentage of every block subsidy.

Any temporary network-bootstrap subsidy for AI compute would be a separate, explicitly bounded issuance program. It MUST have its own consensus-visible budget and termination rule and MUST NOT silently dilute or redirect the CPU block subsidy.

No AI job result may create arbitrary NIAH. A job settlement may transfer existing funds or consume a separately authorized issuance budget only under an explicitly versioned protocol rule.

### Lane C — storage and proof-of-service

Storage/service nodes are compensated for objectively verified service under the Proof-of-Service protocol. Their compensation SHOULD primarily come from service payments/fees.

Any bootstrap issuance for proof-of-service MUST be separately budgeted, objectively claimable, capped or otherwise deterministically bounded, and independent of consensus voting power.

Storage/service rewards MUST NOT make service nodes a second block-consensus authority.

## Genesis allocation

No founder, developer, organization, treasury, exchange, marketing, investor, AI-worker, storage-node, or service-node allocation is authorized by this V1 architecture document.

Until a later monetary-policy vector explicitly authorizes a nonzero genesis allocation, implementations MUST treat **genesis spendable allocation as zero** for production-policy planning.

Devnet/test fixtures MAY create test balances, but those balances are test-only and MUST NOT be interpreted as a production premine or production supply commitment.

## Main emission shape

NIAHCIA V1 SHOULD use a **smooth deterministic decay** for the CPU block subsidy rather than abrupt large halving cliffs.

The exact function and constants remain pending numerical simulation. Before activation, the protocol MUST lock all of the following in a canonical vector:

- initial CPU block subsidy;
- exact integer decay formula;
- decay precision/rounding rule;
- main-emission target or transition condition;
- tail-subsidy amount, if any;
- transition height/rule;
- cumulative issuance vectors at selected heights.

The implementation MUST derive the subsidy from block height and fixed consensus constants only.

## Long-run security floor

The V1 architecture selects a **nonzero CPU-PoW tail subsidy** as the intended long-run design, subject to final numerical simulation before activation.

Rationale: NIAHCIA should not assume that transaction fees alone will always provide an adequate security budget. A fixed per-block tail subsidy provides a predictable minimum native incentive for CPU miners. Because a fixed nominal tail subsidy becomes a smaller percentage of an ever-growing supply, its percentage issuance rate declines over time.

Accordingly, production NIAHCIA is intended to have no finite terminal maximum supply once tail emission begins. Wallets, explorers, APIs, and documentation MUST distinguish between:

- cumulative issued supply at a given height; and
- a nonexistent finite `max_supply` ceiling under tail emission.

The final tail amount MUST be chosen only after modeling security budget, expected block frequency, early distribution, and long-run annual issuance.

## Supply accounting

Consensus MUST expose or make derivable at least:

`issued_supply(height)`

This is the sum of all consensus-authorized newly created NIAH through the specified height, including CPU subsidy and any separately authorized protocol issuance programs.

Transfers and fee redistribution do not change issued supply. Burns, if introduced by a later fee policy, MUST be accounted separately so clients can distinguish gross issuance from net circulating monetary effects.

Recommended accounting concepts are:

- `gross_issued_supply` — all protocol-created NIAH;
- `protocol_burned_supply` — all provably destroyed NIAH under consensus rules;
- `net_protocol_supply = gross_issued_supply - protocol_burned_supply`.

Lost keys are not protocol burns and MUST NOT be subtracted from consensus supply accounting.

## Fee-policy separation

The existence of an execution-engine base-fee mechanism does not by itself decide NIAHCIA monetary policy.

A separate fee-policy specification MUST lock:

- effective gas-price calculation;
- base-fee handling;
- miner/block-producer fee share;
- whether any fee component is burned;
- rounding and overflow behavior;
- reorg accounting.

Until that specification is locked, implementations MUST NOT describe execution-engine fee burning as NIAHCIA's final economic policy.

## No hidden issuance

Every code path capable of increasing total NIAH supply MUST correspond to an explicit, versioned consensus rule.

The following MUST NOT mint NIAH by themselves:

- RPC calls;
- administrator actions;
- website actions;
- scheduler decisions;
- worker registration;
- service-node registration;
- agent creation;
- model registration;
- storage commitments without an authorized reward rule;
- AI results without an authorized payment/issuance rule.

## Required numerical work before production lock

Before public testnet economics are treated as production candidates, generate an emission simulation covering at least:

- first 1,000 blocks;
- 1 day, 30 days, 1 year, 5 years, 10 years, and 25 years at target block interval;
- cumulative CPU issuance;
- annual newly issued NIAH;
- annual issuance as a percentage of existing supply;
- transition into tail emission;
- tail issuance per day and year;
- sensitivity to actual average block intervals differing from target;
- integer rounding behavior at every decay boundary.

At least one canonical machine-readable vector MUST accompany the final constants.

## Compatibility rule

The architecture locked here is part of the NIAHCIA economic contract:

- CPU consensus issuance is separate from GPU AI economics;
- CPU consensus issuance is separate from storage/service economics;
- service nodes do not gain consensus authority from rewards;
- genesis production allocation defaults to zero unless explicitly superseded before launch;
- the intended CPU emission uses smooth decay followed by a nonzero tail subsidy;
- issuance is deterministic and integer-accounted.

Exact numerical subsidy constants are intentionally **not** locked by this document. They require simulation and a separate canonical monetary-policy vector before production activation.
