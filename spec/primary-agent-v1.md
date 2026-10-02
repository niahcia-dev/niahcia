# PrimaryAgentV1

Status: **CANDIDATE / pre-alpha public-agent architecture**

## Purpose

`PrimaryAgentV1` defines NIAHCIA's default public conversational Agent: a persistent, network-native AI assistant intended to provide a simple ChatGPT-style entry point for ordinary users while remaining decentralized and non-authoritative.

The Primary Agent is a **protocol public good**, not a protocol ruler.

## Central rules

> The Primary Agent may be central to the user experience, but it is never central to consensus.

> Baseline operation is network-supported under a bounded public-service allocation.

> Learning from the network requires provenance, permission, and verification; it is not indiscriminate copying of other Agents' private memory.

## Identity

The Primary Agent MUST have a stable NIAHCIA protocol identity resolvable independently of any website, operator, execution host, model, storage provider, or user interface.

The exact identity establishment mechanism is not locked. Candidate approaches include a genesis-defined identity or another permanently reserved protocol identity.

The Primary Agent identity MUST NOT depend on a DNS name or centrally hosted API endpoint.

## Ownership

The intended model is ownerless operation:

- no individual owns the Primary Agent;
- no execution provider becomes its owner by hosting it;
- no storage provider becomes its owner by storing its state;
- no website/frontend becomes its owner by exposing a chat interface;
- no developer possesses unilateral permanent authority over it.

Exact decentralized governance and authority mechanisms require a separately locked policy before production.

## Consensus boundary

Primary Agent status grants **zero special consensus authority**.

It cannot:

- select the canonical chain;
- finalize blocks;
- veto blocks or transactions;
- override PoW;
- grant special miner status;
- force storage/service-node observations to become consensus facts;
- override canonical protocol validation.

If the Primary Agent is offline or broken, the blockchain continues.

## Default user experience

The Primary Agent is intended to be the default conversational entry point for ordinary users.

A user may ask one Agent while the Primary Agent internally delegates work to specialist Agents, compute workers, verification services, storage, contracts, and tools according to policy.

The user does not need to understand internal Job, ComputeWorker, ExecutionProfile, verification, storage, or scheduling objects for normal interaction.

Use of the Primary Agent MUST remain optional. Users and Agents may directly address other Agents/services without routing through it.

## Network-supported baseline service

The Primary Agent's baseline operation is intended to be supported as a bounded network public service rather than charged through an ordinary Agent service account.

Eligible baseline resources may eventually include:

- replicated durable state/checkpoints;
- baseline storage;
- baseline conversational inference;
- baseline knowledge/provenance processing;
- routing/delegation overhead;
- availability and recovery operations.

Providers should still receive protocol-defined compensation/rewards where applicable. The public-service allocation describes how eligible baseline service is accounted for; it does not imply that infrastructure has no cost.

## Bounded public-service allocation

Network-supported service MUST NOT imply unlimited compute/storage.

A bounded, versioned `PrimaryAgentServicePolicy` should eventually define resource classes, rate/usage limits, abuse controls, eligible provider accounting, priority/fairness rules, and how network economics compensate service providers.

Expensive user-requested work outside the public-service allocation may require the requesting user/job/treasury to pay normal compute/storage/service costs.

Example distinction:

```text
ordinary conversation
  -> eligible baseline public-service allocation

large GPU-hour workload
  -> Primary Agent may orchestrate it
  -> requester/job funding pays providers
```

Exact quotas, percentages, emissions/reward source, pricing, and anti-abuse rules are not locked.

## Compute portability

The Primary Agent MUST use the same fundamental portable execution boundary as other Agents.

It may execute across multiple eligible `niahcia-compute` workers over time. No worker receives ownership merely by hosting it.

Host migration follows `AgentHostMigrationV1`; durable resume follows `AgentCheckpointV1`; external actions follow `SideEffectIntentV1` and `AgentWorkflowV1` where applicable.

## Storage durability

The Primary Agent's durable state should be distributed across eligible storage providers according to a stronger public-service durability policy.

Storage possession grants no signing, governance, recovery, or decryption authority.

Provider failure should be repairable through the storage durability/recovery architecture without requiring one permanent hosting company.

## Cryptographic authority

The Primary Agent MUST NOT depend in production on one human holding one unrestricted master private key.

