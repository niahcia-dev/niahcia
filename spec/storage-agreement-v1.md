# StorageAgreementV1

Status: **CANDIDATE / pre-alpha durability contract**

## Purpose

`StorageAgreementV1` expresses that a requester wants a content-addressed object or object set kept retrievable for a defined period under an explicit durability target and payment budget.

It answers the higher-level questions that storage challenges alone do not answer:

- what must remain available;
- for how long;
- at what durability/replication target;
- under which storage profile;
- who funds the obligation;
- what evidence counts as service;
- what happens when providers disappear.

A StorageAgreement is an economic/service contract. It is **not** chain-consensus authority and does not give storage providers fork-choice or finality power.

## Optional-service boundary

A StorageAgreement is never required merely to submit or execute an AI Job.

The minimum NIAHCIA compute path uses wallet/client-local private state and direct encrypted job/result transport. This specification applies only when a user, Agent, application, or protocol component explicitly requests remote durability, replication, retrieval guarantees, or another storage service.

Loss or complete absence of storage providers MUST NOT prevent base-chain validation, native value transfer, or ordinary direct wallet-to-worker inference where the required model/input is otherwise available.

## Candidate canonical fields

```text
StorageAgreementV1
- schema_version
- agreement_id
- requester_id
- object_root
- object_size
- storage_profile_id
- start_epoch
- end_epoch
- target_provider_count
- minimum_provider_count
- challenge_policy_id
- retrieval_policy_id
- payment_plan_id
- maximum_budget
- privacy_policy_root
- renewal_policy_id
- recovery_policy_id
- created_block
```

All identifiers and roots refer to canonical versioned objects under the applicable serialization/domain specifications.

## Object identity

`object_root` identifies the exact content-addressed object, manifest, or object set covered by the agreement. Providers MUST NOT substitute different bytes because a filename, display name, model name, or agent label matches.

`object_size` is part of the declared service obligation and must be consistent with the referenced canonical manifest/profile.

## Storage profiles

`storage_profile_id` identifies the storage strategy and reconstruction rules.

Initial implementation SHOULD begin with straightforward chunked replication because it is easier to validate and recover during pre-alpha/devnet.

Future profiles may define erasure coding or other durability schemes without changing the meaning of existing profile versions.

A future erasure-coded profile may specify values such as data shards, parity shards, reconstruction threshold, chunk sizing, and commitment rules. Those semantics are deliberately not locked by this document.

## Provider targets

`target_provider_count` is the desired number of independent eligible providers for the covered object/profile.

`minimum_provider_count` is the threshold below which the agreement enters an at-risk/recovery condition.

Provider independence must eventually be defined against Sybil and common-failure-domain risks. Counting multiple identities controlled by one operator or one failure domain as fully independent is unsafe; exact independence scoring is not locked here.

## Duration

`start_epoch` and `end_epoch` define the requested service period under the applicable service-epoch specification.

Expiration ends the provider obligation unless a valid renewal extends/replaces the agreement. Expiration MUST NOT erase the underlying content from providers by consensus rule; it ends the economic obligation.

## Payment

`payment_plan_id` defines how eligible storage/service evidence is compensated.

`maximum_budget` caps the agreement's authorized storage/service spending in canonical native units.

Storage payment must correspond to defined evidence. Merely claiming capacity or registering a large disk is insufficient.

The agreement itself does not decide whether compensation comes from requester escrow, direct periodic settlement, protocol subsidy, or another versioned mechanism; that belongs to PaymentPlan/economic policy.

## Service dimensions

Storage service should be capable of distinguishing at least:

1. **durability** — possession of the committed content over time;
2. **availability** — ability to answer protocol challenges/retrieval requests;
3. **retrieval** — ability to deliver the requested committed bytes/chunks;
4. **service throughput** — useful delivery under explicitly verifiable accounting where supported;
5. **longevity** — continued valid service across epochs.

No metric grants consensus authority.

Metrics must be resistant to self-dealing/replay before they are made compensable. In particular, raw provider-reported bandwidth is not trusted evidence.

