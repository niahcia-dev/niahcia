# StorageHealthV1

Status: **CANDIDATE / pre-alpha observational and recovery model**

## Purpose

`StorageHealthV1` defines a common vocabulary for describing whether a content-addressed object covered by a storage policy/agreement appears sufficiently retrievable and durable, and for coordinating permissionless recovery without turning storage nodes into consensus authorities.

Storage health is derived from verifiable service observations. It is not blockchain finality.

## Candidate states

```text
HEALTHY
DEGRADED
AT_RISK
RECOVERING
UNAVAILABLE
EXPIRED
```

### HEALTHY

Observed eligible providers/reconstruction capacity meet or exceed the agreement's target under the applicable observation policy.

### DEGRADED

Observed service is below target but remains above the minimum safety/reconstruction threshold. The object is still recoverable and repair should be economically encouraged.

### AT_RISK

Observed service is at or below the configured minimum safety threshold while enough valid content is still known to attempt repair. New repair work should receive priority under the applicable policy.

### RECOVERING

One or more eligible providers are actively reconstructing/retrieving/verifying the exact committed object/profile to restore service.

`RECOVERING` is not proof that recovery will succeed.

### UNAVAILABLE

Current valid observations cannot establish enough retrievable committed data to reconstruct the object under its storage profile.

This state describes availability knowledge; it does not prove that no copy exists anywhere.

### EXPIRED

The applicable agreement has ended without an active renewal/replacement obligation. Expiration is an economic/service state, not an instruction to rewrite history or delete data required elsewhere.

## Candidate observation inputs

A health computation may consume versioned evidence including:

- valid provider storage commitments;
- successful storage challenges/responses;
- successful retrieval proofs/tests;
- provider liveness windows;
- reconstruction-threshold evidence for the storage profile;
- service epoch reports;
- active agreement duration/renewal state.

Provider self-report alone is insufficient.

## Determinism boundary

If StorageHealth becomes a consensus-relevant settlement input, the exact observation window, eligible evidence set, transition thresholds, tie behavior, and arithmetic MUST be deterministic and covered by canonical vectors before activation.

Before that freeze, implementations may expose health as derived operational telemetry provided they label it non-authoritative and do not use it to create incompatible economic state.

## Recovery opportunity

A recovery opportunity exists when:

1. an active agreement is below target;
2. sufficient valid content remains retrievable to satisfy the storage profile's reconstruction rules;
3. an eligible provider can obtain and verify that content;
4. the provider can create the required commitment/service evidence.

Recovery should not require permission from the failed provider.

## Recovery procedure

Conceptually:

```text
observe degradation
 -> identify exact object_root + storage_profile
 -> discover surviving providers/chunks
 -> retrieve committed content
 -> verify every required commitment
 -> reconstruct exact object if necessary
 -> establish local durable storage
 -> publish/register applicable storage commitment
 -> answer challenges/retrieval tests
 -> become eligible service provider
 -> health observation improves
```

A recovery participant MUST NOT modify content to repair a corrupt object. Corrupt/mismatched bytes are rejected and recovery continues from other valid sources.

## Agent continuity

When the covered content belongs to an Agent's portable environment, storage recovery preserves bytes/commitments only. It does not grant the recovery provider:

- controller status;
- capability authority;
- payment authority;
- memory decryption keys;
- succession rights;
- permission to alter AgentVersion/AgentManifest commitments.

This separation is required for autonomous agents to purchase persistence safely.

## Anti-gaming requirements

Future compensable health/recovery rules must address:

- Sybil providers pretending to be independent replicas;
- one operator placing all replicas in one failure domain;
- challenge replay;
- providers fabricating retrieval traffic to themselves;
- short-lived storage around predictable challenge times;
- withholding content to manufacture profitable repair events;
- duplicate payment for the same service evidence;
- recovery races and settlement ordering;
- malicious/corrupt chunk injection.

No production reward formula should be frozen until these are modeled and tested.

## Observability

Nodes and tooling should eventually expose at least:

```text
agreement_id
object_root
health_state
observed_provider_count
target_provider_count
minimum_provider_count
last_successful_challenge
last_successful_retrieval
active_recovery_count
agreement_expiry
```

These fields are operationally useful even before health drives protocol settlement.

## Required vectors before lock

1. HEALTHY -> DEGRADED provider-loss case;
2. DEGRADED -> AT_RISK threshold case;
3. AT_RISK -> RECOVERING -> HEALTHY successful repair;
4. failed repair remaining AT_RISK;
5. reconstruction threshold lost -> UNAVAILABLE;
6. renewal preventing EXPIRED;
7. agreement expiry -> EXPIRED;
8. invalid/corrupt recovery chunk rejection;
9. duplicate provider identity/evidence rejection where independence rules apply;
10. deterministic observation-window boundary fixture.

## Consensus boundary

Storage health does not select the canonical chain. Even if a future version uses deterministic health in storage settlement, CPU PoW remains the chain-consensus authority.