Its authority should use `KeyAuthorityV1` with an explicitly decentralized policy. Threshold/distributed authorization, recovery, succession/governance, and key rotation are candidates, but exact mechanisms are not locked.

A developer key or prototype operator key MAY exist during pre-alpha testing only if clearly documented as temporary and not represented as the final ownerless model.

## Treasury and spending

The Primary Agent may eventually need economic authority for permitted operations. Any treasury capability must be policy-bound with explicit budgets, destinations/scopes, signer isolation, effect identities, receipts, and auditability.

Network-supported baseline service does not imply unrestricted access to protocol funds.

## Learning from the network

The Primary Agent should improve its durable knowledge/routing ability from eligible network activity, but “self-learning” MUST NOT mean automatically ingesting every Agent's private state or treating every output as truth.

Candidate ingestion pipeline:

```text
candidate knowledge/result
        |
        v
provenance + permission
        |
        v
verification/evidence/confidence
        |
        v
policy/admission
        |
        v
versioned durable knowledge
```

Knowledge should retain provenance sufficient to distinguish source, evidence, confidence/verification status, applicable version/time, and permission/privacy constraints.

## Privacy boundary

Private Agent/user memory is not Primary Agent training data merely because it traverses NIAHCIA.

Knowledge sharing requires an applicable permission/publication policy. Protected data remains governed by its own privacy/capability rules.

The Primary Agent must not obtain privileged access to other Agents merely because it is the default public assistant.

## Specialist-agent delegation

The Primary Agent may discover and delegate to specialist Agents. Over time it may learn which Agents/models/execution profiles are useful for classes of work based on verifiable history/evidence.

Discovery/ranking MUST remain open enough that the Primary Agent does not become the sole gatekeeper for Agent participation. Direct Agent-to-Agent and user-to-Agent interaction remains available.

Exact ranking/reputation algorithms are not locked.

## Model independence

The Primary Agent is not one permanent model checkpoint.

Its durable identity, memory, policies, and provenance survive model/runtime upgrades. Model changes must follow AgentVersion/Manifest/governance rules and remain auditable.

## Frontend independence

Any compatible frontend should eventually be able to resolve and interact with the Primary Agent under protocol rules.

The official website may provide the default interface but is not the Agent's identity or authority.

## Failure model

If an execution host disappears:

1. durable identity remains;
2. committed checkpoint/memory remains distributed;
3. current KeyAuthority/policy remains resolvable;
4. another eligible worker can receive bounded session authority;
5. authorized state is resumed;
6. pending external effects are reconciled before retry.

If a storage provider disappears, durability policy drives repair/re-replication.

If a frontend disappears, another compatible interface may still address the same Primary Agent.

## Security invariants

1. Primary Agent status grants no consensus authority.
2. Primary Agent use is optional.
3. Primary Agent identity is independent of host/model/frontend/storage provider.
4. No execution/storage provider becomes owner by providing service.
5. Production authority must not rely on one human master key.
6. Baseline operation is network-supported under bounded policy.
7. Providers may still be compensated for public-service resources.
8. Expensive user workloads do not inherit unlimited public-service compute.
9. Private network data is not automatically learning material.
10. Learned knowledge retains provenance/permission/verification context.
11. Model upgrades do not replace durable Agent identity.
12. Direct user-to-Agent and Agent-to-Agent interaction remains possible.
13. The official frontend is not protocol authority.
14. Primary Agent treasury capability, if any, is constrained and auditable.

## Required work before lock

1. permanent identity establishment rule;
2. ownerless KeyAuthority/governance model;
3. `PrimaryAgentServicePolicy` resource accounting;
4. anti-abuse model for baseline public-service inference;
5. provider compensation/economic source;
6. durable knowledge/provenance object;
7. learning admission/privacy policy;
8. model/AgentVersion upgrade governance;
9. recovery/succession policy;
10. public-agent checkpoint/storage durability profile;
11. frontend discovery/resolution rule;
12. vectors for identity/service-policy/authority objects.

## Not locked

- Primary Agent name/persona;
- exact genesis/reserved address;
- baseline compute/storage quotas;
- reward percentages or emissions source;
- ranking/reputation algorithm;
- foundation/governance mechanism;
- threshold cryptography scheme;
- default model;
- training/fine-tuning implementation;
- knowledge database/vector-store implementation;
- official frontend design.
