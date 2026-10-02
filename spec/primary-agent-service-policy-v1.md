# PrimaryAgentServicePolicyV1

Status: **CANDIDATE / pre-alpha network public-service policy**

## Purpose

`PrimaryAgentServicePolicyV1` defines the resource boundary for NIAHCIA's Primary Agent as a persistent decentralized public service.

The policy exists to make baseline Primary Agent availability a network-supported function while preventing that status from becoming unlimited compute, storage, bandwidth, or economic authority.

No reward percentage, emission share, quota, price, or production funding mechanism is locked by this document.

## Central rules

> The network supports a bounded baseline service envelope for the Primary Agent.

> Public-service status grants resource eligibility, not consensus authority or unlimited capacity.

> Providers must be able to account for eligible service without trusting the Primary Agent's own assertion that work qualifies.

## Candidate policy object

```text
PrimaryAgentServicePolicyV1
- schema_version
- policy_id
- primary_agent_id
- effective_from
- expires_at?
- service_classes_root
- eligibility_policy_root
- accounting_policy_root
- fairness_policy_root
- abuse_policy_root
- provider_compensation_policy_root
- upgrade_policy_id
```

Exact NCE/1 field IDs and encoding are not locked.

## Service classes

Candidate baseline service classes include:

```text
CONVERSATIONAL_INFERENCE
STATE_CHECKPOINT
DURABLE_STORAGE
KNOWLEDGE_PROCESSING
ROUTING_AND_DISCOVERY
VERIFICATION_OVERHEAD
RECOVERY_AND_MIGRATION
AVAILABILITY_MAINTENANCE
```

Each class should eventually define measurable units and policy bounds. Different classes need not use the same accounting mechanism.

## Baseline versus user-funded work

The Primary Agent may orchestrate work larger than its public-service envelope, but public-service identity MUST NOT automatically make that work eligible for network-supported accounting.

Example:

```text
short ordinary interaction
    -> may qualify under baseline service policy

large training job / long simulation / bulk generation
    -> separate funded Job
    -> requester/authorized treasury supplies budget
```

The boundary should be machine-evaluable where practical rather than dependent on subjective worker judgment.

## Resource envelopes

Policies may bound service using combinations of:

- compute units;
- accelerator time;
- token/work units where reproducibly measurable;
- memory footprint;
- storage bytes and duration;
- bandwidth;
- concurrent Jobs;
- verification cost;
- per-identity/session windows;
- global epoch windows;
- recovery/migration reserves.

Exact units and constants require benchmarking and economic modeling before lock.

## Provider eligibility

No single provider should be permanently designated as the Primary Agent host.

Eligible providers should satisfy the applicable worker/storage/service profile and verification/accounting requirements. Selection should preserve replaceability and avoid hidden coordinator dependence.

Provider eligibility does not grant access to master Agent authority or unrestricted memory.

## Accounting

Eligible public-service work should produce verifiable accounting evidence sufficient to establish, where applicable:

- policy ID;
- Primary Agent identity;
- Job/work identity;
- service class;
- provider identity;
- measured resource usage;
- execution/storage/verification evidence;
- duplicate/retry relationship;
- settlement eligibility.

A provider MUST NOT receive multiple allocations for the same logical work merely because a Job was retried, migrated, or duplicated unless policy explicitly recognizes additional verified work.

## Provider compensation boundary

Providers may receive protocol-defined compensation for eligible public-service work.

This specification does not choose whether compensation ultimately comes from emissions, fees, a protocol treasury, service rewards, or another mechanism. That decision must be modeled against chain security, inflation, Sybil incentives, service quality, and long-term sustainability.

Compensation rules should reward measurable service rather than self-reported capacity.

## Fairness

Public-service capacity should remain useful to ordinary users without allowing one user, Agent, operator, API client, or application to consume the entire envelope.

Candidate fairness mechanisms include bounded concurrency, adaptive queues, per-session/identity budgets, proof-of-human or reputation mechanisms where appropriate, stake/deposit mechanisms for costly operations, and graceful degradation.

No single identity mechanism is assumed Sybil-proof.

## Abuse resistance

Threats include:

- automated conversation flooding;
- Sybil users resetting limits;
- prompts designed to trigger expensive delegation;
- recursive Agent-to-Agent loops;
- artificial verification amplification;
- storage spam;
- repeated checkpoint/migration churn;
- duplicate provider claims;
- collusion between requester and provider;
- deliberately induced failures intended to earn repeated service compensation;
- disguising large workloads as many baseline requests.

Policy must bound resource amplification even when identities are cheap.

## Delegation containment

When the Primary Agent delegates to specialist Agents or tools, the delegated work inherits an explicit budget/resource envelope.

A specialist cannot recursively delegate unlimited public-service work merely because the call originated from the Primary Agent.

Nested workflows must preserve aggregate accounting.

## Failure and recovery reserve

A portion of policy capacity may be reserved for continuity operations such as checkpoint retrieval, state repair, provider replacement, migration, and recovery.

Normal user traffic should not be able to exhaust all capacity needed to restore the Primary Agent after infrastructure failure.

Exact reserve sizes are not locked.

## Degraded operation

When eligible capacity is scarce, the Primary Agent should degrade predictably rather than acquire special consensus power or silently exceed policy.

Candidate degradation order may include:

1. reduce optional background knowledge work;
2. defer non-urgent verification/reindexing;
3. reduce expensive specialist delegation;
4. queue ordinary requests;
5. preserve identity/state/checkpoint/recovery functions as long as possible.

Exact priorities require later policy review.

## Storage policy

Baseline public-service storage should prioritize the minimum durable state required for continuity plus eligible shared knowledge/provenance under retention rules.

Unbounded user uploads or arbitrary third-party storage are not automatically covered merely because they are referenced by the Primary Agent.

## Economic isolation

Primary Agent public-service accounting MUST remain distinguishable from the Agent's own treasury/spending capabilities.

A compromised execution host must not be able to convert public-service eligibility into unrestricted currency transfers.

## Upgrade safety

Service policy changes require explicit versioning and activation rules. A model/runtime upgrade must not silently change economic eligibility or resource limits.

Nodes/providers should be able to determine which policy governed historical service claims.

## Security invariants

1. Primary Agent public-service status grants no consensus authority.
2. Baseline service is bounded by explicit policy.
3. Large workloads do not automatically qualify because the Primary Agent requested them.
4. Providers are compensated only for policy-eligible measurable service.
5. Retry/migration cannot silently reset resource accounting.
6. Delegation preserves aggregate resource bounds.
7. Cheap Sybil identities cannot create unlimited network-supported capacity.
8. Recovery/continuity resources can be protected from ordinary traffic exhaustion.
9. Public-service accounting cannot become unrestricted treasury authority.
10. Policy upgrades are versioned and auditable.
11. No permanent privileged provider is required.
12. Resource scarcity produces defined degradation, not protocol authority escalation.

## Required work before lock

1. canonical service-class IDs;
2. measurable accounting units;
3. Job/accounting receipt integration;
4. duplicate/retry accounting rules;
5. provider eligibility rules;
6. scheduler/fairness integration;
7. Sybil/abuse simulations;
8. recursive delegation limits;
9. recovery reserve policy;
10. storage-retention integration;
11. compensation-source economic modeling;
12. canonical policy vectors.

## Not locked

- resource quotas;
- block/emission percentages;
- fee shares;
- provider prices;
- token/work-unit formulas;
- queue algorithm;
- identity/rate-limit mechanism;
- proof-of-human mechanism;
- exact degradation thresholds;
- compensation source;
- production activation height.
