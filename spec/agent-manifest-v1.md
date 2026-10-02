# AgentManifestV1

Status: **CANDIDATE / pre-alpha portability contract**

## Purpose

`AgentManifestV1` defines the portable, integrity-verifiable description needed to reconstruct an agent's authorized execution environment on an independent compatible NIAHCIA host.

The manifest does **not** make an execution host the owner of the agent. It binds an Agent/AgentVersion to the roots and policies needed to locate and validate its execution dependencies.

The design objective is simple:

> an agent's identity, authority, economic relationships, lineage, and authorized portable state must not belong to the machine currently executing it.

## Relationship to Agent and AgentVersion

`Agent` remains the stable identity and governance/lifecycle record.

`AgentVersion` remains the immutable execution-critical version chain.

`AgentManifestV1` is the portable resolution layer for a specific AgentVersion. It gathers the exact commitments needed by an independent host without turning mutable discovery metadata into execution authority.

A manifest MUST identify an exact AgentVersion. Following `Agent.current_version` implicitly is not sufficient for deterministic execution.

## Candidate canonical fields

```text
AgentManifestV1
- schema_version
- agent_id
- agent_version
- agent_version_hash
- lineage_root
- model_policy_root
- execution_policy_root
- capability_root
- memory_descriptor_root
- payment_policy_root
- privacy_policy_root
- succession_policy_root
- storage_manifest_root
- created_block
```

All roots are cryptographic commitments to canonical versioned objects or manifests defined by their applicable specifications. An absent optional policy uses the explicit canonical null/empty representation defined by the serialization specification; implementations MUST NOT invent an implicit default that changes execution authority.

## Portability invariant

Given the same valid `AgentManifestV1` and access to all authorized referenced content, two conforming hosts MUST be able to resolve the same set of execution-authorizing commitments even if their local storage layout, hardware, runtime process model, or operator differs.

This does not require probabilistic model output to be byte-identical unless the selected ExecutionProfile/VerificationPolicy requires deterministic reproduction.

## Lineage

`agent_version_hash` commits to the exact immutable AgentVersion.

`lineage_root` commits to the authorized ancestry/version lineage needed to verify that this version descends from the expected Agent identity and governance history.

Hosts MUST NOT treat an unlinked replacement definition as the same agent merely because it uses the same display name, model, or metadata.

## Models and execution

`model_policy_root` identifies the versioned model-selection/routing policy authorized for this AgentVersion.

`execution_policy_root` identifies allowed ExecutionProfiles and execution constraints.

The manifest MUST NOT require one permanent runtime, model vendor, accelerator vendor, or physical host. Compatibility is determined by the referenced versioned policies/profiles.

## Capabilities

`capability_root` commits to the authority the agent may exercise or delegate.

Capability rules should be able to constrain, where applicable:

- permitted tools/services;
- permitted contracts or destinations;
- spending/budget limits;
- expiration/validity windows;
- delegation rights and maximum delegation depth;
- model/profile access;
- storage/memory access;
- revocation conditions;
- rate or usage limits.

A compute worker executing the agent MUST NOT automatically inherit broader controller/treasury authority merely because it hosts inference.

## Portable memory

`memory_descriptor_root` commits to authorized memory descriptors, not necessarily to all memory bytes directly.

Memory may remain content-addressed, encrypted, access-controlled, sharded, or externally stored under the applicable MemoryDescriptor/storage specifications. Portability requires that the manifest identify the authorized memory state and its integrity/access rules without assuming one database or storage provider.

Moving execution hosts MUST NOT silently substitute different memory state under the same manifest commitment.

## Payment policy

`payment_policy_root` identifies the versioned rules under which the agent may fund jobs, purchase services, receive revenue, or interact with its treasury/budget authority.

The execution host is not automatically entitled to control the agent treasury. Payments to compute/storage/verifier participants occur through the applicable Job/PaymentPlan/protocol settlement rules.

## Privacy policy

`privacy_policy_root` is reserved for versioned job/input/output/memory confidentiality requirements.

Potential policy classes may include public, encrypted-input, result-private, memory-private, or attested/confidential execution requirements, but **no concrete privacy class is locked by this specification yet**.

A host that cannot satisfy the referenced privacy policy is ineligible to execute that manifest's protected work.

## Succession policy

`succession_policy_root` commits to the rules that determine what may happen when the current controller/creator becomes unavailable or when a configured succession condition is met.

Candidate policies may eventually support:

- no succession / immutable control;
- designated successor identity;
- contract-governed succession;
- threshold-governed succession;
- time-delayed succession;
- dormancy/freeze rather than transfer.

This specification does not yet lock the state machine or activation conditions. Succession MUST NOT be inferred from operator availability and MUST NOT allow an execution host to seize agent control.

## Storage manifest

`storage_manifest_root` commits to the canonical content-addressed resources required to reconstruct the portable authorized environment, subject to capability/privacy restrictions.

It may resolve model manifests, tool manifests, encrypted memory objects, definitions, and other referenced content. Availability is provided by storage/service participants; the manifest commitment itself does not grant those participants governance authority.

## Host migration

A conforming migration should conceptually perform:

```text
resolve Agent
  -> resolve exact AgentVersion
  -> verify AgentManifestV1 commitment
  -> verify lineage
  -> resolve authorized policies/roots
  -> retrieve permitted canonical content
  -> verify hashes/manifests
  -> prove host eligibility for ExecutionProfile/privacy requirements
  -> execute without acquiring agent ownership
```

The previous host need not authorize the new host unless an applicable capability/policy explicitly requires such authorization. Portability must not create permanent host lock-in.

## Security boundaries

1. A valid manifest authenticates commitments; it does not prove referenced services are currently available.
2. A host MUST verify referenced hashes/roots before use.
3. Host possession of decrypted transient state does not grant protocol ownership.
4. Controller/governance changes remain governed by Agent rules, not by the manifest host.
5. Capability and payment authority must use least privilege.
6. Missing referenced content causes inability to execute, not permission to substitute arbitrary content.
7. Privacy-sensitive material must not be made public merely to satisfy portability.

## Consensus boundary

The complete expanded manifest does not need to live directly in every block. Consensus/on-chain state may commit to identities, versions, roots, authorizations, and settlement state while large model/memory/tool resources remain content-addressed through the service/storage layer.

The exact on-chain commitment and transaction/state-transition mechanism is **not locked here**.

## Required vectors before lock

At minimum:

1. canonical AgentManifestV1 serialization;
2. manifest identifier/hash derivation;
3. AgentVersion binding;
4. lineage-root positive and negative cases;
5. capability-root substitution rejection;
6. memory-root substitution rejection;
7. model/execution-policy substitution rejection;
8. explicit empty optional-policy representation;
9. unknown schema/version rejection behavior;
10. cross-implementation manifest resolution fixture.

## Not locked in V1 candidate

- succession state-machine semantics;
- concrete privacy policy classes;
- governance transitions;
- host-selection/scheduling algorithm;
- reputation/ranking;
- storage replication factor;
- one mandatory runtime/model;
- consensus storage location for the manifest commitment.

These require separate specifications and implementation experience before interoperability freeze.