## Privacy

`privacy_policy_root` identifies confidentiality/access requirements.

A storage provider may store ciphertext and prove possession/availability without learning plaintext. A provider's ability to store or serve encrypted content does not grant decryption authority.

Private agent memory, job inputs/results, or other protected objects must not be made public merely to satisfy proof-of-storage requirements.

Concrete encryption/key-distribution schemes are specified separately.

## Renewal

`renewal_policy_id` defines whether/how an agreement may be renewed.

Candidate future modes include manual renewal, requester-authorized automatic renewal within a budget, and agent-policy-controlled renewal.

An autonomous agent may eventually fund renewal of its own model/memory/storage obligations, but a host must not gain treasury authority merely by providing storage.

## Recovery

`recovery_policy_id` identifies how the network should restore durability when providers disappear.

The target behavior is permissionless repair: if enough valid content survives, an eligible independent provider should be able to retrieve, verify, and begin serving missing replicas/shards without requiring the original publisher to come back online.

Recovery must preserve the exact `object_root`; it cannot substitute a semantically similar object.

## Health states

Storage health is observational/service state, not chain fork-choice state.

The initial candidate lifecycle is:

```text
HEALTHY
  -> DEGRADED
  -> AT_RISK
  -> RECOVERING
  -> HEALTHY

AT_RISK
  -> UNAVAILABLE
```

Suggested meanings:

- `HEALTHY`: observed eligible service meets/exceeds target provider/reconstruction policy.
- `DEGRADED`: below target but still safely above the minimum/reconstruction threshold.
- `AT_RISK`: at or below the configured minimum safety threshold; repair should be prioritized.
- `RECOVERING`: replacement provider(s) are reconstructing/verifying service.
- `UNAVAILABLE`: current observations cannot establish sufficient retrievable data to reconstruct the object.

Exact observation windows, transition hysteresis, and authoritative accounting are not locked here.

## Self-healing objective

The protocol should make repair economically possible without a central storage coordinator:

```text
provider loss detected
  -> agreement becomes degraded/at-risk
  -> eligible provider discovers repair opportunity
  -> provider retrieves surviving canonical chunks
  -> verifies commitments
  -> reconstructs exact object/profile
  -> registers/commits service
  -> passes challenges/retrieval checks
  -> health returns toward target
```

Selection/reward rules must avoid creating a privileged permanent repair scheduler.

## Relationship to existing storage proofs

`StorageAgreementV1` sits above existing storage primitives such as commitments, Merkle structures, challenges, responses, proof-of-service, and service epoch reports.

Those primitives answer whether a provider can prove a particular service fact. The agreement answers why that content should remain available and under what economic/durability objective.

## Security invariants

1. Storage/service evidence never becomes PoW chain authority.
2. Providers cannot redefine `object_root` during recovery.
3. Payment requires evidence defined by the applicable policies.
4. Provider identity count alone is not sufficient proof of independence.
5. Private content may remain encrypted; storage does not imply plaintext access.
6. Expired agreements do not authorize arbitrary deletion of protocol-critical data required by another active obligation.
7. A storage host does not gain ownership/governance/capability rights over an Agent because it stores that Agent's state.
8. Recovery must be reproducible from surviving valid committed content.

## Required vectors before lock

1. canonical agreement serialization and ID;
2. exact object-root binding;
3. duration boundary cases;
4. budget encoding/overflow cases;
5. healthy/degraded/at-risk observational fixtures;
6. provider-loss and replacement/recovery fixture;
7. encrypted-object storage fixture that proves possession without plaintext disclosure;
8. expired vs renewed agreement fixture;
9. mismatched storage-profile rejection;
10. cross-implementation agreement verification.

## Not locked in this candidate

- exact storage pricing;
- production provider independence/Sybil scoring;
- automatic renewal rules;
- concrete privacy/encryption scheme;
- erasure-coding profile;
- repair-provider selection;
- reward allocation;
- geographic placement rules;
- exact health observation windows/hysteresis;
- consensus inclusion/commitment mechanism.
