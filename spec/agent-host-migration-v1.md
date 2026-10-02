# AgentHostMigrationV1

Status: **CANDIDATE / pre-alpha portable execution security model**

## Purpose

`AgentHostMigrationV1` defines the security boundary for moving an Agent's execution from one host/worker to another without transferring ownership, long-term signing keys, unrestricted treasury authority, or implicit access to all historical private memory.

The central rule is:

> execution is portable; authority is not handed to the execution host.

A host is a replaceable compute resource. The durable Agent identity, KeyAuthority, AgentManifest, capabilities, storage commitments, and economic state remain protocol-resolvable independently of the host.

## Conceptual flow

```text
Agent durable identity
      |
      +-- AgentManifest
      +-- KeyAuthority epoch
      +-- MemoryDescriptor(s)
      +-- Storage references
      +-- capabilities/policies
      |
 old host X
      |
      | migration / rescheduling
      v
 new host
      |
      +-- resolves exact manifest/version
      +-- proves/establishes execution context
      +-- receives bounded session capability
      +-- obtains only authorized memory material
      +-- executes job/session
      +-- emits commitments/receipts
      +-- session expires/revokes
```

## Migration is not key transfer

The new host MUST NOT require the Agent's long-term signing private key merely to execute the Agent.

The new host MUST NOT automatically receive:

- master signing authority;
- unrestricted treasury authority;
- recovery authority;
- succession authority;
- historical memory decryption keys outside the authorized scope;
- permission to alter AgentVersion/AgentManifest/KeyAuthority state.

## Candidate migration descriptor

```text
AgentHostMigrationV1
- schema_version
- migration_id
- agent_id
- agent_version_id
- agent_manifest_id
- key_authority_epoch
- source_execution_ref
- destination_worker_id
- execution_profile_id
- memory_scope_root
- capability_root
- session_authority_id
- migration_nonce
- valid_from
- expires_at
- created_block
```

Exact NCE/1 field IDs and inclusion semantics are not locked.

## Memory unlock model

A destination host should receive only the memory/data capability required for the authorized execution scope.

A safe conceptual pattern is envelope encryption:

```text
stored memory ciphertext
       |
       | encrypted with DEK
       v
  data-encryption key
       |
       | released/wrapped only for authorized session scope
       v
 destination execution environment
```

The long-term signing key does not decrypt bulk memory directly.

The mechanism used to deliver/unwrap a DEK for a destination host is a separate cryptographic profile and is not locked by this candidate. Possible future profiles include host/session public-key wrapping, threshold authorization, hardware-backed/attested release, or privacy-preserving execution mechanisms.

## Session authority

Migration should establish a short-lived, least-privileged session authority bound to the exact Agent, AgentVersion/Manifest, destination worker, execution profile, memory scope, capabilities, validity window, and migration/job nonce.

A session must not remain reusable indefinitely after the host stops executing the Agent.

## Source host failure

Migration MUST be possible without cooperation from the old host when the protocol/storage state needed for continuation survives elsewhere.

The old host must not become a portability veto.

If the old host held unique uncommitted state, that state may be lost. Implementations should therefore checkpoint durable Agent state according to the applicable memory/storage policy rather than treating local host state as authoritative persistence.

## Checkpoint boundary

Portable execution needs an explicit durable checkpoint boundary.

A checkpoint should identify the exact committed memory/state version from which another compatible host may resume. Work performed after the last durable checkpoint may need to be retried if a host disappears.

Exactly-once external side effects cannot be assumed merely because Agent memory resumes from a checkpoint. Jobs/tools/payments need idempotency identifiers, receipts, or other protocol-specific replay protection.

## Destination verification

Before granting sensitive session capability, the authorizing component should verify at least the destination worker identity and requested execution context against applicable policy.

Optional future policies may additionally require hardware/runtime attestations. NIAHCIA must not make one TEE vendor a universal protocol authority.

## Host compromise

A malicious host may observe any plaintext intentionally made available to its execution environment unless a stronger private-compute profile is used.

Therefore migration alone does not provide confidential computing. Prototype/pre-alpha deployments must not imply that ordinary GPU hosts cannot read plaintext prompts/memory delivered to them.

Damage should be bounded by scoped memory access, bounded capabilities, session expiry/revocation, spending limits, signer isolation, and durable audit/receipt commitments.

## Rollback protection

A destination host MUST NOT be allowed to resurrect stale authority or stale mutable Agent state as current merely by presenting an old checkpoint.

Migration authorization should bind current `key_authority_epoch` and the selected durable memory/checkpoint version. Protocol state determines which versions are current/acceptable.

## Duplicate execution

Network failures can cause both old and new hosts to execute concurrently.

The protocol must assume duplicate execution is possible. Safety must come from job/session IDs, capability scope, spending limits, state-version checks, idempotent settlement where applicable, and canonical state transitions—not from assuming only one process exists.

## Security invariants

1. Execution host is replaceable and is not Agent identity.
2. Migration does not transfer the long-term signing private key.
3. Destination receives only explicitly scoped memory/capability authority.
4. Old host cooperation is not required when durable state survives elsewhere.
5. Stale authority/checkpoints cannot silently become current.
6. Duplicate execution is expected and must be bounded safely.
7. Signer isolation remains in force across migration.
8. Storage providers do not become decryption authorities by serving migrated state.
9. Ordinary hosts may see authorized plaintext unless a stronger private-compute profile is used.
10. External side effects require replay/idempotency protection independent of memory checkpointing.

## Required vectors before lock

1. migration descriptor canonical serialization/ID;
2. same Agent identity across different workers;
3. destination-bound session capability;
4. expired session rejection;
5. wrong worker/session binding rejection;
6. stale KeyAuthority epoch rejection;
7. stale memory/checkpoint rejection;
8. source-host-dead recovery path;
9. duplicate-execution replay protection fixture;
10. unauthorized memory-scope request rejection;
11. cross-implementation migration fixture.

## Not locked

- DEK transport/wrapping algorithm;
- TEE/attestation requirements;
- migration scheduling;
- checkpoint frequency;
- private inference mechanism;
- live-memory transfer;
- session key algorithm;
- on-chain migration inclusion/settlement.
