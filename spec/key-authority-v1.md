# KeyAuthorityV1

Status: **CANDIDATE / pre-alpha cryptographic authority model**

## Purpose

`KeyAuthorityV1` defines the separation between durable NIAHCIA identity, signing/control authority, encryption authority, and limited execution/session authority.

The central rule is:

> every protocol actor and economically controlled resource ultimately resolves to NIAHCIA cryptographic authority, but one private key should not perform every cryptographic role.

A NIAHCIA address identifies the durable protocol entity. Versioned authority records determine which keys may act for that entity and for which purpose.

## Authority hierarchy

Conceptually:

```text
NIAHCIA address / durable entity
        |
        +-- signing/control authority
        |      +-- transactions
        |      +-- capabilities
        |      +-- key rotation
        |      +-- policy changes
        |
        +-- encryption authority
        |      +-- wraps/authorizes data keys
        |      +-- memory epochs
        |      +-- private objects
        |
        +-- delegated/session authority
               +-- bounded job execution
               +-- bounded spending
               +-- bounded tool/storage access
               +-- expiry/revocation
```

## Identity is not one eternal key

A durable NIAHCIA identity MUST NOT require one private key to remain valid forever.

Keys may rotate while the entity/address remains stable, provided the transition is authorized by the currently valid authority or by an explicitly configured recovery/succession policy.

Historical signatures remain attributable to the key epoch under which they were valid.

## Candidate canonical fields

```text
KeyAuthorityV1
- schema_version
- subject_id
- authority_epoch
- signing_key_descriptor
- encryption_key_descriptor
- capability_root
- recovery_policy_id
- previous_authority_hash
- valid_from
- expires_at
- revocation_root
- created_block
```

Exact key algorithms/descriptor schemas and field IDs are not locked here.

## Subject

`subject_id` resolves to the durable NIAHCIA identity whose authority is being described. Depending on the applicable object specification this may be an account/address-backed Agent, Operator, service identity, or another protocol entity.

A display name, host name, model name, or server identity is never sufficient authority.

## Signing/control authority

The signing authority authorizes protocol actions according to capabilities and policy. It may authorize transactions, capability delegation, key rotation, storage/compute purchases, and other explicitly permitted state transitions.

A signing key MUST NOT automatically be exposed to an AI runtime or remote execution host.

Implementations SHOULD isolate signing behind a signer/key service that evaluates the requested operation and applicable capability/policy before producing a signature.

## Encryption authority

The encryption authority controls access to protected data keys. Large data and agent memory SHOULD be encrypted with randomly generated symmetric data-encryption keys rather than directly with the NIAHCIA signing private key.

Conceptually:

```text
protected object
   -> random data-encryption key (DEK)
   -> authenticated encryption
   -> ciphertext

DEK
   -> wrapped/authorized under versioned agent encryption authority
```

This separation permits signing-key rotation without re-encrypting every stored byte and limits the blast radius of a single key compromise.

## Memory key epochs

Long-lived agent memory SHOULD support encryption epochs.

A new epoch may use a new data/wrapping key while older ciphertext remains decryptable according to the agent's authorized retention/recovery policy.

Rotation MUST NOT silently change the committed plaintext/ciphertext identity used by storage manifests. Re-encryption that changes content commitments is a new versioned storage object/manifest transition.

## Agent identity

An autonomous Agent SHOULD be capable of having its own durable NIAHCIA cryptographic identity distinct from its creator/controller.

The creator may initially authorize or govern the Agent according to the Agent governance policy, but the creator's disappearance need not destroy the Agent if the Agent's own authority, keys, policies, storage, and economic resources remain valid.

Execution hosts MUST NOT receive ownership of the Agent merely because they temporarily execute it.

## Delegated/session authority

Compute workers and tool runtimes should receive the minimum temporary authority necessary for a job.

Delegations may constrain:

- permitted operation/tool;
- destination/contract/service;
- maximum spend;
- storage/memory scope;
- model/execution profile;
- call/delegation depth;
- rate/usage limits;
- validity window;
- job/session identifier.

Possession of a session capability MUST NOT imply possession of the subject's master signing or encryption authority.

## Signer isolation

AI inference runtimes SHOULD NOT receive raw long-term private keys.

A signer component may accept a structured request such as a payment or capability operation, validate it against current policy/capability/budget, and sign only the approved canonical payload.

Prompt/model output is untrusted input to the signer. Natural-language intent alone MUST NOT bypass canonical authorization checks.

## Key compromise

Compromise of one role should be contained where possible:

- compromised session key -> revoke/expire bounded session;
- compromised signing key -> rotate through valid current/recovery authority;
- compromised encryption authority -> rotate future encryption epochs and treat accessible historical protected data according to exposure policy;
- compromised execution host -> revoke its capabilities without changing Agent identity.

Rotation cannot make already disclosed plaintext secret again.

## Lost keys

If all valid signing/recovery authority is lost, control may be permanently lost. NIAHCIA MUST NOT contain a protocol-wide developer/master recovery key.

If all authorized decryption capability for protected memory is lost, durable ciphertext may remain available but unusable. Storage durability and key continuity are separate properties.

Recovery is therefore opt-in and policy-driven rather than silently provided by storage nodes or protocol developers.

## Relationship to recovery/succession

`recovery_policy_id` references a versioned policy describing who/what may authorize an authority transition when normal control is unavailable.

Recovery must not grant storage nodes, compute hosts, miners, developers, or website operators implicit control.

Succession may change governance/controller relationships; key recovery/rotation changes cryptographic authority. These concepts may interact but should not be conflated.

## Security invariants

1. Durable identity is distinct from one physical host and one eternal key.
2. Long-term signing keys are not required inside AI runtimes.
3. Signing authority and encryption authority are separable.
4. Bulk data uses data-encryption keys rather than direct wallet-key encryption.
5. Delegated/session authority is least-privileged and bounded.
6. Storage possession does not imply decryption or governance authority.
7. Compute execution does not imply treasury or master-key authority.
8. Key rotation preserves identity and historical epoch attribution.
9. Recovery is explicit and opt-in; there is no NIAHCIA master recovery key.
10. Natural-language model output is never sufficient authorization for privileged signing.

## Required vectors before lock

1. canonical KeyAuthorityV1 serialization and ID;
2. initial authority epoch;
3. authorized signing-key rotation;
4. unauthorized rotation rejection;
5. encryption-key epoch transition;
6. bounded session delegation;
7. expired/revoked session rejection;
8. previous-authority hash-chain verification;
9. recovery-authorized rotation;
10. unknown algorithm/version rejection;
11. cross-implementation authority verification fixture.

## Not locked in this candidate

- signing/encryption algorithms beyond existing applicable protocol requirements;
- exact key descriptor encoding;
- hardware-wallet/HSM interface;
- threshold cryptography scheme;
- recovery policy state machine;
- succession state machine;
- key escrow (no default escrow is intended);
- exact on-chain authority storage/commitment mechanism.
