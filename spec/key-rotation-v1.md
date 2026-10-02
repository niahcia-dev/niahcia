# KeyRotationV1

Status: **CANDIDATE / pre-alpha authority transition model**

## Purpose

`KeyRotationV1` defines an auditable transition from one valid `KeyAuthorityV1` epoch to the next while preserving the subject's durable NIAHCIA identity.

Key rotation exists so a long-lived Agent/account/operator is not permanently bound to one private key.

## Candidate transition

```text
subject_id
  authority epoch N
       |
       | authorized transition
       v
  authority epoch N+1
```

The transition MUST bind the previous authority state and the exact next authority state.

## Candidate canonical fields

```text
KeyRotationV1
- schema_version
- subject_id
- previous_authority_hash
- next_authority_hash
- transition_reason
- authorization_mode
- authorization_evidence_root
- effective_block
- created_block
```

## Authorization modes

Candidate modes:

- `NORMAL` — authorized by currently valid control authority;
- `RECOVERY` — authorized under the configured RecoveryPolicy;
- `SUCCESSION` — authorized by a separately valid succession/governance transition where permitted.

Exact numeric enum assignments are not locked until NCE/1 allocation.

## Rotation invariants

1. `subject_id` does not change merely because keys rotate.
2. `previous_authority_hash` must match the currently valid authority state at the transition point.
3. `next_authority_hash` must commit to the complete next authority state.
4. stale/forked rotations from an old authority epoch must be rejected under deterministic state-transition rules.
5. historical signatures remain evaluated against the authority epoch valid when they were created.
6. rotation does not retroactively revoke already finalized valid protocol actions.
7. rotation of signing authority does not require re-encrypting all bulk data.
8. changing encrypted-object bytes/content roots requires normal versioned storage/object transitions.

## Emergency compromise response

A subject that still possesses valid recovery authority may rotate away from a compromised signing key.

Future policy may support revocation of outstanding capabilities/session keys as part of or immediately following a rotation. Exact revocation mechanics belong to the capability/authority specifications.

A compromised key can expose actions/data that occurred before detection; rotation limits future authority but cannot erase past disclosure.

## Concurrency and races

Two competing rotations from the same authority epoch must not both become valid canonical successors.

The chain state transition must deterministically accept at most one next authority state according to transaction ordering/canonical state rules. A losing competing rotation becomes stale and cannot be replayed against the new epoch.

## Required vectors before lock

1. normal N -> N+1 rotation;
2. recovery-authorized rotation;
3. wrong previous hash rejection;
4. stale epoch replay rejection;
5. competing rotations deterministic outcome;
6. unchanged subject identity across rotation;
7. historical signature validation across epochs;
8. revoked/expired authorization rejection;
9. canonical transition hash/ID;
10. cross-implementation transition fixture.

## Not locked

- transaction encoding/state-transition location;
- rotation fees;
- minimum/maximum rotation frequency;
- recovery delay;
- threshold-signature mechanism;
- hardware signer integration.
